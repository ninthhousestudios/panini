pub mod phoneme;
pub mod sandhi;

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct DeriveResult {
    pub output: serde_json::Value,
    pub trace: Vec<TraceStep>,
}

#[derive(Debug, Serialize)]
pub struct TraceStep {
    pub step: usize,
    pub rule: String,
    pub rule_ref: Option<String>,
    pub input_state: String,
    pub output_state: String,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeResult {
    pub input: String,
    pub candidates: Vec<AnalyzeCandidate>,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeCandidate {
    pub first: String,
    pub second: String,
    pub rule: String,
    pub rule_ref: Option<String>,
    pub specificity: u8,
}
