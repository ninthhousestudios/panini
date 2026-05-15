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
