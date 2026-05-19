use serde::Deserialize;

use super::declension::TripadiRule;
use super::phoneme::{VOWEL_PHONEMES, tokenize};
use super::{DeriveResult, TraceStep, rule_type_priority};
use crate::error::{PaniniError, Result};
use crate::rule_cache::CachedRule;

#[derive(Debug, Deserialize)]
pub struct ConjugationInput {
    pub dhatu: String,
    pub gana: String,
    pub lakara: String,
    pub pada: String,
    pub purusha: String,
    pub vacana: String,
}

#[derive(Deserialize)]
struct TinSuffix {
    lakara: String,
    purusha: String,
    vacana: String,
    pada: String,
    pratyaya_name: String,
    suffix: String,
    #[serde(default)]
    sutra: String,
}

#[derive(Deserialize)]
struct VikaranaRule {
    gana: String,
    suffix: String,
    #[serde(default)]
    lakara_type: Option<String>,
    #[serde(default)]
    sutra: String,
}

fn lakara_to_type(lakara: &str) -> &'static str {
    match lakara {
        "laṭ" | "loṭ" | "laṅ" | "vidhiliṅ" => "sārvadhātuka",
        "liṭ" | "luṭ" | "lṛṭ" | "āśīrliṅ" | "luṅ" | "lṛṅ" | "leṭ" => "ārdhadhātuka",
        _ => "sārvadhātuka",
    }
}

#[derive(Deserialize)]
struct VerbAngaRule {
    stage: String,
    rule_type: String,
    #[serde(default)]
    condition_dhatu_final: Option<String>,
    #[serde(default)]
    condition_dhatu_vowel: Option<String>,
    #[serde(default)]
    position: Option<String>,
    #[serde(default)]
    input: Option<String>,
    #[serde(default)]
    output: Option<String>,
    #[serde(default)]
    condition_suffix_initial_class: Option<String>,
    #[serde(default)]
    operation_input: Option<String>,
    #[serde(default)]
    operation_output: Option<String>,
    #[serde(default)]
    sutra: String,
}

fn sutra_ref(sutra: &str) -> Option<String> {
    if sutra.is_empty() {
        None
    } else {
        Some(sutra.to_string())
    }
}

const YAN: &[char] = &['y', 'v', 'r', 'l', 'ñ', 'm', 'ṅ', 'ṇ', 'n'];

fn is_yan_initial(suffix: &str) -> bool {
    suffix.chars().next().map_or(false, |c| YAN.contains(&c))
}

fn is_vowel(phoneme: &str) -> bool {
    VOWEL_PHONEMES.contains(&phoneme)
}

fn replace_medial_vowel(dhatu: &str, from: &str, to: &str) -> Option<String> {
    let tokens = tokenize(dhatu);
    if tokens.len() < 3 {
        return None;
    }
    // Medial = not first, not last phoneme
    for i in 1..tokens.len() - 1 {
        if is_vowel(tokens[i]) && tokens[i] == from {
            let mut out = String::new();
            for (j, tok) in tokens.iter().enumerate() {
                if j == i { out.push_str(to); } else { out.push_str(tok); }
            }
            return Some(out);
        }
    }
    None
}

