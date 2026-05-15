use serde::Deserialize;

use super::phoneme::{phoneme_ends_with, phoneme_starts_with, phoneme_strip_prefix, phoneme_strip_suffix, tokenize};
use super::{AnalyzeCandidate, AnalyzeResult, DeriveResult, TraceStep, rule_type_priority};
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

pub fn validate_rules(rules: &[CachedRule]) -> Vec<String> {
    rules
        .iter()
        .enumerate()
        .filter_map(|(i, rule)| {
            match serde_json::from_value::<SandhiParams>(rule.params.clone()) {
                Ok(_) => None,
                Err(e) => Some(format!(
                    "rule {i} ({}): {e}",
                    rule.statement
                )),
            }
        })
        .collect()
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

pub fn analyze_sandhi(rules: &[CachedRule], form: &str) -> Result<AnalyzeResult> {
    let parsed_rules: Vec<(SandhiParams, &CachedRule)> = rules
        .iter()
        .filter_map(|rule| {
            serde_json::from_value::<SandhiParams>(rule.params.clone())
                .ok()
                .filter(|p| p.condition_pratyaya.is_none())
                .map(|p| (p, rule))
        })
        .collect();

    let tokens = tokenize(form);
    let mut candidates = Vec::new();

    for (params, rule) in &parsed_rules {
        let result_tokens = tokenize(&params.result);
        if result_tokens.is_empty() {
            continue;
        }
        let rlen = result_tokens.len();
        if rlen > tokens.len() {
            continue;
        }

        for i in 0..=tokens.len() - rlen {
            if tokens[i..i + rlen] != result_tokens[..] {
                continue;
            }
            let prefix: String = tokens[..i].concat();
            let suffix: String = tokens[i + rlen..].concat();
            let first = format!("{}{}", prefix, params.first);
            let second = format!("{}{}", params.second, suffix);

            if first.is_empty() || second.is_empty() {
                continue;
            }

            candidates.push(AnalyzeCandidate {
                first,
                second,
                rule: rule.statement.clone(),
                rule_ref: if params.sutra.is_empty() {
                    None
                } else {
                    Some(params.sutra.clone())
                },
                specificity: rule_type_priority(&params.rule_type),
            });
        }
    }

    candidates.sort_by(|a, b| b.specificity.cmp(&a.specificity));

    Ok(AnalyzeResult {
        input: form.to_string(),
        candidates,
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
            // 8.4.40 stoḥ ścunā ścuḥ — dental → palatal
            serde_json::json!({"first": "t", "second": "c", "result": "cc", "sutra": "8.4.40", "sutra_position": "08.04.040", "rule_type": "utsarga"}),
            serde_json::json!({"first": "t", "second": "ch", "result": "cch", "sutra": "8.4.40", "sutra_position": "08.04.040", "rule_type": "utsarga"}),
            serde_json::json!({"first": "t", "second": "j", "result": "jj", "sutra": "8.4.40", "sutra_position": "08.04.040", "rule_type": "utsarga"}),
            serde_json::json!({"first": "t", "second": "jh", "result": "jjh", "sutra": "8.4.40", "sutra_position": "08.04.040", "rule_type": "utsarga"}),
            serde_json::json!({"first": "d", "second": "c", "result": "cc", "sutra": "8.4.40", "sutra_position": "08.04.040", "rule_type": "utsarga"}),
            serde_json::json!({"first": "d", "second": "ch", "result": "cch", "sutra": "8.4.40", "sutra_position": "08.04.040", "rule_type": "utsarga"}),
            serde_json::json!({"first": "d", "second": "j", "result": "jj", "sutra": "8.4.40", "sutra_position": "08.04.040", "rule_type": "utsarga"}),
            serde_json::json!({"first": "d", "second": "jh", "result": "jjh", "sutra": "8.4.40", "sutra_position": "08.04.040", "rule_type": "utsarga"}),
            serde_json::json!({"first": "n", "second": "c", "result": "ñc", "sutra": "8.4.40", "sutra_position": "08.04.040", "rule_type": "utsarga"}),
            serde_json::json!({"first": "n", "second": "ch", "result": "ñch", "sutra": "8.4.40", "sutra_position": "08.04.040", "rule_type": "utsarga"}),
            serde_json::json!({"first": "n", "second": "j", "result": "ñj", "sutra": "8.4.40", "sutra_position": "08.04.040", "rule_type": "utsarga"}),
            serde_json::json!({"first": "n", "second": "jh", "result": "ñjh", "sutra": "8.4.40", "sutra_position": "08.04.040", "rule_type": "utsarga"}),
            serde_json::json!({"first": "t", "second": "ś", "result": "cch", "sutra": "8.4.40", "sutra_position": "08.04.040", "rule_type": "utsarga"}),
            serde_json::json!({"first": "d", "second": "ś", "result": "cch", "sutra": "8.4.40", "sutra_position": "08.04.040", "rule_type": "utsarga"}),
            serde_json::json!({"first": "n", "second": "ś", "result": "ñch", "sutra": "8.4.40", "sutra_position": "08.04.040", "rule_type": "utsarga"}),
            // 8.4.41 ṣṭunā ṣṭuḥ — dental → retroflex
            serde_json::json!({"first": "t", "second": "ṭ", "result": "ṭṭ", "sutra": "8.4.41", "sutra_position": "08.04.041", "rule_type": "utsarga"}),
            serde_json::json!({"first": "t", "second": "ṭh", "result": "ṭṭh", "sutra": "8.4.41", "sutra_position": "08.04.041", "rule_type": "utsarga"}),
            serde_json::json!({"first": "t", "second": "ḍ", "result": "ḍḍ", "sutra": "8.4.41", "sutra_position": "08.04.041", "rule_type": "utsarga"}),
            serde_json::json!({"first": "t", "second": "ḍh", "result": "ḍḍh", "sutra": "8.4.41", "sutra_position": "08.04.041", "rule_type": "utsarga"}),
            serde_json::json!({"first": "d", "second": "ṭ", "result": "ṭṭ", "sutra": "8.4.41", "sutra_position": "08.04.041", "rule_type": "utsarga"}),
            serde_json::json!({"first": "d", "second": "ṭh", "result": "ṭṭh", "sutra": "8.4.41", "sutra_position": "08.04.041", "rule_type": "utsarga"}),
            serde_json::json!({"first": "d", "second": "ḍ", "result": "ḍḍ", "sutra": "8.4.41", "sutra_position": "08.04.041", "rule_type": "utsarga"}),
            serde_json::json!({"first": "d", "second": "ḍh", "result": "ḍḍh", "sutra": "8.4.41", "sutra_position": "08.04.041", "rule_type": "utsarga"}),
            serde_json::json!({"first": "n", "second": "ṭ", "result": "ṇṭ", "sutra": "8.4.41", "sutra_position": "08.04.041", "rule_type": "utsarga"}),
            serde_json::json!({"first": "n", "second": "ṭh", "result": "ṇṭh", "sutra": "8.4.41", "sutra_position": "08.04.041", "rule_type": "utsarga"}),
            serde_json::json!({"first": "n", "second": "ḍ", "result": "ṇḍ", "sutra": "8.4.41", "sutra_position": "08.04.041", "rule_type": "utsarga"}),
            serde_json::json!({"first": "n", "second": "ḍh", "result": "ṇḍh", "sutra": "8.4.41", "sutra_position": "08.04.041", "rule_type": "utsarga"}),
            // 8.4.53 jhalāṃ jaś jhali — voicing before voiced consonant
            serde_json::json!({"first": "t", "second": "g", "result": "dg", "sutra": "8.4.53", "sutra_position": "08.04.053", "rule_type": "utsarga"}),
            serde_json::json!({"first": "t", "second": "gh", "result": "dgh", "sutra": "8.4.53", "sutra_position": "08.04.053", "rule_type": "utsarga"}),
            serde_json::json!({"first": "t", "second": "d", "result": "dd", "sutra": "8.4.53", "sutra_position": "08.04.053", "rule_type": "utsarga"}),
            serde_json::json!({"first": "t", "second": "dh", "result": "ddh", "sutra": "8.4.53", "sutra_position": "08.04.053", "rule_type": "utsarga"}),
            serde_json::json!({"first": "t", "second": "b", "result": "db", "sutra": "8.4.53", "sutra_position": "08.04.053", "rule_type": "utsarga"}),
            serde_json::json!({"first": "t", "second": "bh", "result": "dbh", "sutra": "8.4.53", "sutra_position": "08.04.053", "rule_type": "utsarga"}),
            serde_json::json!({"first": "k", "second": "g", "result": "gg", "sutra": "8.4.53", "sutra_position": "08.04.053", "rule_type": "utsarga"}),
            serde_json::json!({"first": "k", "second": "gh", "result": "ggh", "sutra": "8.4.53", "sutra_position": "08.04.053", "rule_type": "utsarga"}),
            serde_json::json!({"first": "k", "second": "j", "result": "gj", "sutra": "8.4.53", "sutra_position": "08.04.053", "rule_type": "utsarga"}),
            serde_json::json!({"first": "k", "second": "d", "result": "gd", "sutra": "8.4.53", "sutra_position": "08.04.053", "rule_type": "utsarga"}),
            serde_json::json!({"first": "k", "second": "dh", "result": "gdh", "sutra": "8.4.53", "sutra_position": "08.04.053", "rule_type": "utsarga"}),
            serde_json::json!({"first": "k", "second": "b", "result": "gb", "sutra": "8.4.53", "sutra_position": "08.04.053", "rule_type": "utsarga"}),
            serde_json::json!({"first": "k", "second": "bh", "result": "gbh", "sutra": "8.4.53", "sutra_position": "08.04.053", "rule_type": "utsarga"}),
            serde_json::json!({"first": "p", "second": "g", "result": "bg", "sutra": "8.4.53", "sutra_position": "08.04.053", "rule_type": "utsarga"}),
            serde_json::json!({"first": "p", "second": "d", "result": "bd", "sutra": "8.4.53", "sutra_position": "08.04.053", "rule_type": "utsarga"}),
            serde_json::json!({"first": "p", "second": "b", "result": "bb", "sutra": "8.4.53", "sutra_position": "08.04.053", "rule_type": "utsarga"}),
            serde_json::json!({"first": "p", "second": "bh", "result": "bbh", "sutra": "8.4.53", "sutra_position": "08.04.053", "rule_type": "utsarga"}),
            // 8.4.45 nasal assimilation
            serde_json::json!({"first": "t", "second": "n", "result": "nn", "sutra": "8.4.45", "sutra_position": "08.04.045", "rule_type": "utsarga"}),
            serde_json::json!({"first": "t", "second": "m", "result": "nm", "sutra": "8.4.45", "sutra_position": "08.04.045", "rule_type": "utsarga"}),
            serde_json::json!({"first": "k", "second": "ṅ", "result": "ṅṅ", "sutra": "8.4.45", "sutra_position": "08.04.045", "rule_type": "utsarga"}),
            serde_json::json!({"first": "k", "second": "n", "result": "ṅn", "sutra": "8.4.45", "sutra_position": "08.04.045", "rule_type": "utsarga"}),
            serde_json::json!({"first": "k", "second": "m", "result": "ṅm", "sutra": "8.4.45", "sutra_position": "08.04.045", "rule_type": "utsarga"}),
            serde_json::json!({"first": "p", "second": "n", "result": "mn", "sutra": "8.4.45", "sutra_position": "08.04.045", "rule_type": "utsarga"}),
            serde_json::json!({"first": "p", "second": "m", "result": "mm", "sutra": "8.4.45", "sutra_position": "08.04.045", "rule_type": "utsarga"}),
            // 8.3.23 mo'nusvāraḥ — m before consonant → ṃ (representative subset)
            serde_json::json!({"first": "m", "second": "k", "result": "ṃk", "sutra": "8.3.23", "sutra_position": "08.03.023", "rule_type": "utsarga"}),
            serde_json::json!({"first": "m", "second": "g", "result": "ṃg", "sutra": "8.3.23", "sutra_position": "08.03.023", "rule_type": "utsarga"}),
            serde_json::json!({"first": "m", "second": "c", "result": "ṃc", "sutra": "8.3.23", "sutra_position": "08.03.023", "rule_type": "utsarga"}),
            serde_json::json!({"first": "m", "second": "t", "result": "ṃt", "sutra": "8.3.23", "sutra_position": "08.03.023", "rule_type": "utsarga"}),
            serde_json::json!({"first": "m", "second": "p", "result": "ṃp", "sutra": "8.3.23", "sutra_position": "08.03.023", "rule_type": "utsarga"}),
            serde_json::json!({"first": "m", "second": "s", "result": "ṃs", "sutra": "8.3.23", "sutra_position": "08.03.023", "rule_type": "utsarga"}),
            serde_json::json!({"first": "m", "second": "n", "result": "ṃn", "sutra": "8.3.23", "sutra_position": "08.03.023", "rule_type": "utsarga"}),
            // 8.2.39 jhalāṃ jaśo'nte — stop voicing before vowel (representative subset)
            serde_json::json!({"first": "t", "second": "a", "result": "da", "sutra": "8.2.39", "sutra_position": "08.02.039", "rule_type": "utsarga"}),
            serde_json::json!({"first": "t", "second": "ā", "result": "dā", "sutra": "8.2.39", "sutra_position": "08.02.039", "rule_type": "utsarga"}),
            serde_json::json!({"first": "t", "second": "i", "result": "di", "sutra": "8.2.39", "sutra_position": "08.02.039", "rule_type": "utsarga"}),
            serde_json::json!({"first": "t", "second": "u", "result": "du", "sutra": "8.2.39", "sutra_position": "08.02.039", "rule_type": "utsarga"}),
            serde_json::json!({"first": "t", "second": "e", "result": "de", "sutra": "8.2.39", "sutra_position": "08.02.039", "rule_type": "utsarga"}),
            serde_json::json!({"first": "k", "second": "a", "result": "ga", "sutra": "8.2.39", "sutra_position": "08.02.039", "rule_type": "utsarga"}),
            serde_json::json!({"first": "k", "second": "ā", "result": "gā", "sutra": "8.2.39", "sutra_position": "08.02.039", "rule_type": "utsarga"}),
            serde_json::json!({"first": "k", "second": "i", "result": "gi", "sutra": "8.2.39", "sutra_position": "08.02.039", "rule_type": "utsarga"}),
            serde_json::json!({"first": "k", "second": "ī", "result": "gī", "sutra": "8.2.39", "sutra_position": "08.02.039", "rule_type": "utsarga"}),
            serde_json::json!({"first": "ṭ", "second": "a", "result": "ḍa", "sutra": "8.2.39", "sutra_position": "08.02.039", "rule_type": "utsarga"}),
            serde_json::json!({"first": "p", "second": "a", "result": "ba", "sutra": "8.2.39", "sutra_position": "08.02.039", "rule_type": "utsarga"}),
            serde_json::json!({"first": "p", "second": "i", "result": "bi", "sutra": "8.2.39", "sutra_position": "08.02.039", "rule_type": "utsarga"}),
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

    #[test]
    fn analyze_guna_vowel() {
        let rules = fixture_rules();
        let result = analyze_sandhi(&rules, "devendra").unwrap();
        let found = result
            .candidates
            .iter()
            .any(|c| c.first == "deva" && c.second == "indra");
        assert!(found, "expected deva + indra in candidates: {:#?}", result.candidates);
    }

    #[test]
    fn analyze_visarga_before_a() {
        let rules = fixture_rules();
        // rāmaḥ + atra → rāmo 'tra (rule: aḥ + a → o ')
        let result = analyze_sandhi(&rules, "rāmo 'tra").unwrap();
        let found = result
            .candidates
            .iter()
            .any(|c| c.first == "rāmaḥ" && c.second == "atra");
        assert!(found, "expected rāmaḥ + atra in candidates: {:#?}", result.candidates);
    }

    #[test]
    fn analyze_visarga_before_voiced() {
        let rules = fixture_rules();
        // naraḥ + gacchati → narogacchati (rule: aḥ + g → og)
        let result = analyze_sandhi(&rules, "narogacchati").unwrap();
        let found = result
            .candidates
            .iter()
            .any(|c| c.first == "naraḥ" && c.second == "gacchati");
        assert!(found, "expected naraḥ + gacchati in candidates: {:#?}", result.candidates);
    }

    #[test]
    fn analyze_specificity_ranking() {
        let mut rules = fixture_rules();
        // Add an apavāda rule that also produces "e" from a+i (same result, higher priority)
        rules.push(CachedRule {
            params: serde_json::json!({
                "first": "a", "second": "i", "result": "e",
                "sutra": "6.1.94", "sutra_position": "06.01.094",
                "rule_type": "apavāda"
            }),
            statement: "apavāda guṇa rule".into(),
        });
        let result = analyze_sandhi(&rules, "devendra").unwrap();
        let matching: Vec<_> = result
            .candidates
            .iter()
            .filter(|c| c.first == "deva" && c.second == "indra")
            .collect();
        assert!(matching.len() >= 2, "expected at least 2 matching candidates");
        assert!(
            matching[0].specificity > matching[1].specificity,
            "apavāda ({}) should rank before utsarga ({})",
            matching[0].specificity,
            matching[1].specificity
        );
    }

    #[test]
    fn analyze_no_valid_decomposition() {
        let rules = fixture_rules();
        let result = analyze_sandhi(&rules, "tatkim").unwrap();
        assert!(
            result.candidates.is_empty(),
            "expected no candidates for 'tatkim': {:#?}",
            result.candidates
        );
    }

    #[test]
    fn derive_class_assimilation_palatal() {
        let rules = fixture_rules();
        let result = derive_sandhi(
            &rules,
            SandhiInput { first: "tat".into(), second: "ca".into() },
        ).unwrap();
        assert_eq!(result.output["result"], "tacca");
        assert_eq!(result.trace.len(), 1);
        assert_eq!(result.trace[0].rule_ref.as_deref(), Some("8.4.40"));
    }

    #[test]
    fn derive_class_assimilation_palatal_voiced() {
        let rules = fixture_rules();
        let result = derive_sandhi(
            &rules,
            SandhiInput { first: "tat".into(), second: "jayati".into() },
        ).unwrap();
        assert_eq!(result.output["result"], "tajjayati");
        assert_eq!(result.trace[0].rule_ref.as_deref(), Some("8.4.40"));
    }

    #[test]
    fn derive_class_assimilation_retroflex() {
        let rules = fixture_rules();
        let result = derive_sandhi(
            &rules,
            SandhiInput { first: "tat".into(), second: "ṭīkā".into() },
        ).unwrap();
        assert_eq!(result.output["result"], "taṭṭīkā");
        assert_eq!(result.trace[0].rule_ref.as_deref(), Some("8.4.41"));
    }

    #[test]
    fn derive_voicing_before_voiced() {
        let rules = fixture_rules();
        let result = derive_sandhi(
            &rules,
            SandhiInput { first: "tat".into(), second: "gacchati".into() },
        ).unwrap();
        assert_eq!(result.output["result"], "tadgacchati");
        assert_eq!(result.trace[0].rule_ref.as_deref(), Some("8.4.53"));
    }

    #[test]
    fn derive_nasal_assimilation() {
        let rules = fixture_rules();
        let result = derive_sandhi(
            &rules,
            SandhiInput { first: "tat".into(), second: "nayati".into() },
        ).unwrap();
        assert_eq!(result.output["result"], "tannayati");
        assert_eq!(result.trace[0].rule_ref.as_deref(), Some("8.4.45"));
    }

    #[test]
    fn derive_anusvara() {
        let rules = fixture_rules();
        let result = derive_sandhi(
            &rules,
            SandhiInput { first: "sam".into(), second: "kalpam".into() },
        ).unwrap();
        assert_eq!(result.output["result"], "saṃkalpam");
        assert_eq!(result.trace[0].rule_ref.as_deref(), Some("8.3.23"));
    }

    #[test]
    fn derive_stop_voicing_before_vowel() {
        let rules = fixture_rules();
        let result = derive_sandhi(
            &rules,
            SandhiInput { first: "tat".into(), second: "atra".into() },
        ).unwrap();
        assert_eq!(result.output["result"], "tadatra");
        assert_eq!(result.trace[0].rule_ref.as_deref(), Some("8.2.39"));
    }

    #[test]
    fn derive_velar_voicing_before_vowel() {
        let rules = fixture_rules();
        let result = derive_sandhi(
            &rules,
            SandhiInput { first: "vāk".into(), second: "īśvaraḥ".into() },
        ).unwrap();
        assert_eq!(result.output["result"], "vāgīśvaraḥ");
        assert_eq!(result.trace[0].rule_ref.as_deref(), Some("8.2.39"));
    }

    #[test]
    fn no_false_sandhi_velar_voiceless_palatal() {
        let rules = fixture_rules();
        let result = derive_sandhi(
            &rules,
            SandhiInput { first: "vāk".into(), second: "ca".into() },
        ).unwrap();
        assert_eq!(result.output["result"], "vākca");
        assert!(result.trace.is_empty());
    }

    #[test]
    fn analyze_consonant_round_trip() {
        let rules = fixture_rules();
        let cases = [
            ("tat", "ca"),
            ("tat", "jayati"),
            ("tat", "ṭīkā"),
            ("tat", "gacchati"),
            ("tat", "nayati"),
            ("sam", "kalpam"),
            ("tat", "atra"),
            ("vāk", "īśvaraḥ"),
        ];
        for (first, second) in cases {
            let derived = derive_sandhi(
                &rules,
                SandhiInput { first: first.into(), second: second.into() },
            ).unwrap();
            let combined = derived.output["result"].as_str().unwrap();
            let analyzed = analyze_sandhi(&rules, combined).unwrap();
            let found = analyzed.candidates.iter().any(|c| c.first == first && c.second == second);
            assert!(
                found,
                "consonant round-trip failed: {} + {} → {}: candidates = {:#?}",
                first, second, combined, analyzed.candidates
            );
        }
    }

    #[test]
    fn analyze_consonant_ranking() {
        let mut rules = fixture_rules();
        rules.push(CachedRule {
            params: serde_json::json!({
                "first": "t", "second": "g", "result": "dg",
                "sutra": "8.4.53", "sutra_position": "08.04.053",
                "rule_type": "apavāda"
            }),
            statement: "apavāda voicing rule".into(),
        });
        let result = analyze_sandhi(&rules, "tadgacchati").unwrap();
        let matching: Vec<_> = result
            .candidates
            .iter()
            .filter(|c| c.first == "tat" && c.second == "gacchati")
            .collect();
        assert!(matching.len() >= 2, "expected both utsarga and apavāda candidates");
        assert!(
            matching[0].specificity > matching[1].specificity,
            "apavāda should rank before utsarga"
        );
    }

    #[test]
    fn round_trip_vowel_cases() {
        let rules = fixture_rules();
        let cases = [
            ("deva", "indra"),   // a+i → e (guṇa)
            ("deva", "artha"),   // a+a → ā (savarna-dīrgha)
            ("deva", "udaya"),   // a+u → o (guṇa)
            ("deva", "eṣa"),    // a+e → ai (vṛddhi)
            ("deva", "ojas"),    // a+o → au (vṛddhi)
            ("devi", "atra"),    // i+a → ya (yan)
        ];
        for (first, second) in cases {
            let derived = derive_sandhi(
                &rules,
                SandhiInput { first: first.into(), second: second.into() },
            ).unwrap();
            let combined = derived.output["result"].as_str().unwrap();
            let analyzed = analyze_sandhi(&rules, combined).unwrap();
            let found = analyzed.candidates.iter().any(|c| c.first == first && c.second == second);
            assert!(
                found,
                "round-trip failed for {} + {} → {}: candidates = {:#?}",
                first, second, combined, analyzed.candidates
            );
        }
    }
}
