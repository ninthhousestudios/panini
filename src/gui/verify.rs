use std::sync::Arc;

use iced::widget::{scrollable, text, Column};
use iced::{Element, Task};

use crate::engine::consistency::{
    CheckReport, check_anga_rules, check_pratyaya_rules, check_sandhi_rules, check_sup_suffix,
    check_tripadi_rules,
};
use crate::rule_cache::RuleCache;

use super::theme;

#[derive(Default)]
pub struct State {
    pub loaded: bool,
    pub loading: bool,
    pub reports: Vec<CheckReport>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Loaded(Vec<CheckReport>),
}

pub fn load(cache: &Arc<RuleCache>) -> Task<Message> {
    let cache = cache.clone();
    Task::perform(
        async move { run_checks(&cache) },
        Message::Loaded,
    )
}

fn run_checks(cache: &RuleCache) -> Vec<CheckReport> {
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
    reports
}

pub fn update(state: &mut State, msg: Message) -> Task<Message> {
    match msg {
        Message::Loaded(reports) => {
            state.reports = reports;
            state.loading = false;
            state.loaded = true;
            Task::none()
        }
    }
}

pub fn view(state: &State) -> Element<'_, Message> {
    if state.loading {
        return text("Running consistency checks…").size(16).into();
    }
    if !state.loaded {
        return text("Loading…").size(16).into();
    }

    let mut cards = Column::new().spacing(16);

    for report in &state.reports {
        cards = cards.push(report_card(report));
    }

    scrollable(cards).height(iced::Fill).into()
}

fn report_card(report: &CheckReport) -> Element<'_, Message> {
    let verdict_text = if report.summary.clean { "clean" } else { "issues found" };

    let mut card = Column::new().spacing(4);
    card = card.push(
        text(format!(
            "{} — {} rules — {}",
            report.template, report.total_rules, verdict_text
        ))
        .size(16)
        .font(theme::latin()),
    );

    if !report.summary.verdict.is_empty() {
        card = card.push(text(report.summary.verdict.clone()).size(13));
    }

    let cov = &report.coverage;
    if !cov.covered_combinations_label.is_empty() {
        card = card.push(
            text(format!("{}: {}", cov.covered_combinations_label, cov.covered_combinations))
                .size(12),
        );
    }

    if !cov.rules_by_type.is_empty() {
        let types: Vec<String> = cov
            .rules_by_type
            .iter()
            .map(|(k, v)| format!("{k}: {v}"))
            .collect();
        card = card.push(text(types.join("  |  ")).size(12));
    }

    if !report.parse_errors.is_empty() {
        card = card.push(
            text(format!("{} parse errors", report.parse_errors.len())).size(13),
        );
    }

    if !report.shadowed_rules.is_empty() {
        card = card.push(
            text(format!("{} shadowed rules", report.shadowed_rules.len())).size(13),
        );
    }

    if !report.ambiguous_overlaps.is_empty() {
        card = card.push(
            text(format!("{} ambiguous overlaps", report.ambiguous_overlaps.len())).size(13),
        );
    }

    card.into()
}
