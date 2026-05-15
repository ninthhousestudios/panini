use std::sync::Arc;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::{ErrorData, ServerHandler, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::engine;
use crate::engine::declension::{DeclensionInput, derive_declension};
use crate::engine::sandhi::{SandhiInput, analyze_sandhi, derive_sandhi};
use crate::error::PaniniError;
use crate::rule_cache::RuleCache;

#[derive(Clone)]
pub struct PaniniServer {
    cache: Arc<RuleCache>,
    tool_router: ToolRouter<Self>,
}

impl PaniniServer {
    pub fn new(cache: Arc<RuleCache>) -> Self {
        Self {
            cache,
            tool_router: Self::tool_router(),
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct HealthArgs {}

#[derive(Debug, Serialize)]
pub struct HealthOutput {
    pub status: &'static str,
    pub version: &'static str,
    pub rule_templates: usize,
    pub total_rules: usize,
    pub sandhi_rules: usize,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeriveArgs {
    pub domain: String,
    pub operation: String,
    pub input: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct DeriveOutput {
    pub domain: String,
    pub operation: String,
    pub input: serde_json::Value,
    pub result: serde_json::Value,
    pub trace: Vec<engine::TraceStep>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AnalyzeArgs {
    pub domain: String,
    pub operation: String,
    pub form: String,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeOutput {
    pub domain: String,
    pub operation: String,
    pub form: String,
    pub candidates: Vec<engine::AnalyzeCandidate>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ParadigmArgs {
    pub domain: String,
    pub stem: String,
    pub stem_type: String,
}

const CASES: [&str; 8] = ["1", "2", "3", "4", "5", "6", "7", "8"];
const NUMBERS: [&str; 3] = ["sg", "du", "pl"];

#[derive(Debug, Serialize)]
pub struct ParadigmCell {
    pub case: String,
    pub number: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<Vec<engine::TraceStep>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ParadigmOutput {
    pub domain: String,
    pub stem: String,
    pub stem_type: String,
    pub cells: Vec<ParadigmCell>,
}

#[tool_router(router = tool_router)]
impl PaniniServer {
    #[tool(description = "Health check. Returns version and rule cache statistics.")]
    pub async fn panini_health(
        &self,
        Parameters(_args): Parameters<HealthArgs>,
    ) -> Result<String, ErrorData> {
        let out = HealthOutput {
            status: "ok",
            version: env!("CARGO_PKG_VERSION"),
            rule_templates: self.cache.template_count(),
            total_rules: self.cache.total_rules(),
            sandhi_rules: self.cache.rule_count("sandhi_rule"),
        };
        serde_json::to_string_pretty(&out).map_err(json_err)
    }

    #[tool(description = "Forward derivation. domain=vyakarana, operation=sandhi|declension. Sandhi input: {first, second}. Declension input: {stem, stem_type, case, number}. Returns result and sūtra-cited trace.")]
    pub async fn panini_derive(
        &self,
        Parameters(args): Parameters<DeriveArgs>,
    ) -> Result<String, ErrorData> {
        if args.domain != "vyakarana" {
            return Err(to_error_data(PaniniError::InvalidArgument {
                tool: "panini_derive".into(),
                argument: "domain".into(),
                constraint: "must be 'vyakarana'".into(),
                received: args.domain,
            }));
        }

        let derive_result = match args.operation.as_str() {
            "sandhi" => {
                let rules = self.cache.get_rules("sandhi_rule");
                if rules.is_empty() {
                    return Err(to_error_data(PaniniError::NoRulesLoaded(
                        "sandhi_rule".into(),
                    )));
                }
                let input: SandhiInput =
                    serde_json::from_value(args.input.clone()).map_err(|e| {
                        to_error_data(PaniniError::InvalidArgument {
                            tool: "panini_derive".into(),
                            argument: "input".into(),
                            constraint: "requires {first, second}".into(),
                            received: e.to_string(),
                        })
                    })?;
                derive_sandhi(rules, input).map_err(to_error_data)?
            }
            "declension" => {
                let input: DeclensionInput =
                    serde_json::from_value(args.input.clone()).map_err(|e| {
                        to_error_data(PaniniError::InvalidArgument {
                            tool: "panini_derive".into(),
                            argument: "input".into(),
                            constraint: "requires {stem, stem_type, case, number}".into(),
                            received: e.to_string(),
                        })
                    })?;
                derive_declension(
                    self.cache.get_rules("sup_suffix"),
                    self.cache.get_rules("pratyaya_rule"),
                    self.cache.get_rules("anga_rule"),
                    self.cache.get_rules("sandhi_rule"),
                    self.cache.get_rules("tripadi_rule"),
                    input,
                )
                .map_err(to_error_data)?
            }
            other => {
                return Err(to_error_data(PaniniError::InvalidArgument {
                    tool: "panini_derive".into(),
                    argument: "operation".into(),
                    constraint: "must be 'sandhi' or 'declension'".into(),
                    received: other.into(),
                }));
            }
        };

        let out = DeriveOutput {
            domain: args.domain,
            operation: args.operation,
            input: args.input,
            result: derive_result.output,
            trace: derive_result.trace,
        };
        serde_json::to_string_pretty(&out).map_err(json_err)
    }

    #[tool(description = "Reverse sandhi analysis. domain=vyakarana, operation=sandhi. Takes a combined form and returns ranked candidate decompositions with sūtra references.")]
    pub async fn panini_analyze(
        &self,
        Parameters(args): Parameters<AnalyzeArgs>,
    ) -> Result<String, ErrorData> {
        if args.domain != "vyakarana" {
            return Err(to_error_data(PaniniError::InvalidArgument {
                tool: "panini_analyze".into(),
                argument: "domain".into(),
                constraint: "must be 'vyakarana'".into(),
                received: args.domain,
            }));
        }
        if args.operation != "sandhi" {
            return Err(to_error_data(PaniniError::InvalidArgument {
                tool: "panini_analyze".into(),
                argument: "operation".into(),
                constraint: "must be 'sandhi'".into(),
                received: args.operation,
            }));
        }

        let rules = self.cache.get_rules("sandhi_rule");
        if rules.is_empty() {
            return Err(to_error_data(PaniniError::NoRulesLoaded(
                "sandhi_rule".into(),
            )));
        }

        let analyze_result = analyze_sandhi(rules, &args.form).map_err(to_error_data)?;

        let out = AnalyzeOutput {
            domain: args.domain,
            operation: args.operation,
            form: args.form,
            candidates: analyze_result.candidates,
        };
        serde_json::to_string_pretty(&out).map_err(json_err)
    }

    #[tool(description = "Full paradigm generation. domain=vyakarana. Takes stem and stem_type, returns 24-cell grid (8 cases × 3 numbers) with derived forms and sūtra-cited traces.")]
    pub async fn panini_paradigm(
        &self,
        Parameters(args): Parameters<ParadigmArgs>,
    ) -> Result<String, ErrorData> {
        if args.domain != "vyakarana" {
            return Err(to_error_data(PaniniError::InvalidArgument {
                tool: "panini_paradigm".into(),
                argument: "domain".into(),
                constraint: "must be 'vyakarana'".into(),
                received: args.domain,
            }));
        }

        let sup = self.cache.get_rules("sup_suffix");
        let pratyaya = self.cache.get_rules("pratyaya_rule");
        let anga = self.cache.get_rules("anga_rule");
        let sandhi = self.cache.get_rules("sandhi_rule");
        let tripadi = self.cache.get_rules("tripadi_rule");

        let mut cells = Vec::with_capacity(24);
        for case in CASES {
            for number in NUMBERS {
                let input = DeclensionInput {
                    stem: args.stem.clone(),
                    stem_type: args.stem_type.clone(),
                    case: case.into(),
                    number: number.into(),
                };
                match derive_declension(sup, pratyaya, anga, sandhi, tripadi, input) {
                    Ok(result) => {
                        let form = result.output["form"].as_str().map(String::from);
                        cells.push(ParadigmCell {
                            case: case.into(),
                            number: number.into(),
                            form,
                            trace: Some(result.trace),
                            error: None,
                        });
                    }
                    Err(e) => {
                        cells.push(ParadigmCell {
                            case: case.into(),
                            number: number.into(),
                            form: None,
                            trace: None,
                            error: Some(e.to_string()),
                        });
                    }
                }
            }
        }

        let out = ParadigmOutput {
            domain: args.domain,
            stem: args.stem,
            stem_type: args.stem_type,
            cells,
        };
        serde_json::to_string_pretty(&out).map_err(json_err)
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for PaniniServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(
                "panini v0.1.0 — Sanskrit grammatical derivation engine. \
                 Tools: panini_health, panini_derive (sandhi|declension), panini_analyze, panini_paradigm.",
            )
    }
}

fn json_err(e: serde_json::Error) -> ErrorData {
    ErrorData::internal_error(format!("JSON serialization error: {e}"), None)
}

fn to_error_data(e: PaniniError) -> ErrorData {
    ErrorData::internal_error(e.to_string(), None)
}
