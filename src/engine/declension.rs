use std::collections::HashSet;

use serde::Deserialize;
use tracing::warn;

use super::phoneme::{first_phoneme, last_phoneme};
use super::{DeclensionAnalysis, DeclensionCandidate, DeriveResult, TraceStep, rule_type_priority};
use crate::error::{PaniniError, Result};
use crate::rule_cache::CachedRule;

#[derive(Debug, Deserialize)]
pub struct DeclensionInput {
    pub stem: String,
    pub stem_type: String,
    pub case: String,
    pub number: String,
}

#[derive(Deserialize)]
pub(crate) struct SupSuffix {
    pub(crate) stem_class: String,
    pub(crate) vibhakti: String,
    pub(crate) vacana: String,
    pub(crate) pratyaya: String,
    pub(crate) suffix: String,
    #[serde(default)]
    pub(crate) markers: Vec<String>,
    #[serde(default)]
    pub(crate) sutra: String,
    #[serde(default)]
    pub(crate) sutra_position: String,
}

#[derive(Deserialize)]
pub(crate) struct PratyayaRule {
    pub(crate) condition_stem_class: String,
    #[serde(default)]
    pub(crate) condition_suffix: String,
    pub(crate) input_suffix: String,
    pub(crate) output_suffix: String,
    #[serde(default)]
    pub(crate) condition_vibhakti: Option<String>,
    #[serde(default)]
    pub(crate) sutra: String,
    #[serde(default)]
    pub(crate) sutra_position: String,
    #[serde(default)]
    pub(crate) rule_type: String,
}

#[derive(Deserialize)]
pub(crate) struct AngaRule {
    pub(crate) condition_stem_final: String,
    #[serde(default)]
    pub(crate) condition_suffix_initial: Option<String>,
    #[serde(default)]
    pub(crate) condition_vacana: Option<String>,
    #[serde(default)]
    pub(crate) operation: Option<String>,
    #[serde(default)]
    pub(crate) operation_target: Option<String>,
    pub(crate) operation_input: String,
    pub(crate) operation_output: String,
    #[serde(default)]
    pub(crate) sutra: String,
    #[serde(default)]
    pub(crate) sutra_position: String,
    #[serde(default)]
    pub(crate) rule_type: String,
}

#[derive(Deserialize)]
pub(crate) struct DeclensionSandhiRule {
    pub(crate) first: String,
    pub(crate) second: String,
    pub(crate) result: String,
    #[serde(default)]
    pub(crate) condition_pratyaya: Option<String>,
    #[serde(default)]
    pub(crate) sutra: String,
    #[serde(default)]
    pub(crate) sutra_position: String,
    #[serde(default)]
    pub(crate) rule_type: String,
}

#[derive(Deserialize)]
pub(crate) struct TripadiRule {
    #[serde(default)]
    pub(crate) condition_preceding: Option<String>,
    #[serde(default)]
    pub(crate) condition_following: Option<String>,
    pub(crate) input: String,
    pub(crate) output: String,
    pub(crate) position: String,
    #[serde(default)]
    pub(crate) sutra: String,
    #[serde(default)]
    pub(crate) sutra_position: String,
    #[serde(default)]
    pub(crate) rule_type: String,
}

fn translate_case(case: &str) -> &str {
    match case {
        "1" | "nominative" => "prathama",
        "2" | "accusative" => "dvitiya",
        "3" | "instrumental" => "tritiya",
        "4" | "dative" => "caturthi",
        "5" | "ablative" => "pancami",
        "6" | "genitive" => "sasthi",
        "7" | "locative" => "saptami",
        "8" | "vocative" => "sambodhana",
        other => other,
    }
}

fn translate_number(number: &str) -> &str {
    match number {
        "sg" | "singular" => "ekavacana",
        "du" | "dual" => "dvivacana",
        "pl" | "plural" => "bahuvacana",
        other => other,
    }
}

fn sutra_ref(sutra: &str) -> Option<String> {
    if sutra.is_empty() { None } else { Some(sutra.to_string()) }
}

