use std::collections::BTreeMap;
use std::sync::Arc;

use iced::widget::{column, scrollable, text, text_input, Column};
use iced::{Element, Task};

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
    Task::perform(
        async move { collect_sutras(&cache) },
        Message::Loaded,
    )
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
        return text("Loading sūtras…").size(16).into();
    }

    let filter_input = text_input("Search sūtras…", &state.filter)
        .on_input(Message::FilterChanged)
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

    let count = text(format!("{} sūtras", filtered.len())).size(14);

    let mut list = Column::new().spacing(4);
    for entry in &filtered {
        let tags = entry.templates.join(", ");
        let card = column![
            text(format!("{}  —  {}", entry.sutra, entry.statement))
                .size(15)
                .font(theme::latin()),
            text(tags).size(12),
        ]
        .spacing(2);
        list = list.push(card);
    }

    column![filter_input, count, scrollable(list).height(iced::Fill)]
        .spacing(8)
        .into()
}
