use serde::Deserialize;

use super::declension::TripadiRule;
use super::phoneme::{VOWEL_PHONEMES, first_phoneme, last_phoneme, tokenize};
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
    is_pit: bool,
    #[serde(default)]
    sutra: String,
}

#[derive(Deserialize)]
struct VikaranaRule {
    gana: String,
    #[serde(default)]
    vikarana_name: String,
    suffix: String,
    #[serde(default)]
    lakara_type: Option<String>,
    #[serde(default)]
    it_markers: Vec<String>,
    #[serde(default)]
    sutra: String,
    #[serde(default)]
    insertion_mode: Option<String>,
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
    condition_anga_final: Option<String>,
    #[serde(default)]
    condition_vikarana: Option<String>,
    #[serde(default)]
    condition_suffix_pit: Option<bool>,
    #[serde(default)]
    condition_suffix_initial_type: Option<String>,
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

fn upadha_is_vowel(dhatu: &str) -> bool {
    let tokens = tokenize(dhatu);
    if tokens.len() < 2 {
        return false;
    }
    is_vowel(tokens[tokens.len() - 2])
}

fn is_upadha_laghu(dhatu: &str) -> bool {
    let tokens = tokenize(dhatu);
    if tokens.len() < 2 {
        return false;
    }
    let upadha = tokens[tokens.len() - 2];
    matches!(upadha, "a" | "i" | "u" | "ṛ" | "ḷ")
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

    // Compute ṅit status (1.2.4 + 1.1.5)
    let vikarana_is_nit = {
        let is_sarvadhatuka = vik_params.lakara_type.as_deref() == Some("sārvadhātuka");
        let is_pit = vik_params.it_markers.iter().any(|m| m == "p");
        is_sarvadhatuka && !is_pit
    };
    let vikarana_is_nit_marker = vik_params.it_markers.iter().any(|m| m == "ṇ");
    let is_infix = vik_params.insertion_mode.as_deref() == Some("infix");

    step_num += 1;
    if is_infix {
        let tokens = tokenize(&current_dhatu);
        let final_c = tokens.last().map_or("", |t| *t);
        let prefix = &current_dhatu[..current_dhatu.len() - final_c.len()];
        trace.push(TraceStep {
            step: step_num,
            rule: vik_rule.statement.clone(),
            rule_ref: sutra_ref(&vik_params.sutra),
            input_state: format!("{} + {}", current_dhatu, current_tin),
            output_state: format!("{}{}{} + {}", prefix, vikarana, final_c, current_tin),
        });
    } else {
        trace.push(TraceStep {
            step: step_num,
            rule: vik_rule.statement.clone(),
            rule_ref: sutra_ref(&vik_params.sutra),
            input_state: format!("{} + {}", current_dhatu, current_tin),
            output_state: format!("{} + {} + {}", current_dhatu, vikarana, current_tin),
        });
    }

    // --- Dvitva (reduplication) for gaṇa 3 (6.1.1) ---
    if input.gana == "3" {
        let tokens = tokenize(&current_dhatu);
        let mut abhyasa = String::new();
        for (i, &tok) in tokens.iter().enumerate() {
            if is_vowel(tok) {
                let short = match tok {
                    "ā" => "a",
                    "ī" => "i",
                    "ū" => "u",
                    "ṝ" => "ṛ",
                    other => other,
                };
                abhyasa.push_str(short);
                break;
            } else if i == 0 {
                let deaspirated = match tok {
                    "kh" => "k",
                    "gh" => "g",
                    "ch" => "c",
                    "jh" => "j",
                    "ṭh" => "ṭ",
                    "ḍh" => "ḍ",
                    "th" => "t",
                    "dh" => "d",
                    "ph" => "p",
                    "bh" => "b",
                    other => other,
                };
                let replaced = match deaspirated {
                    "k" => "c",
                    "g" => "j",
                    "ṅ" => "ñ",
                    "h" => "j",
                    other => other,
                };
                abhyasa.push_str(replaced);
            }
        }
        let old_dhatu = current_dhatu.clone();
        current_dhatu = format!("{}{}", abhyasa, old_dhatu);
        step_num += 1;
        trace.push(TraceStep {
            step: step_num,
            rule: format!(
                "{} → {} + {} (dvitva + abhyāsa, Aṣṭ. 6.1.1, 7.4.59-60, 7.4.62)",
                old_dhatu, abhyasa, old_dhatu
            ),
            rule_ref: Some("6.1.1".into()),
            input_state: format!("{} + {}", old_dhatu, current_tin),
            output_state: format!("{} + {}", current_dhatu, current_tin),
        });
    }

    // 7.1.4 ad abhyastāt: jhi → ati after reduplicated stem
    if input.gana == "3" && pratyaya_name == "jhi" {
        let old_tin = current_tin.clone();
        current_tin = "ati".to_string();
        step_num += 1;
        trace.push(TraceStep {
            step: step_num,
            rule: "jhi → ati after abhyasta (ad abhyastāt, Aṣṭ. 7.1.4)".into(),
            rule_ref: Some("7.1.4".into()),
            input_state: format!("{} + {}", current_dhatu, old_tin),
            output_state: format!("{} + {}", current_dhatu, current_tin),
        });
    }

    // --- Layer 3: Pre-vikaraṇa aṅga operations ---
    let parsed_anga: Vec<(VerbAngaRule, &CachedRule)> = verb_anga_rules
        .iter()
        .filter_map(|rule| {
            serde_json::from_value::<VerbAngaRule>(rule.params.clone())
                .ok()
                .map(|p| (p, rule))
        })
        .collect();

    // Sub-pass 1: guṇa or vṛddhi (gated by ṅit — 1.1.5)
    // For ṇit vikaraṇas (gaṇa 10): vṛddhi when upadha is dīrgha vowel,
    // guṇa when upadha is laghu vowel, nothing when upadha is consonant.
    let skip_guna_vrddhi = vikarana_is_nit
        || vikarana.is_empty()
        || (vikarana_is_nit_marker && !upadha_is_vowel(&current_dhatu));
    if !skip_guna_vrddhi {
        let use_vrddhi = vikarana_is_nit_marker && !is_upadha_laghu(&current_dhatu);
        let rule_type_filter = if use_vrddhi { "vrddhi" } else { "guna" };

    for (params, rule) in parsed_anga
        .iter()
        .filter(|(p, _)| p.stage == "pre_vikarana" && p.rule_type == rule_type_filter)
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
    } // end if !skip_guna_vrddhi

    // Form the aṅga (dhātu + vikaraṇa)
    let (mut anga, vikarana_byte_offset) = if is_infix {
        let tokens = tokenize(&current_dhatu);
        let final_c = tokens.last().map_or("", |t| *t);
        let prefix = &current_dhatu[..current_dhatu.len() - final_c.len()];
        let prefix_len = prefix.len();

        // 6.4.111 śnasor allopaḥ: elide 'a' of infix before non-pit tiṅ
        let use_allopa = !tin_params.is_pit && vikarana.ends_with('a');
        let actual_infix = if use_allopa {
            &vikarana[..vikarana.len() - 1]
        } else {
            vikarana.as_str()
        };

        let formed = format!("{}{}{}", prefix, actual_infix, final_c);

        if use_allopa {
            step_num += 1;
            trace.push(TraceStep {
                step: step_num,
                rule: format!("{} → {} (allopaḥ, Aṣṭ. 6.4.111)", vikarana, actual_infix),
                rule_ref: Some("6.4.111".into()),
                input_state: format!("{}{}{} + {}", prefix, vikarana, final_c, current_tin),
                output_state: format!("{} + {}", formed, current_tin),
            });
        }

        (formed, prefix_len)
    } else {
        let offset = current_dhatu.len();
        (format!("{}{}", current_dhatu, vikarana), offset)
    };

    let vikarana_name = vik_params.vikarana_name.clone();
    let tin_is_pit = tin_params.is_pit;

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

    // Sub-pass: guṇa of aṅga upadha before pit tiṅ (7.3.84)
    // When vikaraṇa is ṅit, pre-vikaraṇa root guṇa is blocked. But pit tiṅ
    // independently triggers guṇa of the aṅga's upadha (penultimate phoneme)
    // if it's an iK vowel. E.g. √kṛ + u + ti: aṅga kṛu, upadha ṛ → ar → karoti.
    // Only needed when vikaraṇa is ṅit — otherwise pre-vikaraṇa guṇa already handled it.
    if !current_tin.is_empty() && tin_is_pit && vikarana_is_nit {
        let tokens = tokenize(&anga);
        if tokens.len() >= 2 {
            let upadha_idx = tokens.len() - 2;
            let upadha = tokens[upadha_idx].to_string();
            let guna_replacement = match upadha.as_str() {
                "i" | "ī" => Some("e"),
                "u" | "ū" => Some("o"),
                "ṛ" | "ṝ" => Some("ar"),
                "ḷ" => Some("al"),
                _ => None,
            };
            if let Some(replacement) = guna_replacement {
                let old_anga = anga.clone();
                let mut new_anga = String::new();
                for (j, tok) in tokens.iter().enumerate() {
                    if j == upadha_idx {
                        new_anga.push_str(replacement);
                    } else {
                        new_anga.push_str(tok);
                    }
                }
                anga = new_anga;
                step_num += 1;
                trace.push(TraceStep {
                    step: step_num,
                    rule: format!(
                        "{} → {} (guṇa of upadha before pit sārvadhātuka, Aṣṭ. 7.3.84)",
                        upadha, replacement
                    ),
                    rule_ref: sutra_ref("7.3.84"),
                    input_state: format!("{} + {}", old_anga, current_tin),
                    output_state: format!("{} + {}", anga, current_tin),
                });
            }
        }
    }

    // Sub-pass: guṇa of aṅga-final vowel before consonant-initial pit tiṅ
    // Non-pit sārvadhātuka tiṅ is ṅit (1.2.4), which blocks guṇa (1.1.5)
    if !current_tin.is_empty() && tin_is_pit {
        for (params, rule) in parsed_anga
            .iter()
            .filter(|(p, _)| p.stage == "pre_tin" && p.rule_type == "guna_anga_final")
        {
            if let (Some(cond), Some(inp), Some(out)) =
                (&params.condition_anga_final, &params.input, &params.output)
            {
                let tin_initial_is_consonant = first_phoneme(&current_tin)
                    .map_or(false, |ph| !is_vowel(ph));
                if anga.ends_with(cond.as_str()) && tin_initial_is_consonant {
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
    }

    // Sub-pass: yaṇ at aṅga-tiṅ junction (u → v before vowel)
    if !current_tin.is_empty() {
        for (params, rule) in parsed_anga
            .iter()
            .filter(|(p, _)| p.stage == "pre_tin" && p.rule_type == "yan_junction")
        {
            if let (Some(cond), Some(inp), Some(out)) =
                (&params.condition_anga_final, &params.input, &params.output)
            {
                let tin_initial_is_vowel = first_phoneme(&current_tin)
                    .map_or(false, |ph| is_vowel(ph));
                if anga.ends_with(cond.as_str()) && tin_initial_is_vowel {
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
    }

    // Sub-pass: śnā alternation (nā/nī/n for gaṇa 9)
    if !current_tin.is_empty() {
        for (params, rule) in parsed_anga
            .iter()
            .filter(|(p, _)| p.stage == "pre_tin" && p.rule_type == "sna_alternation")
        {
            if let Some(ref vik_name) = params.condition_vikarana {
                if *vik_name != vikarana_name {
                    continue;
                }
            }
            let pit_match = match params.condition_suffix_pit {
                Some(true) => tin_is_pit,
                Some(false) => !tin_is_pit,
                None => true,
            };
            if !pit_match {
                continue;
            }
            if let Some(ref suf_type) = params.condition_suffix_initial_type {
                let tin_initial_is_vowel = first_phoneme(&current_tin)
                    .map_or(false, |ph| is_vowel(ph));
                let type_match = match suf_type.as_str() {
                    "consonant" => !tin_initial_is_vowel,
                    "vowel" => tin_initial_is_vowel,
                    _ => true,
                };
                if !type_match {
                    continue;
                }
            }
            if let (Some(inp), Some(out)) = (&params.input, &params.output) {
                if anga.ends_with(inp.as_str()) {
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
    }

    // ā-lopa: abhyasta (gaṇa 3) stems ending in ā, before non-pit tiṅ (6.4.64)
    // dadhā + tas → dadh + tas; dadhā + ati → dadh + ati
    if !current_tin.is_empty() && input.gana == "3" && !tin_is_pit && anga.ends_with("ā") {
        let old_state = format!("{} + {}", anga, current_tin);
        anga = anga[..anga.len() - "ā".len()].to_string();
        step_num += 1;
        trace.push(TraceStep {
            step: step_num,
            rule: "ā → lopa in abhyasta before ṅit sārvadhātuka (Aṣṭ. 6.4.64)".into(),
            rule_ref: Some("6.4.64".into()),
            input_state: old_state,
            output_state: format!("{} + {}", anga, current_tin),
        });

        // When ā-lopa creates a voiced-aspirate + voiceless junction (e.g. dadh + tas),
        // 8.4.55 devoices the aspirate (dh → t) and the lost aspiration displaces to
        // the initial consonant of the stem: dadh + tas → dhat + tas → dhattaḥ
        let anga_final_ph = last_phoneme(&anga).unwrap_or("").to_string();
        let tin_initial_ph = first_phoneme(&current_tin).unwrap_or("").to_string();
        let car = match anga_final_ph.as_str() {
            "gh" => Some("k"),
            "jh" => Some("c"),
            "ḍh" => Some("ṭ"),
            "dh" => Some("t"),
            "bh" => Some("p"),
            _ => None,
        };
        let is_khar = matches!(
            tin_initial_ph.as_str(),
            "k" | "kh" | "c" | "ch" | "ṭ" | "ṭh" | "t" | "th" | "p" | "ph"
                | "ś" | "ṣ" | "s"
        );
        if let Some(devoiced) = car {
            if is_khar {
                let old_state = format!("{} + {}", anga, current_tin);
                let anga_prefix = &anga[..anga.len() - anga_final_ph.len()];
                let first_ph = first_phoneme(anga_prefix).unwrap_or("").to_string();
                let aspirated = match first_ph.as_str() {
                    "k" => "kh",
                    "g" => "gh",
                    "c" => "ch",
                    "j" => "jh",
                    "ṭ" => "ṭh",
                    "ḍ" => "ḍh",
                    "t" => "th",
                    "d" => "dh",
                    "p" => "ph",
                    "b" => "bh",
                    other => other,
                };
                let new_prefix =
                    format!("{}{}", aspirated, &anga_prefix[first_ph.len()..]);
                let combined =
                    format!("{}{}{}", new_prefix, devoiced, current_tin);
                step_num += 1;
                trace.push(TraceStep {
                    step: step_num,
                    rule: format!(
                        "{} → {} (khari ca) + aspiration → {} (Aṣṭ. 8.4.55)",
                        anga_final_ph, devoiced, aspirated
                    ),
                    rule_ref: Some("8.4.55".into()),
                    input_state: old_state,
                    output_state: combined.clone(),
                });
                anga = combined;
                current_tin = String::new();
            }
        }
    }

    // ṇatva: n → ṇ in vikaraṇa-derived portion when dhātu has r/ṣ/ṛ/ṝ trigger (8.4.2)
    if vikarana.starts_with('n') {
        let triggers_natva = tokenize(&anga[..vikarana_byte_offset])
            .iter()
            .any(|ph| matches!(*ph, "ṛ" | "ṝ" | "r" | "ṣ"));
        if triggers_natva {
            let vik_portion = &anga[vikarana_byte_offset..];
            if vik_portion.starts_with('n') {
                let old_anga = anga.clone();
                anga = format!(
                    "{}{}",
                    &anga[..vikarana_byte_offset],
                    vik_portion.replacen("n", "ṇ", 1)
                );
                step_num += 1;
                trace.push(TraceStep {
                    step: step_num,
                    rule: "n → ṇ (ṇatva, aṭkupvāṅ... Aṣṭ. 8.4.2)".into(),
                    rule_ref: Some("8.4.2".into()),
                    input_state: format!("{} + {}", old_anga, current_tin),
                    output_state: format!("{} + {}", anga, current_tin),
                });
            }
        }
    }

    // Sub-pass: consonant junction sandhi (8.4.55 khari ca)
    if !current_tin.is_empty() {
        for (params, rule) in parsed_anga
            .iter()
            .filter(|(p, _)| p.stage == "pre_tin" && p.rule_type == "consonant_junction")
        {
            if let (Some(op_in), Some(op_out)) =
                (&params.operation_input, &params.operation_output)
            {
                let anga_final = last_phoneme(&anga).unwrap_or("").to_string();
                let tin_initial = first_phoneme(&current_tin).unwrap_or("").to_string();
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
                "is_pit": true,
                "sutra": "3.4.78", "sutra_position": "03.04.078"
            }),
            "prathama ekavacana: tip → ti",
        )]
    }

    fn fixture_vikarana() -> Vec<CachedRule> {
        vec![make_rule(
            json!({
                "gana": "1", "vikarana_name": "śap", "suffix": "a",
                "it_markers": ["ś", "p"],
                "lakara_type": "sārvadhātuka",
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

    #[test]
    fn sap_it_markers_deserialize() {
        let json = json!({
            "gana": "1", "vikarana_name": "śap", "suffix": "a",
            "it_markers": ["ś", "p"],
            "lakara_type": "sārvadhātuka",
            "sutra": "3.1.68"
        });
        let rule: VikaranaRule = serde_json::from_value(json).unwrap();
        assert_eq!(rule.it_markers, vec!["ś", "p"]);
        let is_sarvadhatuka = rule.lakara_type.as_deref() == Some("sārvadhātuka");
        let is_pit = rule.it_markers.iter().any(|m| m == "p");
        let is_nit = is_sarvadhatuka && !is_pit;
        assert!(!is_nit, "śap is pit, so should NOT be ṅit");
    }

    #[test]
    fn nit_blocks_guna() {
        let vik = vec![make_rule(
            json!({
                "gana": "4", "vikarana_name": "śyan", "suffix": "ya",
                "it_markers": ["ś", "n"],
                "lakara_type": "sārvadhātuka",
                "sutra": "3.1.69", "sutra_position": "03.01.069"
            }),
            "class 4: śyan → ya",
        )];
        let anga = vec![make_rule(
            json!({
                "stage": "pre_vikarana", "rule_type": "guna",
                "condition_dhatu_vowel": "u", "position": "dhatu_medial",
                "input": "u", "output": "o",
                "sutra": "7.3.84", "sutra_position": "07.03.084"
            }),
            "u → o (medial guṇa)",
        )];
        let result = derive_conjugation(
            &fixture_tin(),
            &vik,
            &anga,
            &fixture_tripadi(),
            ConjugationInput {
                dhatu: "div".into(),
                gana: "4".into(),
                lakara: "laṭ".into(),
                pada: "parasmaipada".into(),
                purusha: "prathama".into(),
                vacana: "ekavacana".into(),
            },
        )
        .unwrap();
        assert_eq!(result.output["form"], "divyati",
            "gaṇa 4 śyan is ṅit — guṇa must be blocked");
    }

    #[test]
    fn is_upadha_laghu_check() {
        assert!(is_upadha_laghu("cur"), "upadha 'u' is laghu");
        assert!(is_upadha_laghu("tud"), "upadha 'u' is laghu");
        assert!(!is_upadha_laghu("cint"), "upadha 'n' is consonant, not laghu");
    }
}
