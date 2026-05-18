use std::sync::Arc;

use iced::widget::{column, container, scrollable, text, Column, Row};
use iced::{Element, Fill, Task};

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
    Task::perform(async move { run_checks(&cache) }, Message::Loaded)
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
        return text("Running consistency checks…")
            .size(14)
            .color(theme::TEXT_SECONDARY)
            .into();
    }
    if !state.loaded {
        return text("Switch to this tab to run checks.")
            .size(14)
            .color(theme::TEXT_SECONDARY)
            .into();
    }

    let intro = text(
        "Automated consistency checks against the loaded rule set. \
         No shadowed rules, no ambiguous overlaps, full coverage reporting.",
    )
    .size(13)
    .color(theme::TEXT_SECONDARY);

    let mut cards = Column::new().spacing(20);
    for report in &state.reports {
        cards = cards.push(report_card(report));
    }

    scrollable(column![intro, cards].spacing(16))
        .height(Fill)
        .into()
}

fn report_card(report: &CheckReport) -> Element<'_, Message> {
    let heading = text(format!(
        "{} — {} rules",
        report.template, report.total_rules
    ))
    .size(17)
    .color(theme::ACCENT)
    .font(theme::latin());

    let verdict_style = if report.summary.clean {
        theme::verdict_clean
    } else {
        theme::verdict_issues
    };
    let verdict_color = if report.summary.clean {
        theme::CLEAN_TEXT
    } else {
        theme::ERROR_TEXT
    };
    let verdict = container(
        text(report.summary.verdict.clone())
            .size(14)
            .color(verdict_color),
    )
    .style(verdict_style)
    .padding([12, 16])
    .width(Fill);

    let mut sections = Column::new().spacing(12);
    sections = sections.push(heading);
    sections = sections.push(verdict);

    let cov = &report.coverage;
    if !cov.covered_combinations_label.is_empty() {
        let mut stats_row = Row::new().spacing(12);
        stats_row = stats_row.push(stat_badge(
            &cov.covered_combinations.to_string(),
            &cov.covered_combinations_label,
        ));
        stats_row = stats_row.push(stat_badge(
            &cov.rules_by_sutra.len().to_string(),
            "distinct sūtras",
        ));
        for dim in &cov.dimensions {
            stats_row = stats_row.push(stat_badge(
                &dim.values.len().to_string(),
                &dim.label,
            ));
        }
        sections = sections.push(stats_row);
    }

    if !cov.rules_by_type.is_empty() {
        let mut type_row = Row::new().spacing(6);
        for (k, v) in &cov.rules_by_type {
            type_row = type_row.push(
                container(
                    text(format!("{k}: {v}"))
                        .size(12)
                        .font(theme::latin())
                        .color(theme::ACCENT),
                )
                .style(theme::tag_style)
                .padding([4, 8]),
            );
        }
        sections = sections.push(type_row);
    }

    if !report.parse_errors.is_empty() {
        let mut errs = Column::new().spacing(4);
        errs = errs.push(
            text(format!("Parse Errors ({})", report.parse_errors.len()))
                .size(14)
                .font(theme::latin()),
        );
        for err in &report.parse_errors {
            errs = errs.push(
                container(
                    column![
                        text(format!("Rule #{}: {}", err.index, err.statement))
                            .size(13)
                            .color(theme::ACCENT),
                        text(err.error.clone())
                            .size(12)
                            .color(theme::TEXT_SECONDARY),
                    ]
                    .spacing(2),
                )
                .style(theme::card)
                .padding([8, 12]),
            );
        }
        sections = sections.push(errs);
    }

    if !report.shadowed_rules.is_empty() {
        let mut sh = Column::new().spacing(4);
        sh = sh.push(
            text(format!("Shadowed Rules ({})", report.shadowed_rules.len()))
                .size(14)
                .font(theme::latin()),
        );
        for s in &report.shadowed_rules {
            sh = sh.push(
                container(
                    column![
                        text(s.pattern.clone()).size(13).color(theme::ACCENT),
                        text(format!(
                            "{} ({}) shadowed by {} ({})",
                            s.shadowed_sutra,
                            s.shadowed_type,
                            s.shadowed_by_sutra,
                            s.shadowed_by_type
                        ))
                        .size(12)
                        .color(theme::TEXT_SECONDARY),
                    ]
                    .spacing(2),
                )
                .style(theme::card)
                .padding([8, 12]),
            );
        }
        sections = sections.push(sh);
    }

    if !report.ambiguous_overlaps.is_empty() {
        let mut ao = Column::new().spacing(4);
        ao = ao.push(
            text(format!(
                "Ambiguous Overlaps ({})",
                report.ambiguous_overlaps.len()
            ))
            .size(14)
            .font(theme::latin()),
        );
        for o in &report.ambiguous_overlaps {
            ao = ao.push(
                container(
                    column![
                        text(o.pattern.clone()).size(13).color(theme::ACCENT),
                        text(format!(
                            "{} → {} vs {} → {} (both {})",
                            o.rule_a_sutra,
                            o.rule_a_result,
                            o.rule_b_sutra,
                            o.rule_b_result,
                            o.rule_a_type
                        ))
                        .size(12)
                        .color(theme::TEXT_SECONDARY),
                    ]
                    .spacing(2),
                )
                .style(theme::card)
                .padding([8, 12]),
            );
        }
        sections = sections.push(ao);
    }

    container(sections)
        .style(theme::card)
        .padding(20)
        .width(Fill)
        .into()
}

fn stat_badge<'a>(value: &str, label: &str) -> Element<'a, Message> {
    container(
        column![
            text(value.to_string())
                .size(22)
                .color(theme::ACCENT)
                .center(),
            text(label.to_uppercase())
                .size(10)
                .font(theme::latin())
                .color(theme::TEXT_SECONDARY)
                .center(),
        ]
        .spacing(2)
        .align_x(iced::Center),
    )
    .style(theme::card)
    .padding([12, 16])
    .into()
}
