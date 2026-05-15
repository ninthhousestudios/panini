use std::sync::Arc;

use rmcp::ServiceExt;
use rmcp::model::{CallToolRequestParams, CallToolResult, RawContent};
use rmcp::service::Peer;
use rmcp::transport::streamable_http_client::{
    StreamableHttpClientTransport, StreamableHttpClientTransportConfig,
};
use serde::Deserialize;

pub struct VidyaClient {
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
        config.auth_header = auth_token.map(|t| format!("Bearer {t}"));
        let transport = StreamableHttpClientTransport::from_config(config);
        let client = ().serve(transport).await?;
        Ok(Self {
            peer: client.peer().clone(),
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
    claims: Vec<ClaimEntry>,
}

#[derive(Debug, Deserialize)]
struct ClaimEntry {
    template_slug: String,
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

    Ok(output
        .claims
        .into_iter()
        .map(|c| RawClaim {
            template_slug: c.template_slug,
            params: c.params,
            statement: c.statement,
        })
        .collect())
}
