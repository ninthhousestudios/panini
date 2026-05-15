pub mod declension;
pub mod phoneme;
pub mod sandhi;

use serde::Serialize;

pub fn rule_type_priority(rule_type: &str) -> u8 {
    match rule_type {
        "apavāda" | "apavada" => 4,
        "nitya" => 3,
        "paribhāṣā" | "paribhasha" => 2,
        "utsarga" => 1,
        _ => 0,
    }
}

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
