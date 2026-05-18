use std::collections::BTreeMap;
use std::sync::Arc;

use iced::widget::{column, container, row, scrollable, text, text_input, Column, Row};
use iced::{Element, Fill, Task};

use crate::api::SutraEntry;
use crate::rule_cache::RuleCache;

use super::theme;

#[derive(Default)]
pub struct State {
    pub loaded: bool,
    pub all_sutras: Vec<SutraEntry>,
    pub filter: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    Loaded(Vec<SutraEntry>),
    FilterChanged(String),
}

pub fn load(cache: &Arc<RuleCache>) -> Task<Message> {
    let cache = cache.clone();
    Task::perform(async move { collect_sutras(&cache) }, Message::Loaded)
}

fn collect_sutras(cache: &RuleCache) -> Vec<SutraEntry> {
    let mut by_sutra: BTreeMap<String, (String, Vec<String>)> = BTreeMap::new();
    for (template, rules) in cache.all_templates() {
        for rule in rules {
            let sutra = rule
                .params
                .get("sutra")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if sutra.is_empty() {
                continue;
            }
            let entry = by_sutra
                .entry(sutra.to_string())
                .or_insert_with(|| (rule.statement.clone(), Vec::new()));
            if !entry.1.contains(&template.to_string()) {
                entry.1.push(template.to_string());
            }
        }
    }
    by_sutra
        .into_iter()
        .map(|(sutra, (statement, templates))| SutraEntry {
            sutra,
            statement,
            templates,
        })
        .collect()
}

pub fn update(state: &mut State, msg: Message) -> Task<Message> {
    match msg {
        Message::Loaded(entries) => {
            state.all_sutras = entries;
            state.loaded = true;
            Task::none()
        }
        Message::FilterChanged(v) => {
            state.filter = v;
            Task::none()
        }
    }
}

pub fn view(state: &State) -> Element<'_, Message> {
    if !state.loaded {
        return text("Loading sūtras…")
            .size(14)
            .color(theme::TEXT_SECONDARY)
            .into();
    }

    let filter_input = text_input("Search sūtras by number or text…", &state.filter)
        .on_input(Message::FilterChanged)
        .style(theme::input_style)
        .width(400);

    let q = state.filter.to_lowercase();
    let filtered: Vec<_> = state
        .all_sutras
        .iter()
        .filter(|e| {
            q.is_empty()
                || e.sutra.contains(&q)
                || e.statement.to_lowercase().contains(&q)
                || e.templates.iter().any(|t| t.to_lowercase().contains(&q))
        })
        .collect();

    let count = text(format!(
        "{} sūtra{}",
        filtered.len(),
        if filtered.len() != 1 { "s" } else { "" }
    ))
    .size(14)
    .color(theme::TEXT_SECONDARY);

    let mut list = Column::new().spacing(6);
    for entry in &filtered {
        let mut tags_row = Row::new().spacing(4);
        for t in &entry.templates {
            tags_row = tags_row.push(
                container(
                    text(t.clone())
                        .size(11)
                        .font(theme::latin())
                        .color(theme::ACCENT),
                )
                .style(theme::tag_style)
                .padding([2, 6]),
            );
        }

        let card = container(
            row![
                text(entry.sutra.clone())
                    .size(14)
                    .font(theme::latin())
                    .color(theme::ACCENT)
                    .width(80),
                text(entry.statement.clone())
                    .size(14)
                    .font(theme::latin())
                    .width(Fill),
                tags_row,
            ]
            .spacing(12)
            .align_y(iced::Center),
        )
        .style(theme::card)
        .padding([10, 14]);
        list = list.push(card);
    }

    column![filter_input, count, scrollable(list).height(Fill)]
        .spacing(10)
        .into()
}
