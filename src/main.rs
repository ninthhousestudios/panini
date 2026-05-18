use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use axum::http::header;
use axum::response::Html;
use axum::routing::{any_service, get, post};
use clap::Parser;
use rmcp::ServiceExt;
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager,
    tower::{StreamableHttpServerConfig, StreamableHttpService},
};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tower_http::validate_request::ValidateRequestHeaderLayer;
use tracing_subscriber::EnvFilter;

use panini::api;
use panini::config::Config;
use panini::mcp::PaniniServer;
use panini::rule_cache::RuleCache;
use panini::vidya_client::VidyaClient;

#[derive(Parser)]
#[command(name = "panini", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(clap::Subcommand)]
enum Command {
    Gui,
    Serve {
        #[arg(long)]
        stdio: bool,
        #[arg(long)]
        auth_token_file: Option<PathBuf>,
        #[arg(long)]
        http_port: Option<u16>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let cfg = Config::from_env();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(&cfg.log_level)),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    match cli.command.unwrap_or(Command::Gui) {
        Command::Gui => {
            eprintln!("Loading rules from vidya…");
            let cache = build_cache(&cfg).await?;
            eprintln!("Launching GUI…");
            panini::gui::run(cache).map_err(|e| anyhow::anyhow!("{e}"))?;
            Ok(())
        }
        Command::Serve {
            stdio,
            auth_token_file,
            http_port,
        } => {
            if stdio {
                let cache = build_cache(&cfg).await?;
                serve_stdio(cache).await
            } else {
                let port = http_port.unwrap_or(cfg.http_port);
                let addr = format!("{}:{}", cfg.http_host, port);
                let listener = match TcpListener::bind(&addr).await {
                    Ok(l) => l,
                    Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
                        eprintln!("panini already running on {addr}");
                        std::process::exit(0);
                    }
                    Err(e) => return Err(e.into()),
                };
                let cache = build_cache(&cfg).await?;
                serve_http(auth_token_file, cache, listener).await
            }
        }
    }
}

async fn build_cache(cfg: &Config) -> anyhow::Result<Arc<RuleCache>> {
    tracing::info!(url = %cfg.vidya_url, "connecting to vidya");
    let vidya = VidyaClient::connect(&cfg.vidya_url, cfg.vidya_auth_token.as_deref())
        .await
        .context("failed to connect to vidya — is it running?")?;

    let mut cache = RuleCache::new();
    let sandhi_claims = vidya
        .fetch_claims("vyakarana", "sandhi_rule")
        .await
        .context("failed to fetch sandhi rules from vidya")?;
    let count = sandhi_claims.len();
    if count == 0 {
        anyhow::bail!("zero sandhi rules loaded from vidya — check seed data");
    }
    cache.load_template("sandhi_rule".into(), sandhi_claims);

    let parse_errors =
        panini::engine::sandhi::validate_rules(cache.get_rules("sandhi_rule"));
    if !parse_errors.is_empty() {
        for err in &parse_errors {
            tracing::error!(%err, "unparseable sandhi rule");
        }
        anyhow::bail!(
            "{} of {} sandhi rules failed to parse — check vidya seed data",
            parse_errors.len(),
            count
        );
    }
    tracing::info!(count, "cached sandhi rules (all validated)");

    for template in ["sup_suffix", "pratyaya_rule", "anga_rule", "tripadi_rule"] {
        let claims = vidya
            .fetch_claims("vyakarana", template)
            .await
            .context(format!("failed to fetch {template} from vidya"))?;
        if claims.is_empty() {
            anyhow::bail!("zero {template} rules loaded from vidya — declension requires all five template types");
        }
        let count = claims.len();
        cache.load_template(template.into(), claims);
        tracing::info!(template, count, "cached rules");
    }

    Ok(Arc::new(cache))
}

