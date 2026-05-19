use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::rule_type_priority;
use crate::rule_cache::CachedRule;

// --- Shared report types ---

#[derive(Debug, Clone, Serialize)]
pub struct CheckReport {
    pub template: String,
    pub total_rules: usize,
    pub parse_errors: Vec<RuleParseError>,
    pub shadowed_rules: Vec<ShadowedRule>,
    pub ambiguous_overlaps: Vec<AmbiguousOverlap>,
    pub coverage: CoverageSummary,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub paradigm_gaps: Vec<ParadigmGap>,
    pub summary: ReportSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuleParseError {
    pub index: usize,
    pub statement: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ShadowedRule {
    pub shadowed_sutra: String,
    pub shadowed_type: String,
    pub shadowed_by_sutra: String,
    pub shadowed_by_type: String,
    pub pattern: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AmbiguousOverlap {
    pub pattern: String,
    pub rule_a_sutra: String,
    pub rule_a_result: String,
    pub rule_a_type: String,
    pub rule_b_sutra: String,
    pub rule_b_result: String,
    pub rule_b_type: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CoverageDimension {
    pub label: String,
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CoverageSummary {
    pub dimensions: Vec<CoverageDimension>,
    pub covered_combinations: usize,
    pub covered_combinations_label: String,
    pub rules_by_sutra: BTreeMap<String, usize>,
    pub rules_by_type: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParadigmGap {
    pub stem_class: String,
    pub missing_cells: Vec<String>,
    pub present: usize,
    pub expected: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReportSummary {
    pub clean: bool,
    pub parse_error_count: usize,
    pub shadowed_count: usize,
    pub ambiguous_count: usize,
    pub verdict: String,
}

// --- Helpers ---

fn count_sutra(sutra: &str, map: &mut BTreeMap<String, usize>) {
    if !sutra.is_empty() {
        *map.entry(sutra.to_string()).or_default() += 1;
    }
}

fn count_type(rule_type: &str, map: &mut BTreeMap<String, usize>) {
    let rtype = if rule_type.is_empty() {
        "unspecified"
    } else {
        rule_type
    };
    *map.entry(rtype.to_string()).or_default() += 1;
}

fn build_summary(
    parsed_count: usize,
    parse_errors: &[RuleParseError],
    shadowed_rules: &[ShadowedRule],
    ambiguous_overlaps: &[AmbiguousOverlap],
    paradigm_gaps: &[ParadigmGap],
    extra_clean_msg: &str,
) -> ReportSummary {
    let clean = parse_errors.is_empty()
        && shadowed_rules.is_empty()
        && ambiguous_overlaps.is_empty()
        && paradigm_gaps.is_empty();

    let verdict = if clean {
        format!("All {} rules are well-formed with no issues. {}", parsed_count, extra_clean_msg)
    } else {
        let mut parts = Vec::new();
        if !parse_errors.is_empty() {
            parts.push(format!("{} parse error(s)", parse_errors.len()));
        }
        if !shadowed_rules.is_empty() {
            parts.push(format!("{} shadowed rule(s)", shadowed_rules.len()));
        }
        if !ambiguous_overlaps.is_empty() {
            parts.push(format!("{} ambiguous overlap(s)", ambiguous_overlaps.len()));
        }
        if !paradigm_gaps.is_empty() {
            let total_missing: usize = paradigm_gaps.iter().map(|g| g.missing_cells.len()).sum();
            parts.push(format!("{} missing paradigm cell(s)", total_missing));
        }
        let issue_count = parse_errors.len()
            + shadowed_rules.len()
            + ambiguous_overlaps.len()
            + paradigm_gaps.iter().map(|g| g.missing_cells.len()).sum::<usize>();
        format!("{} issue(s) in {} rules: {}", issue_count, parsed_count, parts.join(", "))
    };

    ReportSummary {
        clean,
        parse_error_count: parse_errors.len(),
        shadowed_count: shadowed_rules.len(),
        ambiguous_count: ambiguous_overlaps.len(),
        verdict,
    }
}

/// Check for shadowing/ambiguity in a group of rules that share the same match key.
/// `result_fn` extracts the "result" string from a rule for comparison.
fn check_overlap_group<T>(
    pattern: &str,
    group: &mut [&T],
    priority_fn: impl Fn(&T) -> u8,
    position_fn: impl Fn(&T) -> &str,
    result_fn: impl Fn(&T) -> &str,
    sutra_fn: impl Fn(&T) -> &str,
    type_fn: impl Fn(&T) -> &str,
    shadowed_rules: &mut Vec<ShadowedRule>,
    ambiguous_overlaps: &mut Vec<AmbiguousOverlap>,
) {
    if group.len() < 2 {
        return;
    }

    group.sort_by(|a, b| {
        let pa = priority_fn(a);
        let pb = priority_fn(b);
        pb.cmp(&pa).then_with(|| position_fn(b).cmp(position_fn(a)))
    });

    let winner = &group[0];
    for loser in &group[1..] {
        if result_fn(loser) == result_fn(winner) {
            shadowed_rules.push(ShadowedRule {
                shadowed_sutra: sutra_fn(loser).to_string(),
                shadowed_type: type_fn(loser).to_string(),
                shadowed_by_sutra: sutra_fn(winner).to_string(),
                shadowed_by_type: type_fn(winner).to_string(),
                pattern: pattern.to_string(),
            });
        } else {
            let wp = priority_fn(winner);
            let lp = priority_fn(loser);
            if wp == lp {
                ambiguous_overlaps.push(AmbiguousOverlap {
                    pattern: pattern.to_string(),
                    rule_a_sutra: sutra_fn(winner).to_string(),
                    rule_a_result: result_fn(winner).to_string(),
                    rule_a_type: type_fn(winner).to_string(),
                    rule_b_sutra: sutra_fn(loser).to_string(),
                    rule_b_result: result_fn(loser).to_string(),
                    rule_b_type: type_fn(loser).to_string(),
                });
            }
        }
    }
}

// --- Sandhi ---

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

pub fn check_sandhi_rules(rules: &[CachedRule]) -> CheckReport {
    let mut parse_errors = Vec::new();
    let mut parsed: Vec<SandhiParams> = Vec::new();

    for (i, rule) in rules.iter().enumerate() {
        match serde_json::from_value::<SandhiParams>(rule.params.clone()) {
            Ok(p) => parsed.push(p),
            Err(e) => parse_errors.push(RuleParseError {
                index: i,
                statement: rule.statement.clone(),
                error: e.to_string(),
            }),
        }
    }

    let mut by_pattern: BTreeMap<String, Vec<&SandhiParams>> = BTreeMap::new();
    for params in &parsed {
        if params.condition_pratyaya.is_some() {
            continue;
        }
        let key = format!("{} + {}", params.first, params.second);
        by_pattern.entry(key).or_default().push(params);
    }

    let mut shadowed_rules = Vec::new();
    let mut ambiguous_overlaps = Vec::new();

    for (pattern, mut group) in by_pattern {
        check_overlap_group(
            &pattern,
            &mut group,
            |p| rule_type_priority(&p.rule_type),
            |p| p.sutra_position.as_str(),
            |p| p.result.as_str(),
            |p| p.sutra.as_str(),
            |p| p.rule_type.as_str(),
            &mut shadowed_rules,
            &mut ambiguous_overlaps,
        );
    }

    let mut first_set = BTreeSet::new();
    let mut second_set = BTreeSet::new();
    let mut covered_pairs = BTreeSet::new();
    let mut rules_by_sutra: BTreeMap<String, usize> = BTreeMap::new();
    let mut rules_by_type: BTreeMap<String, usize> = BTreeMap::new();

    for params in &parsed {
        first_set.insert(params.first.clone());
        second_set.insert(params.second.clone());
        covered_pairs.insert((params.first.clone(), params.second.clone()));
        count_sutra(&params.sutra, &mut rules_by_sutra);
        count_type(&params.rule_type, &mut rules_by_type);
    }

    let coverage = CoverageSummary {
        dimensions: vec![
            CoverageDimension {
                label: "first phonemes".into(),
                values: first_set.into_iter().collect(),
            },
            CoverageDimension {
                label: "second phonemes".into(),
                values: second_set.into_iter().collect(),
            },
        ],
        covered_combinations: covered_pairs.len(),
        covered_combinations_label: "phoneme pairs".into(),
        rules_by_sutra,
        rules_by_type,
    };

    let extra = format!(
        "{} unique phoneme pairs covered across {} sūtras.",
        covered_pairs.len(),
        coverage.rules_by_sutra.len()
    );
    let summary = build_summary(
        parsed.len(),
        &parse_errors,
        &shadowed_rules,
        &ambiguous_overlaps,
        &[],
        &extra,
    );

    CheckReport {
        template: "sandhi_rule".into(),
        total_rules: rules.len(),
        parse_errors,
        shadowed_rules,
        ambiguous_overlaps,
        coverage,
        paradigm_gaps: Vec::new(),
        summary,
    }
}

// --- sup_suffix ---

#[derive(Debug, Deserialize)]
struct SupParams {
    stem_class: String,
    vibhakti: String,
    vacana: String,
    pratyaya: String,
    suffix: String,
    #[serde(default)]
    sutra: String,
    #[serde(default)]
    sutra_position: String,
}

const VIBHAKTIS: &[&str] = &[
    "prathama",
    "dvitiya",
    "tritiya",
    "caturthi",
    "pancami",
    "sasthi",
    "saptami",
    "sambodhana",
];
const VACANAS: &[&str] = &["ekavacana", "dvivacana", "bahuvacana"];

pub fn check_sup_suffix(rules: &[CachedRule]) -> CheckReport {
    let mut parse_errors = Vec::new();
    let mut parsed: Vec<SupParams> = Vec::new();

    for (i, rule) in rules.iter().enumerate() {
        match serde_json::from_value::<SupParams>(rule.params.clone()) {
            Ok(p) => parsed.push(p),
            Err(e) => parse_errors.push(RuleParseError {
                index: i,
                statement: rule.statement.clone(),
                error: e.to_string(),
            }),
        }
    }

    // Group by (stem_class, vibhakti, vacana) — first match wins in engine
    let mut by_key: BTreeMap<String, Vec<&SupParams>> = BTreeMap::new();
    for params in &parsed {
        let key = format!("{} / {} / {}", params.stem_class, params.vibhakti, params.vacana);
        by_key.entry(key).or_default().push(params);
    }

    let mut shadowed_rules = Vec::new();
    let mut ambiguous_overlaps = Vec::new();

    for (pattern, group) in &by_key {
        if group.len() < 2 {
            continue;
        }
        let winner = &group[0];
        for loser in &group[1..] {
            let winner_result = format!("{}/{}", winner.pratyaya, winner.suffix);
            let loser_result = format!("{}/{}", loser.pratyaya, loser.suffix);
            if winner_result == loser_result {
                shadowed_rules.push(ShadowedRule {
                    shadowed_sutra: loser.sutra.clone(),
                    shadowed_type: "duplicate".into(),
                    shadowed_by_sutra: winner.sutra.clone(),
                    shadowed_by_type: "first-match".into(),
                    pattern: pattern.clone(),
                });
            } else {
                ambiguous_overlaps.push(AmbiguousOverlap {
                    pattern: pattern.clone(),
                    rule_a_sutra: winner.sutra.clone(),
                    rule_a_result: winner_result,
                    rule_a_type: "first-match".into(),
                    rule_b_sutra: loser.sutra.clone(),
                    rule_b_result: loser_result,
                    rule_b_type: "shadowed".into(),
                });
            }
        }
    }

    // Paradigm completeness: for each stem_class, check all 24 cells
    let stem_classes: BTreeSet<&str> = parsed.iter().map(|p| p.stem_class.as_str()).collect();
    let mut paradigm_gaps = Vec::new();

    for sc in &stem_classes {
        let present: BTreeSet<(&str, &str)> = parsed
            .iter()
            .filter(|p| p.stem_class == *sc)
            .map(|p| (p.vibhakti.as_str(), p.vacana.as_str()))
            .collect();

        let mut missing = Vec::new();
        for vib in VIBHAKTIS {
            for vac in VACANAS {
                if !present.contains(&(*vib, *vac)) {
                    missing.push(format!("{vib} {vac}"));
                }
            }
        }

        if !missing.is_empty() {
            paradigm_gaps.push(ParadigmGap {
                stem_class: sc.to_string(),
                missing_cells: missing,
                present: present.len(),
                expected: 24,
            });
        }
    }

    let mut rules_by_sutra: BTreeMap<String, usize> = BTreeMap::new();
    let mut rules_by_type: BTreeMap<String, usize> = BTreeMap::new();

    for params in &parsed {
        count_sutra(&params.sutra, &mut rules_by_sutra);
        *rules_by_type.entry("suffix-lookup".into()).or_default() += 1;
    }

    let coverage = CoverageSummary {
        dimensions: vec![
            CoverageDimension {
                label: "stem classes".into(),
                values: stem_classes.iter().map(|s| s.to_string()).collect(),
            },
            CoverageDimension {
                label: "vibhaktis".into(),
                values: parsed
                    .iter()
                    .map(|p| p.vibhakti.clone())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect(),
            },
            CoverageDimension {
                label: "vacanas".into(),
                values: parsed
                    .iter()
                    .map(|p| p.vacana.clone())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect(),
            },
        ],
        covered_combinations: by_key.len(),
        covered_combinations_label: "paradigm cells".into(),
        rules_by_sutra,
        rules_by_type,
    };

    let extra = format!(
        "{} stem class(es), {} of {} paradigm cells filled.",
        stem_classes.len(),
        by_key.len(),
        stem_classes.len() * 24
    );
    let summary = build_summary(
        parsed.len(),
        &parse_errors,
        &shadowed_rules,
        &ambiguous_overlaps,
        &paradigm_gaps,
        &extra,
    );

    CheckReport {
        template: "sup_suffix".into(),
        total_rules: rules.len(),
        parse_errors,
        shadowed_rules,
        ambiguous_overlaps,
        coverage,
        paradigm_gaps,
        summary,
    }
}

// --- pratyaya_rule ---

#[derive(Debug, Deserialize)]
struct PratyayaParams {
    condition_stem_class: String,
    #[serde(default)]
    condition_suffix: String,
    input_suffix: String,
    output_suffix: String,
    #[serde(default)]
    condition_vibhakti: Option<String>,
    #[serde(default)]
    sutra: String,
    #[serde(default)]
    sutra_position: String,
    #[serde(default)]
    rule_type: String,
}

pub fn check_pratyaya_rules(rules: &[CachedRule]) -> CheckReport {
    let mut parse_errors = Vec::new();
    let mut parsed: Vec<PratyayaParams> = Vec::new();

    for (i, rule) in rules.iter().enumerate() {
        match serde_json::from_value::<PratyayaParams>(rule.params.clone()) {
            Ok(p) => parsed.push(p),
            Err(e) => parse_errors.push(RuleParseError {
                index: i,
                statement: rule.statement.clone(),
                error: e.to_string(),
            }),
        }
    }

    // Key: (stem_class, condition_suffix, input_suffix, condition_vibhakti)
    let mut by_pattern: BTreeMap<String, Vec<&PratyayaParams>> = BTreeMap::new();
    for params in &parsed {
        let vib = params
            .condition_vibhakti
            .as_deref()
            .unwrap_or("*");
        let key = format!(
            "{} / {} / {} [{}]",
            params.condition_stem_class, params.condition_suffix, params.input_suffix, vib
        );
        by_pattern.entry(key).or_default().push(params);
    }

    let mut shadowed_rules = Vec::new();
    let mut ambiguous_overlaps = Vec::new();

    for (pattern, mut group) in by_pattern {
        check_overlap_group(
            &pattern,
            &mut group,
            |p| rule_type_priority(&p.rule_type),
            |p| p.sutra_position.as_str(),
            |p| p.output_suffix.as_str(),
            |p| p.sutra.as_str(),
            |p| p.rule_type.as_str(),
            &mut shadowed_rules,
            &mut ambiguous_overlaps,
        );
    }

    let mut rules_by_sutra: BTreeMap<String, usize> = BTreeMap::new();
    let mut rules_by_type: BTreeMap<String, usize> = BTreeMap::new();
    let mut stem_classes = BTreeSet::new();
    let mut suffixes = BTreeSet::new();

    for params in &parsed {
        count_sutra(&params.sutra, &mut rules_by_sutra);
        count_type(&params.rule_type, &mut rules_by_type);
        stem_classes.insert(params.condition_stem_class.clone());
        suffixes.insert(params.condition_suffix.clone());
    }

    let coverage = CoverageSummary {
        dimensions: vec![
            CoverageDimension {
                label: "stem classes".into(),
                values: stem_classes.into_iter().collect(),
            },
            CoverageDimension {
                label: "pratyayas".into(),
                values: suffixes.into_iter().collect(),
            },
        ],
        covered_combinations: parsed.len(),
        covered_combinations_label: "modification rules".into(),
        rules_by_sutra,
        rules_by_type,
    };

    let extra = format!(
        "{} modification rules across {} sūtras.",
        parsed.len(),
        coverage.rules_by_sutra.len()
    );
    let summary = build_summary(
        parsed.len(),
        &parse_errors,
        &shadowed_rules,
        &ambiguous_overlaps,
        &[],
        &extra,
    );

    CheckReport {
        template: "pratyaya_rule".into(),
        total_rules: rules.len(),
        parse_errors,
        shadowed_rules,
        ambiguous_overlaps,
        coverage,
        paradigm_gaps: Vec::new(),
        summary,
    }
}

// --- anga_rule ---

#[derive(Debug, Deserialize)]
struct AngaParams {
    condition_stem_final: String,
    #[serde(default)]
    condition_suffix_initial: Option<String>,
    #[serde(default)]
    condition_vacana: Option<String>,
    #[serde(default)]
    condition_vibhakti: Option<String>,
    operation_input: String,
    operation_output: String,
    #[serde(default)]
    sutra: String,
    #[serde(default)]
    sutra_position: String,
    #[serde(default)]
    rule_type: String,
}

pub fn check_anga_rules(rules: &[CachedRule]) -> CheckReport {
    let mut parse_errors = Vec::new();
    let mut parsed: Vec<AngaParams> = Vec::new();

    for (i, rule) in rules.iter().enumerate() {
        match serde_json::from_value::<AngaParams>(rule.params.clone()) {
            Ok(p) => parsed.push(p),
            Err(e) => parse_errors.push(RuleParseError {
                index: i,
                statement: rule.statement.clone(),
                error: e.to_string(),
            }),
        }
    }

    // Key: (stem_final, suffix_initial, vacana, vibhakti, operation_input)
    let mut by_pattern: BTreeMap<String, Vec<&AngaParams>> = BTreeMap::new();
    for params in &parsed {
        let si = params.condition_suffix_initial.as_deref().unwrap_or("*");
        let vac = params.condition_vacana.as_deref().unwrap_or("*");
        let vib = params.condition_vibhakti.as_deref().unwrap_or("*");
        let key = format!(
            "stem={} suffix={} vacana={} vibhakti={} input={}",
            params.condition_stem_final, si, vac, vib, params.operation_input
        );
        by_pattern.entry(key).or_default().push(params);
    }

    let mut shadowed_rules = Vec::new();
    let mut ambiguous_overlaps = Vec::new();

    for (pattern, mut group) in by_pattern {
        check_overlap_group(
            &pattern,
            &mut group,
            |p| rule_type_priority(&p.rule_type),
            |p| p.sutra_position.as_str(),
            |p| p.operation_output.as_str(),
            |p| p.sutra.as_str(),
            |p| p.rule_type.as_str(),
            &mut shadowed_rules,
            &mut ambiguous_overlaps,
        );
    }

    let mut rules_by_sutra: BTreeMap<String, usize> = BTreeMap::new();
    let mut rules_by_type: BTreeMap<String, usize> = BTreeMap::new();
    let mut stem_finals = BTreeSet::new();
    let mut suffix_initials = BTreeSet::new();

    for params in &parsed {
        count_sutra(&params.sutra, &mut rules_by_sutra);
        count_type(&params.rule_type, &mut rules_by_type);
        stem_finals.insert(params.condition_stem_final.clone());
        if let Some(ref si) = params.condition_suffix_initial {
            suffix_initials.insert(si.clone());
        }
    }

    let coverage = CoverageSummary {
        dimensions: vec![
            CoverageDimension {
                label: "stem-final phonemes".into(),
                values: stem_finals.into_iter().collect(),
            },
            CoverageDimension {
                label: "suffix-initial phonemes".into(),
                values: suffix_initials.into_iter().collect(),
            },
        ],
        covered_combinations: parsed.len(),
        covered_combinations_label: "stem modification rules".into(),
        rules_by_sutra,
        rules_by_type,
    };

    let extra = format!(
        "{} stem modification rules across {} sūtras.",
        parsed.len(),
        coverage.rules_by_sutra.len()
    );
    let summary = build_summary(
        parsed.len(),
        &parse_errors,
        &shadowed_rules,
        &ambiguous_overlaps,
        &[],
        &extra,
    );

    CheckReport {
        template: "anga_rule".into(),
        total_rules: rules.len(),
        parse_errors,
        shadowed_rules,
        ambiguous_overlaps,
        coverage,
        paradigm_gaps: Vec::new(),
        summary,
    }
}

// --- tripadi_rule ---

#[derive(Debug, Deserialize)]
struct TripadiParams {
    #[serde(default)]
    condition_preceding: Option<String>,
    input: String,
    output: String,
    position: String,
    #[serde(default)]
    sutra: String,
    #[serde(default)]
    sutra_position: String,
    #[serde(default)]
    rule_type: String,
}

pub fn check_tripadi_rules(rules: &[CachedRule]) -> CheckReport {
    let mut parse_errors = Vec::new();
    let mut parsed: Vec<TripadiParams> = Vec::new();

    for (i, rule) in rules.iter().enumerate() {
        match serde_json::from_value::<TripadiParams>(rule.params.clone()) {
            Ok(p) => parsed.push(p),
            Err(e) => parse_errors.push(RuleParseError {
                index: i,
                statement: rule.statement.clone(),
                error: e.to_string(),
            }),
        }
    }

    // Key: (position, input, condition_preceding)
    let mut by_pattern: BTreeMap<String, Vec<&TripadiParams>> = BTreeMap::new();
    for params in &parsed {
        let prec = params.condition_preceding.as_deref().unwrap_or("*");
        let key = format!("{} / {} [prec={}]", params.position, params.input, prec);
        by_pattern.entry(key).or_default().push(params);
    }

    let mut shadowed_rules = Vec::new();
    let mut ambiguous_overlaps = Vec::new();

    for (pattern, mut group) in by_pattern {
        check_overlap_group(
            &pattern,
            &mut group,
            |p| rule_type_priority(&p.rule_type),
            |p| p.sutra_position.as_str(),
            |p| p.output.as_str(),
            |p| p.sutra.as_str(),
            |p| p.rule_type.as_str(),
            &mut shadowed_rules,
            &mut ambiguous_overlaps,
        );
    }

    let mut rules_by_sutra: BTreeMap<String, usize> = BTreeMap::new();
    let mut rules_by_type: BTreeMap<String, usize> = BTreeMap::new();
    let mut positions = BTreeSet::new();

    for params in &parsed {
        count_sutra(&params.sutra, &mut rules_by_sutra);
        count_type(&params.rule_type, &mut rules_by_type);
        positions.insert(params.position.clone());
    }

    let coverage = CoverageSummary {
        dimensions: vec![CoverageDimension {
            label: "positions".into(),
            values: positions.into_iter().collect(),
        }],
        covered_combinations: parsed.len(),
        covered_combinations_label: "tripādī rules".into(),
        rules_by_sutra,
        rules_by_type,
    };

    let extra = format!(
        "{} tripādī rules across {} sūtras.",
        parsed.len(),
        coverage.rules_by_sutra.len()
    );
    let summary = build_summary(
        parsed.len(),
        &parse_errors,
        &shadowed_rules,
        &ambiguous_overlaps,
        &[],
        &extra,
    );

    CheckReport {
        template: "tripadi_rule".into(),
        total_rules: rules.len(),
        parse_errors,
        shadowed_rules,
        ambiguous_overlaps,
        coverage,
        paradigm_gaps: Vec::new(),
        summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rule_cache::CachedRule;

    fn make(params: serde_json::Value) -> CachedRule {
        let stmt = params
            .get("sutra")
            .and_then(|v| v.as_str())
            .unwrap_or("test")
            .to_string();
        CachedRule {
            statement: stmt,
            params,
        }
    }

    // --- sandhi ---

    #[test]
    fn sandhi_clean() {
        let rules = vec![
            make(serde_json::json!({
                "first": "a", "second": "a", "result": "ā",
                "sutra": "6.1.101", "sutra_position": "06.01.101", "rule_type": "utsarga"
            })),
            make(serde_json::json!({
                "first": "a", "second": "i", "result": "e",
                "sutra": "6.1.87", "sutra_position": "06.01.087", "rule_type": "utsarga"
            })),
        ];
        let report = check_sandhi_rules(&rules);
        assert!(report.summary.clean);
        assert_eq!(report.coverage.covered_combinations, 2);
    }

    #[test]
    fn sandhi_shadowed() {
        let rules = vec![
            make(serde_json::json!({
                "first": "a", "second": "a", "result": "ā",
                "sutra": "6.1.101", "sutra_position": "06.01.101", "rule_type": "utsarga"
            })),
            make(serde_json::json!({
                "first": "a", "second": "a", "result": "ā",
                "sutra": "6.1.101-dup", "sutra_position": "06.01.101", "rule_type": "utsarga"
            })),
        ];
        let report = check_sandhi_rules(&rules);
        assert!(!report.summary.clean);
        assert_eq!(report.shadowed_rules.len(), 1);
    }

    #[test]
    fn sandhi_ambiguous() {
        let rules = vec![
            make(serde_json::json!({
                "first": "a", "second": "a", "result": "ā",
                "sutra": "6.1.101", "sutra_position": "06.01.101", "rule_type": "utsarga"
            })),
            make(serde_json::json!({
                "first": "a", "second": "a", "result": "a",
                "sutra": "6.1.102", "sutra_position": "06.01.102", "rule_type": "utsarga"
            })),
        ];
        let report = check_sandhi_rules(&rules);
        assert_eq!(report.ambiguous_overlaps.len(), 1);
    }

    #[test]
    fn sandhi_apavada_ok() {
        let rules = vec![
            make(serde_json::json!({
                "first": "a", "second": "a", "result": "ā",
                "sutra": "6.1.101", "sutra_position": "06.01.101", "rule_type": "utsarga"
            })),
            make(serde_json::json!({
                "first": "a", "second": "a", "result": "a",
                "sutra": "6.1.107", "sutra_position": "06.01.107", "rule_type": "apavāda"
            })),
        ];
        assert!(check_sandhi_rules(&rules).summary.clean);
    }

    #[test]
    fn sandhi_parse_error() {
        let rules = vec![CachedRule {
            statement: "broken".into(),
            params: serde_json::json!({"bogus": true}),
        }];
        let report = check_sandhi_rules(&rules);
        assert_eq!(report.parse_errors.len(), 1);
    }

    #[test]
    fn sandhi_pratyaya_excluded() {
        let rules = vec![
            make(serde_json::json!({
                "first": "a", "second": "a", "result": "ā",
                "sutra": "6.1.101", "sutra_position": "06.01.101", "rule_type": "utsarga"
            })),
            make(serde_json::json!({
                "first": "a", "second": "a", "result": "a",
                "sutra": "6.1.107", "sutra_position": "06.01.107",
                "rule_type": "apavāda", "condition_pratyaya": "am"
            })),
        ];
        assert!(check_sandhi_rules(&rules).summary.clean);
    }

    // --- sup_suffix ---

    #[test]
    fn sup_complete_paradigm_clean() {
        let rules: Vec<CachedRule> = VIBHAKTIS
            .iter()
            .flat_map(|v| {
                VACANAS.iter().map(move |n| {
                    make(serde_json::json!({
                        "stem_class": "a-stem-m", "vibhakti": v, "vacana": n,
                        "pratyaya": "test", "suffix": "x", "sutra": "4.1.2"
                    }))
                })
            })
            .collect();
        let report = check_sup_suffix(&rules);
        assert!(report.summary.clean);
        assert!(report.paradigm_gaps.is_empty());
        assert_eq!(report.coverage.covered_combinations, 24);
    }

    #[test]
    fn sup_detects_missing_cells() {
        let rules = vec![make(serde_json::json!({
            "stem_class": "a-stem-m", "vibhakti": "prathama", "vacana": "ekavacana",
            "pratyaya": "su", "suffix": "s", "sutra": "4.1.2"
        }))];
        let report = check_sup_suffix(&rules);
        assert!(!report.summary.clean);
        assert_eq!(report.paradigm_gaps.len(), 1);
        assert_eq!(report.paradigm_gaps[0].present, 1);
        assert_eq!(report.paradigm_gaps[0].expected, 24);
        assert_eq!(report.paradigm_gaps[0].missing_cells.len(), 23);
    }

    #[test]
    fn sup_duplicate_detected() {
        let rules = vec![
            make(serde_json::json!({
                "stem_class": "a-stem-m", "vibhakti": "prathama", "vacana": "ekavacana",
                "pratyaya": "su", "suffix": "s", "sutra": "4.1.2"
            })),
            make(serde_json::json!({
                "stem_class": "a-stem-m", "vibhakti": "prathama", "vacana": "ekavacana",
                "pratyaya": "su", "suffix": "s", "sutra": "4.1.2-dup"
            })),
        ];
        let report = check_sup_suffix(&rules);
        assert_eq!(report.shadowed_rules.len(), 1);
    }

    // --- pratyaya_rule ---

    #[test]
    fn pratyaya_clean() {
        let rules = vec![
            make(serde_json::json!({
                "condition_stem_class": "a-stem-m", "condition_suffix": "bhis",
                "input_suffix": "bhis", "output_suffix": "ais",
                "sutra": "7.1.9", "sutra_position": "07.01.009", "rule_type": "nitya"
            })),
            make(serde_json::json!({
                "condition_stem_class": "a-stem-m", "condition_suffix": "ṭā",
                "input_suffix": "ā", "output_suffix": "ina",
                "sutra": "7.1.12", "sutra_position": "07.01.012", "rule_type": "nitya"
            })),
        ];
        assert!(check_pratyaya_rules(&rules).summary.clean);
    }

    #[test]
    fn pratyaya_ambiguous() {
        let rules = vec![
            make(serde_json::json!({
                "condition_stem_class": "a-stem-m", "condition_suffix": "bhis",
                "input_suffix": "bhis", "output_suffix": "ais",
                "sutra": "7.1.9", "sutra_position": "07.01.009", "rule_type": "nitya"
            })),
            make(serde_json::json!({
                "condition_stem_class": "a-stem-m", "condition_suffix": "bhis",
                "input_suffix": "bhis", "output_suffix": "WRONG",
                "sutra": "7.1.99", "sutra_position": "07.01.099", "rule_type": "nitya"
            })),
        ];
        let report = check_pratyaya_rules(&rules);
        assert_eq!(report.ambiguous_overlaps.len(), 1);
    }

    // --- anga_rule ---

    #[test]
    fn anga_clean() {
        let rules = vec![make(serde_json::json!({
            "condition_stem_final": "a", "condition_suffix_initial": "y",
            "operation_input": "a", "operation_output": "ā",
            "sutra": "7.3.101", "sutra_position": "07.03.101", "rule_type": "utsarga"
        }))];
        assert!(check_anga_rules(&rules).summary.clean);
    }

    #[test]
    fn anga_parse_error() {
        let rules = vec![CachedRule {
            statement: "broken".into(),
            params: serde_json::json!({"bad": true}),
        }];
        assert_eq!(check_anga_rules(&rules).parse_errors.len(), 1);
    }

    // --- tripadi_rule ---

    #[test]
    fn tripadi_clean() {
        let rules = vec![
            make(serde_json::json!({
                "input": "s", "output": "r", "position": "word_final",
                "sutra": "8.2.66", "sutra_position": "08.02.066", "rule_type": "nitya"
            })),
            make(serde_json::json!({
                "input": "r", "output": "ḥ", "position": "word_final",
                "sutra": "8.3.15", "sutra_position": "08.03.015", "rule_type": "nitya"
            })),
        ];
        assert!(check_tripadi_rules(&rules).summary.clean);
    }

    #[test]
    fn tripadi_shadowed() {
        let rules = vec![
            make(serde_json::json!({
                "input": "s", "output": "r", "position": "word_final",
                "sutra": "8.2.66", "sutra_position": "08.02.066", "rule_type": "nitya"
            })),
            make(serde_json::json!({
                "input": "s", "output": "r", "position": "word_final",
                "sutra": "8.2.66-dup", "sutra_position": "08.02.066", "rule_type": "nitya"
            })),
        ];
        assert_eq!(check_tripadi_rules(&rules).shadowed_rules.len(), 1);
    }
}
