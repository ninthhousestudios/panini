use std::sync::Arc;

use anyhow::Context;
use rmcp::ServiceExt;
use tracing_subscriber::EnvFilter;

use panini::config::Config;
use panini::mcp::PaniniServer;
use panini::rule_cache::RuleCache;
use panini::vidya_client::VidyaClient;

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
    tracing::info!(count, "cached sandhi rules");
    cache.load_template("sandhi_rule".into(), sandhi_claims);

    let server = PaniniServer::new(Arc::new(cache));
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