async fn serve_stdio(cache: Arc<RuleCache>) -> anyhow::Result<()> {
    let server = PaniniServer::new(cache);
    let (stdin, stdout) = rmcp::transport::stdio();
    let service = server
        .serve((stdin, stdout))
        .await
        .context("starting MCP service")?;

    tokio::select! {
        res = service.waiting() => { res.context("MCP service error")?; }
        _ = shutdown_signal() => { tracing::info!("shutting down"); }
    }
    Ok(())
}

async fn serve_http(
    auth_token_file: Option<PathBuf>,
    cache: Arc<RuleCache>,
    listener: TcpListener,
) -> anyhow::Result<()> {
    let bearer_token = match auth_token_file {
        Some(path) => {
            let token = std::fs::read_to_string(&path)
                .with_context(|| format!("reading auth token from {}", path.display()))?
                .trim()
                .to_string();
            anyhow::ensure!(!token.is_empty(), "auth token file is empty");
            anyhow::ensure!(
                token.chars().all(|c| !c.is_control()),
                "auth token contains control characters"
            );
            Some(token)
        }
        None => {
            tracing::warn!("running without auth — not for production use");
            None
        }
    };

    let cancel = CancellationToken::new();
    let shttp_config =
        StreamableHttpServerConfig::default().with_cancellation_token(cancel.clone());

    let mut session_manager = LocalSessionManager::default();
    session_manager.session_config.keep_alive = None;
    let session_manager = Arc::new(session_manager);

    let cache_for_mcp = cache.clone();
    let mcp_service = StreamableHttpService::new(
        move || Ok(PaniniServer::new(cache_for_mcp.clone())),
        session_manager,
        shttp_config,
    );

    let normalize_accept = axum::middleware::from_fn(
        |mut req: axum::http::Request<axum::body::Body>,
         next: axum::middleware::Next| async move {
            use axum::http::header::ACCEPT;
            let needs_fix = req
                .headers()
                .get(ACCEPT)
                .and_then(|v| v.to_str().ok())
                .is_none_or(|v| {
                    !v.contains("application/json") || !v.contains("text/event-stream")
                });
            if needs_fix {
                req.headers_mut().insert(
                    ACCEPT,
                    "application/json, text/event-stream".parse().unwrap(),
                );
            }
            next.run(req).await
        },
    );

    let api_routes = axum::Router::new()
        .route("/api/health", get(api::health))
        .route("/api/derive", post(api::derive))
        .route("/api/analyze", post(api::analyze))
        .route("/api/paradigm", post(api::paradigm))
        .route("/api/sutras", get(api::sutras))
        .route("/api/check", get(api::check))
        .with_state(cache);

    let protected = axum::Router::new()
        .route("/mcp", any_service(mcp_service))
        .layer(normalize_accept)
        .merge(api_routes);

    let static_html = include_str!("../static/index.html");
    let ui_routes = axum::Router::new()
        .route(
            "/",
            get(move || async move {
                ([(header::CACHE_CONTROL, "no-cache")], Html(static_html))
            }),
        )
        .route(
            "/health",
            get(|| async { axum::Json(serde_json::json!({"status": "ok"})) }),
        );

    #[allow(deprecated)]
    let app = if let Some(ref token) = bearer_token {
        ui_routes.merge(protected.layer(ValidateRequestHeaderLayer::bearer(token)))
    } else {
        ui_routes.merge(protected)
    };

    let addr = listener.local_addr()?;
    tracing::info!(%addr, "panini HTTP server listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            shutdown_signal().await;
            cancel.cancel();
        })
        .await?;
    Ok(())
}

#[cfg(unix)]
async fn shutdown_signal() {
    use tokio::signal::unix::{SignalKind, signal};
    let mut sigint = signal(SignalKind::interrupt()).expect("SIGINT handler");
    let mut sigterm = signal(SignalKind::terminate()).expect("SIGTERM handler");
    tokio::select! {
        _ = sigint.recv() => {}
        _ = sigterm.recv() => {}
    }
}

#[cfg(not(unix))]
async fn shutdown_signal() {
    tokio::signal::ctrl_c().await.expect("Ctrl+C handler");
}
