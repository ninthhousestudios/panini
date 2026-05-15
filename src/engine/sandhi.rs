use serde::Deserialize;

use super::phoneme::{phoneme_ends_with, phoneme_starts_with, phoneme_strip_prefix, phoneme_strip_suffix};
use super::{DeriveResult, TraceStep};
use crate::error::Result;
use crate::rule_cache::CachedRule;

#[derive(Debug, Deserialize)]
pub struct SandhiInput {
    pub first: String,
    pub second: String,
}

#[derive(Debug, Deserialize)]
struct SandhiParams {
    first: String,
    second: String,
    result: String,
    #[serde(default)]
    sutra: String,
    #[serde(default)]
    sutra_position: String,
    #[serde(default)]
    rule_type: String,
    #[serde(default)]
    condition_pratyaya: Option<String>,
}

fn rule_type_priority(rule_type: &str) -> u8 {
    match rule_type {
        "apavāda" | "apavada" => 4,
        "nitya" => 3,
        "paribhāṣā" | "paribhasha" => 2,
        "utsarga" => 1,
        _ => 0,
    }
}

pub fn derive_sandhi(rules: &[CachedRule], input: SandhiInput) -> Result<DeriveResult> {
    let mut parsed_rules: Vec<(SandhiParams, &CachedRule)> = rules
        .iter()
        .filter_map(|rule| {
            serde_json::from_value::<SandhiParams>(rule.params.clone())
                .ok()
                .filter(|p| p.condition_pratyaya.is_none())
                .map(|p| (p, rule))
        })
        .collect();

    parsed_rules.sort_by(|(a, _), (b, _)| {
        let pa = rule_type_priority(&a.rule_type);
        let pb = rule_type_priority(&b.rule_type);
        pb.cmp(&pa)
            .then_with(|| b.sutra_position.cmp(&a.sutra_position))
    });

    let mut trace = Vec::new();
    let mut current_first = input.first.clone();
    let mut current_second = input.second.clone();
    let mut result_str = format!("{}{}", current_first, current_second);

    for iteration in 0..100 {
        let mut matched = false;

        for (params, rule) in &parsed_rules {
            if phoneme_ends_with(&current_first, &params.first)
                && phoneme_starts_with(&current_second, &params.second)
            {
                let input_state = format!("{} + {}", current_first, current_second);

                let prefix = phoneme_strip_suffix(&current_first, &params.first).unwrap();
                let suffix = phoneme_strip_prefix(&current_second, &params.second).unwrap();
                result_str = format!("{}{}{}", prefix, params.result, suffix);

                trace.push(TraceStep {
                    step: iteration + 1,
                    rule: rule.statement.clone(),
                    rule_ref: if params.sutra.is_empty() {
                        None
                    } else {
                        Some(params.sutra.clone())
                    },
                    input_state,
                    output_state: result_str.clone(),
                });

                current_first = result_str.clone();
                current_second = String::new();
                matched = true;
                break;
            }
        }

        if !matched || current_second.is_empty() {
            break;
        }
    }

    Ok(DeriveResult {
        output: serde_json::json!({
            "input": format!("{} + {}", input.first, input.second),
            "result": result_str,
            "steps": trace.len(),
        }),
        trace,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_rules() -> Vec<CachedRule> {
        let rules_json = vec![
            // savarna-dirgha: a + a → ā (6.1.101)
            serde_json::json!({"first": "a", "second": "a", "result": "ā", "sutra": "6.1.101", "sutra_position": "06.01.101", "rule_type": "utsarga"}),
            // guna: a + i → e (6.1.87)
            serde_json::json!({"first": "a", "second": "i", "result": "e", "sutra": "6.1.87", "sutra_position": "06.01.087", "rule_type": "utsarga"}),
            // guna: a + u → o (6.1.87)
            serde_json::json!({"first": "a", "second": "u", "result": "o", "sutra": "6.1.87", "sutra_position": "06.01.087", "rule_type": "utsarga"}),
            // vrddhi: a + e → ai (6.1.88)
            serde_json::json!({"first": "a", "second": "e", "result": "ai", "sutra": "6.1.88", "sutra_position": "06.01.088", "rule_type": "utsarga"}),
            // vrddhi: a + o → au (6.1.88)
            serde_json::json!({"first": "a", "second": "o", "result": "au", "sutra": "6.1.88", "sutra_position": "06.01.088", "rule_type": "utsarga"}),
            // yan-sandhi: i + a → ya (6.1.77)
            serde_json::json!({"first": "i", "second": "a", "result": "ya", "sutra": "6.1.77", "sutra_position": "06.01.077", "rule_type": "utsarga"}),
            // yan-sandhi: u + a → va (6.1.77)
            serde_json::json!({"first": "u", "second": "a", "result": "va", "sutra": "6.1.77", "sutra_position": "06.01.077", "rule_type": "utsarga"}),
            // savarna-dirgha: i + i → ī (6.1.101)
            serde_json::json!({"first": "i", "second": "i", "result": "ī", "sutra": "6.1.101", "sutra_position": "06.01.101", "rule_type": "utsarga"}),
            // savarna-dirgha: u + u → ū (6.1.101)
            serde_json::json!({"first": "u", "second": "u", "result": "ū", "sutra": "6.1.101", "sutra_position": "06.01.101", "rule_type": "utsarga"}),
            // r-sandhi: ṛ + a → ra (6.1.77)
            serde_json::json!({"first": "ṛ", "second": "a", "result": "ra", "sutra": "6.1.77", "sutra_position": "06.01.077", "rule_type": "utsarga"}),
            // visarga: aḥ + a → o ' (6.1.109)
            serde_json::json!({"first": "aḥ", "second": "a", "result": "o '", "sutra": "6.1.109", "sutra_position": "06.01.109", "rule_type": "utsarga"}),
            // visarga: aḥ + voiced consonant → o (8.3.17)
            serde_json::json!({"first": "aḥ", "second": "g", "result": "og", "sutra": "8.3.17", "sutra_position": "08.03.017", "rule_type": "utsarga"}),
        ];

        rules_json
            .into_iter()
            .enumerate()
            .map(|(i, params)| CachedRule {
                params,
                statement: format!("test rule {}", i + 1),
            })
            .collect()
    }

    #[test]
    fn guna_a_plus_i() {
        let rules = fixture_rules();
        let result = derive_sandhi(
            &rules,
            SandhiInput {
                first: "a".into(),
                second: "i".into(),
            },
        )
        .unwrap();
        assert_eq!(result.output["result"], "e");
        assert_eq!(result.trace.len(), 1);
        assert_eq!(result.trace[0].rule_ref.as_deref(), Some("6.1.87"));
        assert_eq!(result.trace[0].input_state, "a + i");
        assert_eq!(result.trace[0].output_state, "e");
    }

    #[test]
    fn savarna_dirgha_a_plus_a() {
        let rules = fixture_rules();
        let result = derive_sandhi(
            &rules,
            SandhiInput {
                first: "a".into(),
                second: "a".into(),
            },
        )
        .unwrap();
        assert_eq!(result.output["result"], "ā");
    }

    #[test]
    fn vrddhi_a_plus_e() {
        let rules = fixture_rules();
        let result = derive_sandhi(
            &rules,
            SandhiInput {
                first: "a".into(),
                second: "e".into(),
            },
        )
        .unwrap();
        assert_eq!(result.output["result"], "ai");
        assert_eq!(result.trace[0].rule_ref.as_deref(), Some("6.1.88"));
    }

    #[test]
    fn yan_sandhi_i_plus_a() {
        let rules = fixture_rules();
        let result = derive_sandhi(
            &rules,
            SandhiInput {
                first: "i".into(),
                second: "a".into(),
            },
        )
        .unwrap();
        assert_eq!(result.output["result"], "ya");
        assert_eq!(result.trace[0].rule_ref.as_deref(), Some("6.1.77"));
    }

    #[test]
    fn word_level_sandhi() {
        let rules = fixture_rules();
        let result = derive_sandhi(
            &rules,
            SandhiInput {
                first: "deva".into(),
                second: "indra".into(),
            },
        )
        .unwrap();
        assert_eq!(result.output["result"], "devendra");
    }

    #[test]
    fn all_ten_vowel_cases() {
        let rules = fixture_rules();
        let cases = vec![
            ("a", "a", "ā"),
            ("a", "i", "e"),
            ("a", "u", "o"),
            ("a", "e", "ai"),
            ("a", "o", "au"),
            ("i", "a", "ya"),
            ("u", "a", "va"),
            ("i", "i", "ī"),
            ("u", "u", "ū"),
            ("ṛ", "a", "ra"),
        ];
        for (first, second, expected) in cases {
            let result = derive_sandhi(
                &rules,
                SandhiInput {
                    first: first.into(),
                    second: second.into(),
                },
            )
            .unwrap();
            assert_eq!(
                result.output["result"], expected,
                "{first} + {second} should be {expected}"
            );
        }
    }

    #[test]
    fn visarga_a_plus_a() {
        let rules = fixture_rules();
        let result = derive_sandhi(
            &rules,
            SandhiInput {
                first: "devaḥ".into(),
                second: "atra".into(),
            },
        )
        .unwrap();
        assert_eq!(result.output["result"], "devo 'tra");
        assert_eq!(result.trace[0].rule_ref.as_deref(), Some("6.1.109"));
    }

    #[test]
    fn visarga_before_voiced_consonant() {
        let rules = fixture_rules();
        let result = derive_sandhi(
            &rules,
            SandhiInput {
                first: "devaḥ".into(),
                second: "gacchati".into(),
            },
        )
        .unwrap();
        assert_eq!(result.output["result"], "devogacchati");
        assert_eq!(result.trace[0].rule_ref.as_deref(), Some("8.3.17"));
    }

    #[test]
    fn no_matching_rule() {
        let rules = fixture_rules();
        let result = derive_sandhi(
            &rules,
            SandhiInput {
                first: "tat".into(),
                second: "kim".into(),
            },
        )
        .unwrap();
        assert_eq!(result.output["result"], "tatkim");
        assert!(result.trace.is_empty());
    }

    #[test]
    fn empty_rules() {
        let result = derive_sandhi(
            &[],
            SandhiInput {
                first: "a".into(),
                second: "i".into(),
            },
        )
        .unwrap();
        assert_eq!(result.output["result"], "ai");
        assert!(result.trace.is_empty());
    }
}