pub fn derive_conjugation(
    tin_rules: &[CachedRule],
    vikarana_rules: &[CachedRule],
    verb_anga_rules: &[CachedRule],
    tripadi_rules: &[CachedRule],
    input: ConjugationInput,
) -> Result<DeriveResult> {
    let mut trace = Vec::new();
    let mut step_num = 0usize;

    // --- Layer 1: Tiṅ selection ---
    let tin = tin_rules
        .iter()
        .find_map(|rule| {
            let p: TinSuffix = serde_json::from_value(rule.params.clone()).ok()?;
            if p.lakara == input.lakara
                && p.purusha == input.purusha
                && p.vacana == input.vacana
                && p.pada == input.pada
            {
                Some((p, rule))
            } else {
                None
            }
        })
        .ok_or_else(|| {
            PaniniError::RuleParse(format!(
                "no tin_suffix for lakāra={}, puruṣa={}, vacana={}, pada={}",
                input.lakara, input.purusha, input.vacana, input.pada
            ))
        })?;

    let (tin_params, tin_rule) = tin;
    let pratyaya_name = tin_params.pratyaya_name.clone();
    let mut current_tin = tin_params.suffix.clone();

    step_num += 1;
    trace.push(TraceStep {
        step: step_num,
        rule: tin_rule.statement.clone(),
        rule_ref: sutra_ref(&tin_params.sutra),
        input_state: format!(
            "{} + {} ({} {} {})",
            input.dhatu, pratyaya_name, input.lakara, input.purusha, input.vacana
        ),
        output_state: format!("{} + {}", input.dhatu, current_tin),
    });

    // --- Layer 2: Vikaraṇa insertion ---
    let input_lakara_type = lakara_to_type(&input.lakara);
    let vik = vikarana_rules
        .iter()
        .find_map(|rule| {
            let p: VikaranaRule = serde_json::from_value(rule.params.clone()).ok()?;
            if p.gana != input.gana {
                return None;
            }
            if let Some(ref lt) = p.lakara_type {
                if lt != input_lakara_type {
                    return None;
                }
            }
            Some((p, rule))
        })
        .ok_or_else(|| {
            PaniniError::RuleParse(format!(
                "no vikarana_rule for gaṇa={}, lakāra_type={}",
                input.gana, input_lakara_type
            ))
        })?;

    let (vik_params, vik_rule) = vik;
    let vikarana = vik_params.suffix.clone();
    let mut current_dhatu = input.dhatu.clone();

    step_num += 1;
    trace.push(TraceStep {
        step: step_num,
        rule: vik_rule.statement.clone(),
        rule_ref: sutra_ref(&vik_params.sutra),
        input_state: format!("{} + {}", current_dhatu, current_tin),
        output_state: format!("{} + {} + {}", current_dhatu, vikarana, current_tin),
    });

    // --- Layer 3: Pre-vikaraṇa aṅga operations ---
    let parsed_anga: Vec<(VerbAngaRule, &CachedRule)> = verb_anga_rules
        .iter()
        .filter_map(|rule| {
            serde_json::from_value::<VerbAngaRule>(rule.params.clone())
                .ok()
                .map(|p| (p, rule))
        })
        .collect();

    // Sub-pass 1: guṇa
    for (params, rule) in parsed_anga
        .iter()
        .filter(|(p, _)| p.stage == "pre_vikarana" && p.rule_type == "guna")
    {
        let pos = match params.position.as_deref() {
            Some(p) => p,
            None => continue,
        };
        let applied = match pos {
            "dhatu_final" => {
                if let (Some(cond), Some(inp), Some(out)) =
                    (&params.condition_dhatu_final, &params.input, &params.output)
                {
                    if current_dhatu.ends_with(cond.as_str()) {
                        let old = current_dhatu.clone();
                        current_dhatu = format!(
                            "{}{}",
                            &current_dhatu[..current_dhatu.len() - inp.len()],
                            out
                        );
                        step_num += 1;
                        trace.push(TraceStep {
                            step: step_num,
                            rule: rule.statement.clone(),
                            rule_ref: sutra_ref(&params.sutra),
                            input_state: format!(
                                "{} + {} + {}",
                                old, vikarana, current_tin
                            ),
                            output_state: format!(
                                "{} + {} + {}",
                                current_dhatu, vikarana, current_tin
                            ),
                        });
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            "dhatu_medial" => {
                if let (Some(_cond), Some(inp), Some(out)) =
                    (&params.condition_dhatu_vowel, &params.input, &params.output)
                {
                    if let Some(replaced) = replace_medial_vowel(&current_dhatu, inp, out) {
                        let old = current_dhatu.clone();
                        current_dhatu = replaced;
                        step_num += 1;
                        trace.push(TraceStep {
                            step: step_num,
                            rule: rule.statement.clone(),
                            rule_ref: sutra_ref(&params.sutra),
                            input_state: format!(
                                "{} + {} + {}",
                                old, vikarana, current_tin
                            ),
                            output_state: format!(
                                "{} + {} + {}",
                                current_dhatu, vikarana, current_tin
                            ),
                        });
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            _ => false,
        };
        if applied {
            break;
        }
    }

    // Sub-pass 2: semivowel
    for (params, rule) in parsed_anga
        .iter()
        .filter(|(p, _)| p.stage == "pre_vikarana" && p.rule_type == "semivowel")
    {
        if let (Some(cond), Some(inp), Some(out)) =
            (&params.condition_dhatu_final, &params.input, &params.output)
        {
            if current_dhatu.ends_with(cond.as_str()) {
                let old = current_dhatu.clone();
                current_dhatu = format!(
                    "{}{}",
                    &current_dhatu[..current_dhatu.len() - inp.len()],
                    out
                );
                step_num += 1;
                trace.push(TraceStep {
                    step: step_num,
                    rule: rule.statement.clone(),
                    rule_ref: sutra_ref(&params.sutra),
                    input_state: format!("{} + {} + {}", old, vikarana, current_tin),
                    output_state: format!(
                        "{} + {} + {}",
                        current_dhatu, vikarana, current_tin
                    ),
                });
                break;
            }
        }
    }

    // Form the aṅga (dhātu + vikaraṇa)
    let mut anga = format!("{}{}", current_dhatu, vikarana);

    // --- Layer 4: Pre-tiṅ operations ---
    // dīrgha: a → ā before yaṅ-initial tiṅ
    for (params, rule) in parsed_anga
        .iter()
        .filter(|(p, _)| p.stage == "pre_tin" && p.rule_type == "dirgha")
    {
        if let (Some(class), Some(inp), Some(out)) = (
            &params.condition_suffix_initial_class,
            &params.input,
            &params.output,
        ) {
            if class == "yaṅ" && is_yan_initial(&current_tin) && anga.ends_with(inp.as_str())
            {
                let old_anga = anga.clone();
                anga = format!("{}{}", &anga[..anga.len() - inp.len()], out);
                step_num += 1;
                trace.push(TraceStep {
                    step: step_num,
                    rule: rule.statement.clone(),
                    rule_ref: sutra_ref(&params.sutra),
                    input_state: format!("{} + {}", old_anga, current_tin),
                    output_state: format!("{} + {}", anga, current_tin),
                });
                break;
            }
        }
    }

    // coalescence: a + a → a at junction
    for (params, rule) in parsed_anga
        .iter()
        .filter(|(p, _)| p.stage == "pre_tin" && p.rule_type == "coalescence")
    {
        if let (Some(op_in), Some(op_out)) =
            (&params.operation_input, &params.operation_output)
        {
            let anga_final = anga
                .chars()
                .last()
                .map(|c| c.to_string())
                .unwrap_or_default();
            let tin_initial = current_tin
                .chars()
                .next()
                .map(|c| c.to_string())
                .unwrap_or_default();
            let junction = format!("{}{}", anga_final, tin_initial);
            if junction == *op_in {
                let old_state = format!("{} + {}", anga, current_tin);
                let anga_prefix = &anga[..anga.len() - anga_final.len()];
                let tin_rest = &current_tin[tin_initial.len()..];
                let combined = format!("{}{}{}", anga_prefix, op_out, tin_rest);
                step_num += 1;
                trace.push(TraceStep {
                    step: step_num,
                    rule: rule.statement.clone(),
                    rule_ref: sutra_ref(&params.sutra),
                    input_state: old_state,
                    output_state: combined.clone(),
                });
                anga = combined;
                current_tin = String::new();
                break;
            }
        }
    }

    // Combine aṅga + tiṅ
    let mut result = if current_tin.is_empty() {
        anga
    } else {
        format!("{}{}", anga, current_tin)
    };

    // --- Layer 5: Tripadi ---
    let mut parsed_tripadi: Vec<(TripadiRule, &CachedRule)> = tripadi_rules
        .iter()
        .filter_map(|rule| {
            serde_json::from_value::<TripadiRule>(rule.params.clone())
                .ok()
                .map(|p| (p, rule))
        })
        .collect();
    parsed_tripadi.sort_by(|(a, _), (b, _)| {
        let pa = rule_type_priority(&a.rule_type);
        let pb = rule_type_priority(&b.rule_type);
        pb.cmp(&pa)
            .then_with(|| a.sutra_position.cmp(&b.sutra_position))
    });

    for (params, rule) in &parsed_tripadi {
        if params.position == "word_final" && result.ends_with(&params.input) {
            let old = result.clone();
            result = format!(
                "{}{}",
                &result[..result.len() - params.input.len()],
                params.output
            );
            step_num += 1;
            trace.push(TraceStep {
                step: step_num,
                rule: rule.statement.clone(),
                rule_ref: sutra_ref(&params.sutra),
                input_state: old,
                output_state: result.clone(),
            });
        }
    }

    Ok(DeriveResult {
        output: serde_json::json!({
            "dhatu": input.dhatu,
            "gana": input.gana,
            "lakara": input.lakara,
            "pada": input.pada,
            "purusha": input.purusha,
            "vacana": input.vacana,
            "form": result,
            "steps": trace.len(),
        }),
        trace,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_rule(params: serde_json::Value, statement: &str) -> CachedRule {
        CachedRule {
            params,
            statement: statement.into(),
        }
    }

    fn fixture_tin() -> Vec<CachedRule> {
        vec![make_rule(
            json!({
                "lakara": "laṭ", "purusha": "prathama", "vacana": "ekavacana",
                "pada": "parasmaipada", "pratyaya_name": "tip", "suffix": "ti",
                "sutra": "3.4.78", "sutra_position": "03.04.078"
            }),
            "prathama ekavacana: tip → ti",
        )]
    }

    fn fixture_vikarana() -> Vec<CachedRule> {
        vec![make_rule(
            json!({
                "gana": "1", "vikarana_name": "śap", "suffix": "a",
                "sutra": "3.1.68", "sutra_position": "03.01.068"
            }),
            "class 1: śap → a",
        )]
    }

    fn fixture_verb_anga() -> Vec<CachedRule> {
        vec![
            make_rule(
                json!({
                    "stage": "pre_vikarana", "rule_type": "guna",
                    "condition_dhatu_final": "ū", "position": "dhatu_final",
                    "input": "ū", "output": "o",
                    "sutra": "7.3.84", "sutra_position": "07.03.084"
                }),
                "ū → o (guṇa)",
            ),
            make_rule(
                json!({
                    "stage": "pre_vikarana", "rule_type": "semivowel",
                    "condition_dhatu_final": "o", "position": "dhatu_final",
                    "input": "o", "output": "av",
                    "sutra": "6.1.78", "sutra_position": "06.01.078"
                }),
                "o → av (semivowel)",
            ),
        ]
    }

    fn fixture_tripadi() -> Vec<CachedRule> {
        vec![
            make_rule(
                json!({
                    "context": "word_final", "position": "word_final",
                    "input": "s", "output": "r",
                    "sutra": "8.2.66", "sutra_position": "08.02.066",
                    "rule_type": "nitya"
                }),
                "s → r word-finally",
            ),
            make_rule(
                json!({
                    "context": "word_final", "position": "word_final",
                    "input": "r", "output": "ḥ",
                    "sutra": "8.3.15", "sutra_position": "08.03.015",
                    "rule_type": "nitya"
                }),
                "r → ḥ at avasāna",
            ),
        ]
    }

    fn derive(
        tin: &[CachedRule],
        vik: &[CachedRule],
        anga: &[CachedRule],
        tri: &[CachedRule],
        dhatu: &str,
    ) -> DeriveResult {
        derive_conjugation(
            tin,
            vik,
            anga,
            tri,
            ConjugationInput {
                dhatu: dhatu.into(),
                gana: "1".into(),
                lakara: "laṭ".into(),
                pada: "parasmaipada".into(),
                purusha: "prathama".into(),
                vacana: "ekavacana".into(),
            },
        )
        .unwrap()
    }

    #[test]
    fn bhuu_prathama_ekavacana() {
        let result = derive(
            &fixture_tin(),
            &fixture_vikarana(),
            &fixture_verb_anga(),
            &fixture_tripadi(),
            "bhū",
        );
        assert_eq!(result.output["form"], "bhavati");
    }

    #[test]
    fn trace_includes_sutra_refs() {
        let result = derive(
            &fixture_tin(),
            &fixture_vikarana(),
            &fixture_verb_anga(),
            &fixture_tripadi(),
            "bhū",
        );
        let refs: Vec<_> = result
            .trace
            .iter()
            .filter_map(|t| t.rule_ref.as_deref())
            .collect();
        assert!(refs.contains(&"3.4.78"));
        assert!(refs.contains(&"3.1.68"));
        assert!(refs.contains(&"7.3.84"));
        assert!(refs.contains(&"6.1.78"));
    }

    #[test]
    fn medial_guna() {
        let anga = vec![make_rule(
            json!({
                "stage": "pre_vikarana", "rule_type": "guna",
                "condition_dhatu_vowel": "u", "position": "dhatu_medial",
                "input": "u", "output": "o",
                "sutra": "7.3.84", "sutra_position": "07.03.084"
            }),
            "u → o (medial guṇa)",
        )];
        let result = derive(
            &fixture_tin(),
            &fixture_vikarana(),
            &anga,
            &fixture_tripadi(),
            "budh",
        );
        assert_eq!(result.output["form"], "bodhati");
    }

    #[test]
    fn no_guna_for_consonant_final() {
        let result = derive(
            &fixture_tin(),
            &fixture_vikarana(),
            &fixture_verb_anga(),
            &fixture_tripadi(),
            "paṭh",
        );
        assert_eq!(result.output["form"], "paṭhati");
    }

    fn medial_guna_anga() -> Vec<CachedRule> {
        vec![make_rule(
            json!({
                "stage": "pre_vikarana", "rule_type": "guna",
                "condition_dhatu_vowel": "u", "position": "dhatu_medial",
                "input": "u", "output": "o",
                "sutra": "7.3.84", "sutra_position": "07.03.084"
            }),
            "u → o (medial guṇa)",
        )]
    }

    #[test]
    fn medial_guna_skips_final_vowel() {
        // "śru" has u as final, not medial — should NOT get medial guṇa
        let result = derive(
            &fixture_tin(),
            &fixture_vikarana(),
            &medial_guna_anga(),
            &fixture_tripadi(),
            "śru",
        );
        assert_eq!(result.output["form"], "śruati",
            "final u should not be rewritten by dhatu_medial rule");
    }

    #[test]
    fn medial_guna_skips_initial_vowel() {
        // "ukṣ" has u as initial — should NOT get medial guṇa
        let result = derive(
            &fixture_tin(),
            &fixture_vikarana(),
            &medial_guna_anga(),
            &fixture_tripadi(),
            "ukṣ",
        );
        assert_eq!(result.output["form"], "ukṣati",
            "initial u should not be rewritten by dhatu_medial rule");
    }

    #[test]
    fn medial_guna_multi_vowel_targets_medial() {
        // "krudh" has u between consonants — should get medial guṇa
        let result = derive(
            &fixture_tin(),
            &fixture_vikarana(),
            &medial_guna_anga(),
            &fixture_tripadi(),
            "krudh",
        );
        assert_eq!(result.output["form"], "krodhati");
    }

    #[test]
    fn vikarana_matches_lakara_type() {
        let vik = vec![make_rule(
            json!({
                "gana": "1", "vikarana_name": "śap", "suffix": "a",
                "lakara_type": "sārvadhātuka",
                "sutra": "3.1.68", "sutra_position": "03.01.068"
            }),
            "class 1: śap → a",
        )];
        let result = derive_conjugation(
            &fixture_tin(),
            &vik,
            &fixture_verb_anga(),
            &fixture_tripadi(),
            ConjugationInput {
                dhatu: "bhū".into(),
                gana: "1".into(),
                lakara: "laṭ".into(),
                pada: "parasmaipada".into(),
                purusha: "prathama".into(),
                vacana: "ekavacana".into(),
            },
        );
        assert!(result.is_ok(), "laṭ (sārvadhātuka) should match");
    }

    #[test]
    fn vikarana_rejects_wrong_lakara_type() {
        let vik = vec![make_rule(
            json!({
                "gana": "1", "vikarana_name": "śap", "suffix": "a",
                "lakara_type": "sārvadhātuka",
                "sutra": "3.1.68", "sutra_position": "03.01.068"
            }),
            "class 1: śap → a",
        )];
        let result = derive_conjugation(
            &fixture_tin(),
            &vik,
            &fixture_verb_anga(),
            &fixture_tripadi(),
            ConjugationInput {
                dhatu: "bhū".into(),
                gana: "1".into(),
                lakara: "liṭ".into(),
                pada: "parasmaipada".into(),
                purusha: "prathama".into(),
                vacana: "ekavacana".into(),
            },
        );
        assert!(result.is_err(), "liṭ (ārdhadhātuka) should not match sārvadhātuka vikaraṇa");
    }
}
