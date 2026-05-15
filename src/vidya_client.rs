use std::sync::Arc;

use rmcp::ServiceExt;
use rmcp::model::{CallToolRequestParams, CallToolResult, RawContent};
use rmcp::service::{Peer, RunningService};
use rmcp::transport::streamable_http_client::{
    StreamableHttpClientTransport, StreamableHttpClientTransportConfig,
};
use serde::Deserialize;

type ClientService = RunningService<rmcp::RoleClient, ()>;

pub struct VidyaClient {
    _service: ClientService,
    peer: Peer<rmcp::RoleClient>,
}

#[derive(Debug, Clone)]
pub struct RawClaim {
    pub template_slug: String,
    pub params: serde_json::Value,
    pub statement: String,
}

impl VidyaClient {
    pub async fn connect(url: &str, auth_token: Option<&str>) -> anyhow::Result<Self> {
        let mut config = StreamableHttpClientTransportConfig::default();
        config.uri = Arc::from(url);
        config.auth_header = auth_token.map(|t| t.to_string());
        let transport = StreamableHttpClientTransport::from_config(config);
        let service = ().serve(transport).await?;
        let peer = service.peer().clone();
        Ok(Self {
            _service: service,
            peer,
        })
    }

    pub async fn fetch_claims(
        &self,
        domain: &str,
        claim_template: &str,
    ) -> anyhow::Result<Vec<RawClaim>> {
        let args = serde_json::json!({
            "domain": domain,
            "claim_template": claim_template,
            "include_provenance": false,
        });
        let arguments = args.as_object().unwrap().clone();

        let result: CallToolResult = self
            .peer
            .call_tool(
                CallToolRequestParams::new("vidya_query").with_arguments(arguments),
            )
            .await?;

        parse_claims_from_response(&result)
    }
}

#[derive(Debug, Deserialize)]
struct QueryOutput {
    claims: Option<Vec<ClaimEntry>>,
}

#[derive(Debug, Deserialize)]
struct ClaimEntry {
    claim: ClaimInner,
    template_slug: String,
}

#[derive(Debug, Deserialize)]
struct ClaimInner {
    params: serde_json::Value,
    statement: String,
}

fn parse_claims_from_response(result: &CallToolResult) -> anyhow::Result<Vec<RawClaim>> {
    let text = result
        .content
        .iter()
        .find_map(|c| match &c.raw {
            RawContent::Text(t) => Some(&t.text),
            _ => None,
        })
        .ok_or_else(|| anyhow::anyhow!("no text content in vidya_query response"))?;

    let output: QueryOutput = serde_json::from_str(text)?;
    let claims = output.claims.unwrap_or_default();

    Ok(claims
        .into_iter()
        .map(|c| RawClaim {
            template_slug: c.template_slug,
            params: c.claim.params,
            statement: c.claim.statement,
        })
        .collect())
}
