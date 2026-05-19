use std::collections::BTreeMap;
use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use crate::engine::consistency::{
    CheckReport, check_anga_rules, check_pratyaya_rules, check_sandhi_rules, check_sup_suffix,
    check_tripadi_rules,
};
use crate::engine::conjugation::{ConjugationInput, derive_conjugation};
use crate::engine::declension::{DeclensionInput, analyze_declension, derive_declension};
use crate::engine::sandhi::{SandhiInput, analyze_sandhi, derive_sandhi};
use crate::error::PaniniError;
use crate::mcp::{
    AnalyzeArgs, AnalyzeOutput, ConjugationParadigmArgs, ConjugationParadigmOutput,
    DeclensionAnalyzeOutput, DeriveArgs, DeriveOutput, HealthOutput, ParadigmArgs, ParadigmCell,
    ParadigmOutput, CASES, NUMBERS, PURUSHAS, VACANAS,
};
use crate::rule_cache::RuleCache;

pub type AppState = Arc<RuleCache>;

pub struct ApiError(StatusCode, String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, self.1).into_response()
    }
}

impl From<PaniniError> for ApiError {
    fn from(e: PaniniError) -> Self {
        let status = match &e {
            PaniniError::InvalidArgument { .. } => StatusCode::BAD_REQUEST,
            PaniniError::NoRulesLoaded(_) => StatusCode::SERVICE_UNAVAILABLE,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        ApiError(status, e.to_string())
    }
}

pub async fn health(State(cache): State<AppState>) -> Json<HealthOutput> {
    Json(HealthOutput {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        rule_templates: cache.template_count(),
        total_rules: cache.total_rules(),
        sandhi_rules: cache.rule_count("sandhi_rule"),
    })
}

pub async fn derive(
    State(cache): State<AppState>,
    Json(args): Json<DeriveArgs>,
) -> Result<Json<DeriveOutput>, ApiError> {
    validate_domain("derive", &args.domain)?;

    let derive_result = match args.operation.as_str() {
        "sandhi" => {
            let rules = cache.get_rules("sandhi_rule");
            if rules.is_empty() {
                return Err(PaniniError::NoRulesLoaded("sandhi_rule".into()).into());
            }
            let input: SandhiInput =
                serde_json::from_value(args.input.clone()).map_err(|e| {
                    PaniniError::InvalidArgument {
                        tool: "derive".into(),
                        argument: "input".into(),
                        constraint: "requires {first, second}".into(),
                        received: e.to_string(),
                    }
                })?;
            derive_sandhi(rules, input)?
        }
        "declension" => {
            let input: DeclensionInput =
                serde_json::from_value(args.input.clone()).map_err(|e| {
                    PaniniError::InvalidArgument {
                        tool: "derive".into(),
                        argument: "input".into(),
                        constraint: "requires {stem, stem_type, case, number}".into(),
                        received: e.to_string(),
                    }
                })?;
            derive_declension(
                cache.get_rules("sup_suffix"),
                cache.get_rules("pratyaya_rule"),
                cache.get_rules("anga_rule"),
                cache.get_rules("sandhi_rule"),
                cache.get_rules("tripadi_rule"),
                input,
            )
            ?
        }
        "conjugation" => {
            let input: ConjugationInput =
                serde_json::from_value(args.input.clone()).map_err(|e| {
                    PaniniError::InvalidArgument {
                        tool: "derive".into(),
                        argument: "input".into(),
                        constraint: "requires {dhatu, gana, lakara, pada, purusha, vacana}".into(),
                        received: e.to_string(),
                    }
                })?;
            derive_conjugation(
                cache.get_rules("tin_suffix"),
                cache.get_rules("vikarana_rule"),
                cache.get_rules("verb_anga_rule"),
                cache.get_rules("tripadi_rule"),
                input,
            )?
        }
        other => {
            return Err(PaniniError::InvalidArgument {
                tool: "derive".into(),
                argument: "operation".into(),
                constraint: "must be 'sandhi', 'declension', or 'conjugation'".into(),
                received: other.into(),
            }
            .into());
        }
    };

    Ok(Json(DeriveOutput {
        domain: args.domain,
        operation: args.operation,
        input: args.input,
        result: derive_result.output,
        trace: derive_result.trace,
    }))
}

pub async fn analyze(
    State(cache): State<AppState>,
    Json(args): Json<AnalyzeArgs>,
) -> Result<Response, ApiError> {
    validate_domain("analyze", &args.domain)?;

    match args.operation.as_str() {
        "sandhi" => {
            let rules = cache.get_rules("sandhi_rule");
            if rules.is_empty() {
                return Err(PaniniError::NoRulesLoaded("sandhi_rule".into()).into());
            }
            let result = analyze_sandhi(rules, &args.form)?;
            Ok(Json(AnalyzeOutput {
                domain: args.domain,
                operation: args.operation,
                form: args.form,
                candidates: result.candidates,
            })
            .into_response())
        }
        "declension" => {
            let sup = cache.get_rules("sup_suffix");
            if sup.is_empty() {
                return Err(PaniniError::NoRulesLoaded("sup_suffix".into()).into());
            }
            let result = analyze_declension(
                sup,
                cache.get_rules("pratyaya_rule"),
                cache.get_rules("anga_rule"),
                cache.get_rules("sandhi_rule"),
                cache.get_rules("tripadi_rule"),
                &args.form,
            )
            ?;
            Ok(Json(DeclensionAnalyzeOutput {
                domain: args.domain,
                operation: args.operation,
                form: args.form,
                candidates: result.candidates,
            })
            .into_response())
        }
        other => Err(PaniniError::InvalidArgument {
            tool: "analyze".into(),
            argument: "operation".into(),
            constraint: "must be 'sandhi' or 'declension'".into(),
            received: other.into(),
        }
        .into()),
    }
}

pub async fn paradigm(
    State(cache): State<AppState>,
    Json(args): Json<ParadigmArgs>,
) -> Result<Json<ParadigmOutput>, ApiError> {
    validate_domain("paradigm", &args.domain)?;

    let sup = cache.get_rules("sup_suffix");
    let pratyaya = cache.get_rules("pratyaya_rule");
    let anga = cache.get_rules("anga_rule");
    let sandhi = cache.get_rules("sandhi_rule");
    let tripadi = cache.get_rules("tripadi_rule");

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

    Ok(Json(ParadigmOutput {
        domain: args.domain,
        stem: args.stem,
        stem_type: args.stem_type,
        cells,
    }))
}

#[derive(Debug, Clone, Serialize)]
pub struct SutraEntry {
    pub sutra: String,
    pub statement: String,
    pub templates: Vec<String>,
}

pub async fn sutras(State(cache): State<AppState>) -> Json<Vec<SutraEntry>> {
    let mut by_sutra: BTreeMap<String, (String, Vec<String>)> = BTreeMap::new();

    for (template, rules) in cache.all_templates() {
        for rule in rules {
            let sutra = rule.params.get("sutra").and_then(|v| v.as_str()).unwrap_or("");
            if sutra.is_empty() {
                continue;
            }
            let entry = by_sutra
                .entry(sutra.to_string())
                .or_insert_with(|| (rule.statement.clone(), Vec::new()));
            if !entry.1.contains(&template.to_string()) {
                entry.1.push(template.to_string());
            }
        }
    }

    let entries: Vec<SutraEntry> = by_sutra
        .into_iter()
        .map(|(sutra, (statement, templates))| SutraEntry {
            sutra,
            statement,
            templates,
        })
        .collect();

    Json(entries)
}

pub async fn check(State(cache): State<AppState>) -> Json<Vec<CheckReport>> {
    let mut reports = Vec::new();

    let sandhi = cache.get_rules("sandhi_rule");
    if !sandhi.is_empty() {
        reports.push(check_sandhi_rules(sandhi));
    }

    let sup = cache.get_rules("sup_suffix");
    if !sup.is_empty() {
        reports.push(check_sup_suffix(sup));
    }

    let pratyaya = cache.get_rules("pratyaya_rule");
    if !pratyaya.is_empty() {
        reports.push(check_pratyaya_rules(pratyaya));
    }

    let anga = cache.get_rules("anga_rule");
    if !anga.is_empty() {
        reports.push(check_anga_rules(anga));
    }

    let tripadi = cache.get_rules("tripadi_rule");
    if !tripadi.is_empty() {
        reports.push(check_tripadi_rules(tripadi));
    }

    Json(reports)
}

pub async fn conjugation_paradigm(
    State(cache): State<AppState>,
    Json(args): Json<ConjugationParadigmArgs>,
) -> Result<Json<ConjugationParadigmOutput>, ApiError> {
    validate_domain("conjugation_paradigm", &args.domain)?;

    let tin = cache.get_rules("tin_suffix");
    let vikarana = cache.get_rules("vikarana_rule");
    let verb_anga = cache.get_rules("verb_anga_rule");
    let tripadi = cache.get_rules("tripadi_rule");

    let mut cells = Vec::with_capacity(9);
    for purusha in PURUSHAS {
        for vacana in VACANAS {
            let input = ConjugationInput {
                dhatu: args.dhatu.clone(),
                gana: args.gana.clone(),
                lakara: args.lakara.clone(),
                pada: args.pada.clone(),
                purusha: purusha.into(),
                vacana: vacana.into(),
            };
            match derive_conjugation(tin, vikarana, verb_anga, tripadi, input) {
                Ok(result) => {
                    let form = result.output["form"].as_str().map(String::from);
                    cells.push(ParadigmCell {
                        case: purusha.into(),
                        number: vacana.into(),
                        form,
                        trace: Some(result.trace),
                        error: None,
                    });
                }
                Err(e) => {
                    cells.push(ParadigmCell {
                        case: purusha.into(),
                        number: vacana.into(),
                        form: None,
                        trace: None,
                        error: Some(e.to_string()),
                    });
                }
            }
        }
    }

    Ok(Json(ConjugationParadigmOutput {
        domain: args.domain,
        dhatu: args.dhatu,
        gana: args.gana,
        lakara: args.lakara,
        pada: args.pada,
        cells,
    }))
}

fn validate_domain(endpoint: &str, domain: &str) -> Result<(), ApiError> {
    if domain != "vyakarana" {
        return Err(PaniniError::InvalidArgument {
            tool: endpoint.into(),
            argument: "domain".into(),
            constraint: "must be 'vyakarana'".into(),
            received: domain.into(),
        }
        .into());
    }
    Ok(())
}