pub fn derive_declension(
    sup_rules: &[CachedRule],
    pratyaya_rules: &[CachedRule],
    anga_rules: &[CachedRule],
    sandhi_rules: &[CachedRule],
    tripadi_rules: &[CachedRule],
    input: DeclensionInput,
) -> Result<DeriveResult> {
    let stem_class = &input.stem_type;
    let vibhakti = translate_case(&input.case);
    let vacana = translate_number(&input.number);

    let mut trace = Vec::new();
    let mut step_num = 0usize;

    // --- Layer 1: Suffix selection ---
    let sup = sup_rules
        .iter()
        .find_map(|rule| {
            let p: SupSuffix = serde_json::from_value(rule.params.clone()).ok()?;
            if p.stem_class == *stem_class && p.vibhakti == vibhakti && p.vacana == vacana {
                Some((p, rule))
            } else {
                None
            }
        })
        .ok_or_else(|| PaniniError::RuleParse(format!(
            "no sup_suffix for stem_class={stem_class}, vibhakti={vibhakti}, vacana={vacana}"
        )))?;

    let (sup_params, sup_rule) = sup;
    let pratyaya_name = sup_params.pratyaya.clone();
    let mut current_suffix = sup_params.suffix.clone();
    let mut current_stem = input.stem.clone();

    step_num += 1;
    trace.push(TraceStep {
        step: step_num,
        rule: sup_rule.statement.clone(),
        rule_ref: sutra_ref(&sup_params.sutra),
        input_state: format!(
            "{} + {} ({} {} {})",
            current_stem, pratyaya_name, stem_class, vibhakti, vacana
        ),
        output_state: format!("{} + {}", current_stem, current_suffix),
    });

    // --- Layer 2: Pratyaya modification ---
    let mut parsed_pratyaya: Vec<(PratyayaRule, &CachedRule)> = pratyaya_rules
        .iter()
        .filter_map(|rule| {
            serde_json::from_value::<PratyayaRule>(rule.params.clone())
                .ok()
                .map(|p| (p, rule))
        })
        .collect();
    parsed_pratyaya.sort_by(|(a, _), (b, _)| {
        let pa = rule_type_priority(&a.rule_type);
        let pb = rule_type_priority(&b.rule_type);
        pb.cmp(&pa)
            .then_with(|| b.sutra_position.cmp(&a.sutra_position))
    });

    for (params, rule) in &parsed_pratyaya {
        if params.condition_stem_class != *stem_class {
            continue;
        }
        if params.condition_suffix != pratyaya_name {
            continue;
        }
        if let Some(ref cond_vib) = params.condition_vibhakti {
            if cond_vib != vibhakti {
                continue;
            }
        }
        if params.input_suffix == current_suffix {
            let old_suffix = current_suffix.clone();
            current_suffix = params.output_suffix.clone();
            step_num += 1;
            trace.push(TraceStep {
                step: step_num,
                rule: rule.statement.clone(),
                rule_ref: sutra_ref(&params.sutra),
                input_state: format!("{} + {}", current_stem, old_suffix),
                output_state: format!("{} + {}", current_stem, current_suffix),
            });
            break;
        }
    }

    // --- Layer 3: Anga modification ---
    let mut parsed_anga: Vec<(AngaRule, &CachedRule)> = anga_rules
        .iter()
        .filter_map(|rule| {
            serde_json::from_value::<AngaRule>(rule.params.clone())
                .ok()
                .map(|p| (p, rule))
        })
        .collect();
    parsed_anga.sort_by(|(a, _), (b, _)| {
        let pa = rule_type_priority(&a.rule_type);
        let pb = rule_type_priority(&b.rule_type);
        pb.cmp(&pa)
            .then_with(|| b.sutra_position.cmp(&a.sutra_position))
    });

    let stem_final = current_stem.chars().last().map(|c| c.to_string());
    let suffix_initial = first_phoneme(&current_suffix).map(|s| s.to_string());

    if let Some(ref sf) = stem_final {
        for (params, rule) in &parsed_anga {
            if params.condition_stem_final != *sf {
                continue;
            }
            if let Some(ref cond_si) = params.condition_suffix_initial {
                match suffix_initial {
                    Some(ref si) if si == cond_si => {}
                    _ => continue,
                }
            }
            if let Some(ref cond_vac) = params.condition_vacana {
                if cond_vac != vacana {
                    continue;
                }
            }
            if current_stem.ends_with(&params.operation_input) {
                let old_stem = current_stem.clone();
                current_stem = format!(
                    "{}{}",
                    &current_stem[..current_stem.len() - params.operation_input.len()],
                    params.operation_output
                );
                step_num += 1;
                trace.push(TraceStep {
                    step: step_num,
                    rule: rule.statement.clone(),
                    rule_ref: sutra_ref(&params.sutra),
                    input_state: format!("{} + {}", old_stem, current_suffix),
                    output_state: format!("{} + {}", current_stem, current_suffix),
                });
                break;
            }
        }
    }

    // --- Layer 4: Junction sandhi ---
    if !current_suffix.is_empty() {
        let mut parsed_sandhi: Vec<(DeclensionSandhiRule, &CachedRule)> = sandhi_rules
            .iter()
            .filter_map(|rule| {
                serde_json::from_value::<DeclensionSandhiRule>(rule.params.clone())
                    .ok()
                    .map(|p| (p, rule))
            })
            .collect();
        parsed_sandhi.sort_by(|(a, _), (b, _)| {
            let pa = rule_type_priority(&a.rule_type);
            let pb = rule_type_priority(&b.rule_type);
            pb.cmp(&pa)
                .then_with(|| b.sutra_position.cmp(&a.sutra_position))
        });

        let stem_end = last_phoneme(&current_stem).map(|s| s.to_string());
        let suf_start = first_phoneme(&current_suffix).map(|s| s.to_string());

        if let (Some(ref se), Some(ref ss)) = (stem_end, suf_start) {
            for (params, rule) in &parsed_sandhi {
                if let Some(ref cond_prat) = params.condition_pratyaya {
                    if cond_prat != &pratyaya_name {
                        continue;
                    }
                }
                if params.first == *se && params.second == *ss {
                    let input_state = format!("{} + {}", current_stem, current_suffix);
                    let prefix =
                        &current_stem[..current_stem.len() - se.len()];
                    let remainder =
                        &current_suffix[ss.len()..];
                    let combined = format!("{}{}{}", prefix, params.result, remainder);
                    current_stem = combined;
                    current_suffix = String::new();
                    step_num += 1;
                    trace.push(TraceStep {
                        step: step_num,
                        rule: rule.statement.clone(),
                        rule_ref: sutra_ref(&params.sutra),
                        input_state,
                        output_state: current_stem.clone(),
                    });
                    break;
                }
            }
        }
    }

    let mut result = if current_suffix.is_empty() {
        current_stem
    } else {
        format!("{}{}", current_stem, current_suffix)
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
        match params.position.as_str() {
            "word_final" => {
                if result.ends_with(&params.input) {
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
            "internal" => {
                if params.condition_preceding.as_deref() == Some("iuk") {
                    if let Some(new_result) =
                        try_apply_iuk_retroflexion(&result, &params.input, &params.output)
                    {
                        let old = result.clone();
                        result = new_result;
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
            }
            _ => {}
        }
    }

    Ok(DeriveResult {
        output: serde_json::json!({
            "stem": input.stem,
            "stem_type": input.stem_type,
            "case": input.case,
            "number": input.number,
            "form": result,
            "steps": trace.len(),
        }),
        trace,
    })
}

const IUK_VOWELS: &[char] = &['i', 'u', 'e', 'o'];

fn try_apply_iuk_retroflexion(word: &str, input: &str, output: &str) -> Option<String> {
    let chars: Vec<char> = word.chars().collect();
    for (i, &ch) in chars.iter().enumerate() {
        if ch.to_string() == input && i > 0 && i < chars.len() - 1 {
            let preceding = chars[i - 1];
            if IUK_VOWELS.contains(&preceding) || preceding == 'ṛ' {
                let mut new_word: String = chars[..i].iter().collect();
                new_word.push_str(output);
                let rest: String = chars[i + 1..].iter().collect();
                new_word.push_str(&rest);
                return Some(new_word);
            }
        }
    }
    None
}

const CASES: [&str; 8] = ["1", "2", "3", "4", "5", "6", "7", "8"];
const NUMBERS: [&str; 3] = ["sg", "du", "pl"];

pub fn analyze_declension(
    sup_rules: &[CachedRule],
    pratyaya_rules: &[CachedRule],
    anga_rules: &[CachedRule],
    sandhi_rules: &[CachedRule],
    tripadi_rules: &[CachedRule],
    form: &str,
) -> Result<DeclensionAnalysis> {
    let stem_classes: HashSet<String> = sup_rules
        .iter()
        .filter_map(|r| serde_json::from_value::<SupSuffix>(r.params.clone()).ok())
        .map(|s| s.stem_class)
        .collect();

    let mut candidates = Vec::new();

    for stem_class in &stem_classes {
        let probe_stem = stem_class.split('-').next().unwrap_or(stem_class);
        let mut probe_successes = 0u32;

        for case in &CASES {
            for number in &NUMBERS {
                let probe_input = DeclensionInput {
                    stem: probe_stem.to_string(),
                    stem_type: stem_class.clone(),
                    case: case.to_string(),
                    number: number.to_string(),
                };
                let probe_result = match derive_declension(
                    sup_rules, pratyaya_rules, anga_rules,
                    sandhi_rules, tripadi_rules, probe_input,
                ) {
                    Ok(r) => { probe_successes += 1; r }
                    Err(_) => continue,
                };
                let probe_form = match probe_result.output["form"].as_str() {
                    Some(f) => f,
                    None => continue,
                };

                if probe_form.len() > form.len() || !form.ends_with(probe_form) {
                    continue;
                }

                let stem_base = &form[..form.len() - probe_form.len()];
                let candidate_stem = format!("{}{}", stem_base, probe_stem);

                let verify_input = DeclensionInput {
                    stem: candidate_stem.clone(),
                    stem_type: stem_class.clone(),
                    case: case.to_string(),
                    number: number.to_string(),
                };
                let verify_result = match derive_declension(
                    sup_rules, pratyaya_rules, anga_rules,
                    sandhi_rules, tripadi_rules, verify_input,
                ) {
                    Ok(r) => r,
                    Err(_) => continue,
                };

                if verify_result.output["form"].as_str() == Some(form) {
                    candidates.push(DeclensionCandidate {
                        stem: candidate_stem,
                        stem_type: stem_class.clone(),
                        case: case.to_string(),
                        number: number.to_string(),
                        form: form.to_string(),
                    });
                }
            }
        }

        if probe_successes == 0 {
            warn!(
                stem_class,
                "zero successful probe derivations — probe stem extraction \
                 likely invalid for this stem class"
            );
        }
    }

    candidates.sort_by(|a, b| {
        a.stem_type.cmp(&b.stem_type)
            .then_with(|| a.case.cmp(&b.case))
            .then_with(|| a.number.cmp(&b.number))
    });
    candidates.dedup_by(|a, b| {
        a.stem == b.stem && a.stem_type == b.stem_type
            && a.case == b.case && a.number == b.number
    });

    Ok(DeclensionAnalysis {
        input: form.to_string(),
        candidates,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rule_cache::CachedRule;

    fn make_rule(params: serde_json::Value) -> CachedRule {
        let stmt = params
            .get("sutra")
            .and_then(|v| v.as_str())
            .unwrap_or("test rule")
            .to_string();
        CachedRule {
            statement: stmt,
            params,
        }
    }

    fn fixture_sup_suffixes() -> Vec<CachedRule> {
        let data: Vec<(&str, &str, &str, &str, Vec<&str>, &str)> = vec![
            ("prathama", "ekavacana", "su", "s", vec!["u"], "4.1.2"),
            ("prathama", "dvivacana", "au", "au", vec![], "4.1.2"),
            ("prathama", "bahuvacana", "jas", "as", vec!["j"], "4.1.2"),
            ("dvitiya", "ekavacana", "am", "am", vec![], "4.1.2"),
            ("dvitiya", "dvivacana", "aut", "au", vec!["t"], "4.1.2"),
            ("dvitiya", "bahuvacana", "śas", "as", vec!["ś"], "4.1.2"),
            ("tritiya", "ekavacana", "ṭā", "ā", vec!["ṭ"], "4.1.2"),
            ("tritiya", "dvivacana", "bhyām", "bhyām", vec![], "4.1.2"),
            ("tritiya", "bahuvacana", "bhis", "bhis", vec![], "4.1.2"),
            ("caturthi", "ekavacana", "ṅe", "e", vec!["ṅ"], "4.1.2"),
            ("caturthi", "dvivacana", "bhyām", "bhyām", vec![], "4.1.2"),
            ("caturthi", "bahuvacana", "bhyas", "bhyas", vec![], "4.1.2"),
            ("pancami", "ekavacana", "ṅasi", "as", vec!["ṅ", "i"], "4.1.2"),
            ("pancami", "dvivacana", "bhyām", "bhyām", vec![], "4.1.2"),
            ("pancami", "bahuvacana", "bhyas", "bhyas", vec![], "4.1.2"),
            ("sasthi", "ekavacana", "ṅas", "as", vec!["ṅ"], "4.1.2"),
            ("sasthi", "dvivacana", "os", "os", vec![], "4.1.2"),
            ("sasthi", "bahuvacana", "ām", "ām", vec![], "4.1.2"),
            ("saptami", "ekavacana", "ṅi", "i", vec!["ṅ"], "4.1.2"),
            ("saptami", "dvivacana", "os", "os", vec![], "4.1.2"),
            ("saptami", "bahuvacana", "sup", "su", vec!["p"], "4.1.2"),
            ("sambodhana", "ekavacana", "su", "s", vec!["u"], "4.1.2"),
            ("sambodhana", "dvivacana", "au", "au", vec![], "4.1.2"),
            ("sambodhana", "bahuvacana", "jas", "as", vec!["j"], "4.1.2"),
        ];
        data.into_iter()
            .map(|(vib, vac, prat, suf, markers, sutra)| {
                make_rule(serde_json::json!({
                    "stem_class": "a-stem-m",
                    "vibhakti": vib,
                    "vacana": vac,
                    "pratyaya": prat,
                    "suffix": suf,
                    "markers": markers,
                    "sutra": sutra,
                    "sutra_position": "04.01.002",
                }))
            })
            .collect()
    }

    fn fixture_pratyaya_rules() -> Vec<CachedRule> {
        vec![
            make_rule(serde_json::json!({
                "condition_stem_class": "a-stem-m",
                "condition_suffix": "bhis",
                "condition_markers": [],
                "input_suffix": "bhis",
                "output_suffix": "ais",
                "sutra": "7.1.9",
                "sutra_position": "07.01.009",
                "rule_type": "nitya",
            })),
            make_rule(serde_json::json!({
                "condition_stem_class": "a-stem-m",
                "condition_suffix": "ṭā",
                "condition_markers": [],
                "input_suffix": "ā",
                "output_suffix": "ina",
                "sutra": "7.1.12",
                "sutra_position": "07.01.012",
                "rule_type": "nitya",
            })),
            make_rule(serde_json::json!({
                "condition_stem_class": "a-stem-m",
                "condition_suffix": "ṅasi",
                "condition_markers": [],
                "input_suffix": "as",
                "output_suffix": "t",
                "sutra": "7.1.12",
                "sutra_position": "07.01.012",
                "rule_type": "nitya",
            })),
            make_rule(serde_json::json!({
                "condition_stem_class": "a-stem-m",
                "condition_suffix": "ṅas",
                "condition_markers": [],
                "input_suffix": "as",
                "output_suffix": "sya",
                "sutra": "7.1.12",
                "sutra_position": "07.01.012",
                "rule_type": "nitya",
            })),
            make_rule(serde_json::json!({
                "condition_stem_class": "a-stem-m",
                "condition_suffix": "ṅe",
                "condition_markers": [],
                "input_suffix": "e",
                "output_suffix": "ya",
                "sutra": "7.1.13",
                "sutra_position": "07.01.013",
                "rule_type": "nitya",
            })),
            make_rule(serde_json::json!({
                "condition_stem_class": "a-stem-m",
                "condition_suffix": "ām",
                "condition_markers": [],
                "input_suffix": "ām",
                "output_suffix": "nām",
                "sutra": "7.1.54",
                "sutra_position": "07.01.054",
                "rule_type": "nitya",
            })),
            make_rule(serde_json::json!({
                "condition_stem_class": "a-stem-m",
                "condition_suffix": "su",
                "condition_markers": [],
                "input_suffix": "s",
                "output_suffix": "",
                "sutra": "6.1.69",
                "sutra_position": "06.01.069",
                "rule_type": "nitya",
                "condition_vibhakti": "sambodhana",
            })),
            make_rule(serde_json::json!({
                "condition_stem_class": "a-stem-m",
                "condition_suffix": "śas",
                "condition_markers": [],
                "input_suffix": "as",
                "output_suffix": "n",
                "sutra": "6.1.103",
                "sutra_position": "06.01.103",
                "rule_type": "nitya",
            })),
        ]
    }

    fn fixture_anga_rules() -> Vec<CachedRule> {
        vec![
            // 7.3.101/102: a->aa before y, n, bh, t (utsarga)
            make_rule(serde_json::json!({
                "condition_stem_final": "a", "condition_markers": [],
                "condition_suffix_initial": "y", "condition_vacana": null,
                "operation": "lengthen", "operation_target": "stem_final",
                "operation_input": "a", "operation_output": "ā",
                "sutra": "7.3.101/102", "sutra_position": "07.03.101", "rule_type": "utsarga",
            })),
            make_rule(serde_json::json!({
                "condition_stem_final": "a", "condition_markers": [],
                "condition_suffix_initial": "n", "condition_vacana": null,
                "operation": "lengthen", "operation_target": "stem_final",
                "operation_input": "a", "operation_output": "ā",
                "sutra": "7.3.101/102", "sutra_position": "07.03.101", "rule_type": "utsarga",
            })),
            make_rule(serde_json::json!({
                "condition_stem_final": "a", "condition_markers": [],
                "condition_suffix_initial": "bh", "condition_vacana": null,
                "operation": "lengthen", "operation_target": "stem_final",
                "operation_input": "a", "operation_output": "ā",
                "sutra": "7.3.101/102", "sutra_position": "07.03.101", "rule_type": "utsarga",
            })),
            make_rule(serde_json::json!({
                "condition_stem_final": "a", "condition_markers": [],
                "condition_suffix_initial": "t", "condition_vacana": null,
                "operation": "lengthen", "operation_target": "stem_final",
                "operation_input": "a", "operation_output": "ā",
                "sutra": "7.3.101/102", "sutra_position": "07.03.101", "rule_type": "utsarga",
            })),
            // 7.3.103: a->e before bh/s in bahuvacana (apavada overrides utsarga)
            make_rule(serde_json::json!({
                "condition_stem_final": "a", "condition_markers": [],
                "condition_suffix_initial": "bh", "condition_vacana": "bahuvacana",
                "operation": "substitute", "operation_target": "stem_final",
                "operation_input": "a", "operation_output": "e",
                "sutra": "7.3.103", "sutra_position": "07.03.103", "rule_type": "apavada",
            })),
            make_rule(serde_json::json!({
                "condition_stem_final": "a", "condition_markers": [],
                "condition_suffix_initial": "s", "condition_vacana": "bahuvacana",
                "operation": "substitute", "operation_target": "stem_final",
                "operation_input": "a", "operation_output": "e",
                "sutra": "7.3.103", "sutra_position": "07.03.103", "rule_type": "apavada",
            })),
            // 7.3.104: a->e before o (nitya)
            make_rule(serde_json::json!({
                "condition_stem_final": "a", "condition_markers": [],
                "condition_suffix_initial": "o", "condition_vacana": null,
                "operation": "substitute", "operation_target": "stem_final",
                "operation_input": "a", "operation_output": "e",
                "sutra": "7.3.104", "sutra_position": "07.03.104", "rule_type": "nitya",
            })),
        ]
    }

    fn fixture_sandhi_rules() -> Vec<CachedRule> {
        vec![
            // apavada: a+a->a when pratyaya is "am" (acc sg identity)
            make_rule(serde_json::json!({
                "first": "a", "second": "a", "result": "a",
                "sutra": "6.1.107", "sutra_position": "06.01.107",
                "rule_type": "apavada", "condition_pratyaya": "am",
            })),
            // utsarga vowel sandhi
            make_rule(serde_json::json!({
                "first": "a", "second": "a", "result": "ā",
                "sutra": "6.1.101", "sutra_position": "06.01.101",
                "rule_type": "utsarga",
            })),
            make_rule(serde_json::json!({
                "first": "a", "second": "i", "result": "e",
                "sutra": "6.1.87", "sutra_position": "06.01.087",
                "rule_type": "utsarga",
            })),
            make_rule(serde_json::json!({
                "first": "a", "second": "u", "result": "o",
                "sutra": "6.1.87", "sutra_position": "06.01.087",
                "rule_type": "utsarga",
            })),
            make_rule(serde_json::json!({
                "first": "a", "second": "au", "result": "au",
                "sutra": "6.1.88", "sutra_position": "06.01.088",
                "rule_type": "utsarga",
            })),
            make_rule(serde_json::json!({
                "first": "a", "second": "ai", "result": "ai",
                "sutra": "6.1.88", "sutra_position": "06.01.088",
                "rule_type": "utsarga",
            })),
            // eco'yavayavah: e before vowel -> ay
            make_rule(serde_json::json!({
                "first": "e", "second": "o", "result": "ayo",
                "sutra": "6.1.78", "sutra_position": "06.01.078",
                "rule_type": "utsarga",
            })),
        ]
    }

    fn fixture_tripadi_rules() -> Vec<CachedRule> {
        vec![
            // 8.2.66: word-final s->r
            make_rule(serde_json::json!({
                "context": "word_final", "condition_preceding": null, "condition_following": null,
                "input": "s", "output": "r", "position": "word_final",
                "sutra": "8.2.66", "sutra_position": "08.02.066", "rule_type": "nitya",
            })),
            // 8.3.15: word-final r->visarga
            make_rule(serde_json::json!({
                "context": "word_final", "condition_preceding": null, "condition_following": null,
                "input": "r", "output": "ḥ", "position": "word_final",
                "sutra": "8.3.15", "sutra_position": "08.03.015", "rule_type": "nitya",
            })),
            // 8.3.59: iuk retroflexion s->S
            make_rule(serde_json::json!({
                "context": "after_iuk", "condition_preceding": "iuk", "condition_following": null,
                "input": "s", "output": "ṣ", "position": "internal",
                "sutra": "8.3.59", "sutra_position": "08.03.059", "rule_type": "nitya",
            })),
        ]
    }

    fn derive(case: &str, number: &str) -> DeriveResult {
        derive_declension(
            &fixture_sup_suffixes(),
            &fixture_pratyaya_rules(),
            &fixture_anga_rules(),
            &fixture_sandhi_rules(),
            &fixture_tripadi_rules(),
            DeclensionInput {
                stem: "deva".into(),
                stem_type: "a-stem-m".into(),
                case: case.into(),
                number: number.into(),
            },
        )
        .unwrap()
    }

    fn form(case: &str, number: &str) -> String {
        derive(case, number).output["form"].as_str().unwrap().to_string()
    }

    #[test]
    fn nom_sg() {
        assert_eq!(form("1", "sg"), "devaḥ");
    }

    #[test]
    fn trace_includes_sutra_citations() {
        let r = derive("1", "sg");
        assert!(r.trace.iter().any(|t| t.rule_ref.is_some()));
        let tripadi_step = r.trace.iter().find(|t| t.rule_ref.as_deref() == Some("8.2.66"));
        assert!(tripadi_step.is_some(), "expected tripadi 8.2.66 in trace");
    }

    #[test]
    fn no_op_layers_omitted() {
        let r = derive("1", "du");
        let step_rules: Vec<_> = r.trace.iter().map(|t| t.rule_ref.clone()).collect();
        assert!(
            !step_rules.iter().any(|r| r.as_deref() == Some("7.1.9")),
            "pratyaya rule 7.1.9 should not appear for nom du"
        );
    }

    #[test]
    fn full_deva_paradigm() {
        let expected: Vec<(&str, &str, &str)> = vec![
            ("1", "sg", "devaḥ"),
            ("1", "du", "devau"),
            ("1", "pl", "devāḥ"),
            ("2", "sg", "devam"),
            ("2", "du", "devau"),
            ("2", "pl", "devān"),
            ("3", "sg", "devena"),
            ("3", "du", "devābhyām"),
            ("3", "pl", "devaiḥ"),
            ("4", "sg", "devāya"),
            ("4", "du", "devābhyām"),
            ("4", "pl", "devebhyaḥ"),
            ("5", "sg", "devāt"),
            ("5", "du", "devābhyām"),
            ("5", "pl", "devebhyaḥ"),
            ("6", "sg", "devasya"),
            ("6", "du", "devayoḥ"),
            ("6", "pl", "devānām"),
            ("7", "sg", "deve"),
            ("7", "du", "devayoḥ"),
            ("7", "pl", "deveṣu"),
            ("8", "sg", "deva"),
            ("8", "du", "devau"),
            ("8", "pl", "devāḥ"),
        ];
        for (case, number, exp) in expected {
            assert_eq!(
                form(case, number),
                exp,
                "case={case} number={number} expected={exp}"
            );
        }
    }

    #[test]
    fn english_case_names() {
        assert_eq!(form("nominative", "singular"), "devaḥ");
        assert_eq!(form("accusative", "plural"), "devān");
        assert_eq!(form("instrumental", "dual"), "devābhyām");
    }

    #[test]
    fn iuk_retroflexion() {
        assert_eq!(
            try_apply_iuk_retroflexion("devesu", "s", "ṣ"),
            Some("deveṣu".into())
        );
        assert_eq!(
            try_apply_iuk_retroflexion("devasu", "s", "ṣ"),
            None,
        );
    }

    fn analyze(form: &str) -> super::DeclensionAnalysis {
        analyze_declension(
            &fixture_sup_suffixes(),
            &fixture_pratyaya_rules(),
            &fixture_anga_rules(),
            &fixture_sandhi_rules(),
            &fixture_tripadi_rules(),
            form,
        )
        .unwrap()
    }

    #[test]
    fn analyze_nom_sg() {
        let result = analyze("devaḥ");
        assert!(result.candidates.iter().any(|c| c.stem == "deva"
            && c.case == "1"
            && c.number == "sg"));
    }

    #[test]
    fn analyze_ambiguous_form() {
        let result = analyze("devau");
        let cases: Vec<&str> = result
            .candidates
            .iter()
            .filter(|c| c.stem == "deva" && c.number == "du")
            .map(|c| c.case.as_str())
            .collect();
        assert!(cases.contains(&"1"), "should match nom du");
        assert!(cases.contains(&"2"), "should match acc du");
        assert!(cases.contains(&"8"), "should match voc du");
    }

    #[test]
    fn analyze_no_match() {
        assert!(analyze("xyz").candidates.is_empty());
    }

    #[test]
    fn analyze_round_trip_all_24() {
        for case in CASES {
            for number in NUMBERS {
                let derived = derive(case, number);
                let f = derived.output["form"].as_str().unwrap();
                let analyzed = analyze(f);
                assert!(
                    analyzed.candidates.iter().any(|c| c.stem == "deva"
                        && c.case == case
                        && c.number == number),
                    "round-trip failed: case={case} number={number} form={f}"
                );
            }
        }
    }
}
