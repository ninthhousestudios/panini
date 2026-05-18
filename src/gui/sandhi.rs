use std::sync::Arc;

use iced::widget::{button, column, container, row, scrollable, text, text_input, Column};
use iced::{Center, Element, Task};

use crate::engine::sandhi::{SandhiInput, analyze_sandhi, derive_sandhi};
use crate::engine::script::to_devanagari;
use crate::engine::{AnalyzeCandidate, TraceStep};
use crate::rule_cache::RuleCache;

use super::theme;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum Mode {
    #[default]
    Forward,
    Reverse,
}

#[derive(Default)]
pub struct State {
    pub mode: Mode,
    pub first: String,
    pub second: String,
    pub join_result: Option<JoinResult>,
    pub join_trace: Option<Vec<TraceStep>>,
    pub show_trace: bool,
    pub form: String,
    pub candidates: Vec<AnalyzeCandidate>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct JoinResult {
    pub form: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    SetMode(Mode),
    FirstChanged(String),
    SecondChanged(String),
    FormChanged(String),
    Join,
    Analyze,
    JoinDone(Result<(JoinResult, Vec<TraceStep>), String>),
    AnalyzeDone(Result<Vec<AnalyzeCandidate>, String>),
    ToggleTrace,
}

pub fn update(cache: &Arc<RuleCache>, state: &mut State, msg: Message) -> Task<Message> {
    match msg {
        Message::SetMode(m) => {
            state.mode = m;
            state.error = None;
            Task::none()
        }
        Message::FirstChanged(v) => {
            state.first = v;
            Task::none()
        }
        Message::SecondChanged(v) => {
            state.second = v;
            Task::none()
        }
        Message::FormChanged(v) => {
            state.form = v;
            Task::none()
        }
        Message::Join => {
            if state.first.is_empty() || state.second.is_empty() {
                return Task::none();
            }
            let cache = cache.clone();
            let first = state.first.clone();
            let second = state.second.clone();
            Task::perform(
                async move {
                    let rules = cache.get_rules("sandhi_rule");
                    let input = SandhiInput { first, second };
                    match derive_sandhi(rules, input) {
                        Ok(result) => {
                            let form = result.output["form"]
                                .as_str()
                                .or_else(|| result.output["result"].as_str())
                                .unwrap_or("")
                                .to_string();
                            Ok((JoinResult { form }, result.trace))
                        }
                        Err(e) => Err(e.to_string()),
                    }
                },
                Message::JoinDone,
            )
        }
        Message::Analyze => {
            if state.form.is_empty() {
                return Task::none();
            }
            let cache = cache.clone();
            let form = state.form.clone();
            Task::perform(
                async move {
                    let rules = cache.get_rules("sandhi_rule");
                    match analyze_sandhi(rules, &form) {
                        Ok(result) => Ok(result.candidates),
                        Err(e) => Err(e.to_string()),
                    }
                },
                Message::AnalyzeDone,
            )
        }
        Message::JoinDone(result) => {
            match result {
                Ok((jr, trace)) => {
                    state.join_result = Some(jr);
                    state.join_trace = Some(trace);
                    state.error = None;
                }
                Err(e) => state.error = Some(e),
            }
            Task::none()
        }
        Message::AnalyzeDone(result) => {
            match result {
                Ok(candidates) => {
                    state.candidates = candidates;
                    state.error = None;
                }
                Err(e) => state.error = Some(e),
            }
            Task::none()
        }
        Message::ToggleTrace => {
            state.show_trace = !state.show_trace;
            Task::none()
        }
    }
}

pub fn view(state: &State) -> Element<'_, Message> {
    let mode_bar = row![
        button(text("Forward (join)").size(13).font(theme::latin()))
            .on_press(Message::SetMode(Mode::Forward))
            .padding([6, 16])
            .style(if state.mode == Mode::Forward {
                theme::toggle_active
            } else {
                theme::toggle_inactive
            }),
        button(text("Reverse (split)").size(13).font(theme::latin()))
            .on_press(Message::SetMode(Mode::Reverse))
            .padding([6, 16])
            .style(if state.mode == Mode::Reverse {
                theme::toggle_active
            } else {
                theme::toggle_inactive
            }),
    ]
    .spacing(0);

    let body: Element<'_, Message> = match state.mode {
        Mode::Forward => forward_view(state),
        Mode::Reverse => reverse_view(state),
    };

    let mut content = column![mode_bar, body].spacing(16);

    if let Some(ref err) = state.error {
        content = content.push(text(err.clone()).size(14).color(theme::ERROR_TEXT));
    }

    content.into()
}

fn forward_view(state: &State) -> Element<'_, Message> {
    let first_field = column![
        theme::label("First word (IAST)"),
        text_input("", &state.first)
            .on_input(Message::FirstChanged)
            .on_submit(Message::Join)
            .style(theme::input_style)
            .width(200),
    ]
    .spacing(4);

    let second_field = column![
        theme::label("Second word (IAST)"),
        text_input("", &state.second)
            .on_input(Message::SecondChanged)
            .on_submit(Message::Join)
            .style(theme::input_style)
            .width(200),
    ]
    .spacing(4);

    let join_btn = button(text("Join").size(14).font(theme::latin()))
        .on_press(Message::Join)
        .padding([8, 20])
        .style(theme::accent_btn);

    let inputs = row![first_field, second_field, join_btn]
        .spacing(12)
        .align_y(Center);

    let mut content = column![inputs].spacing(16);

    if let Some(ref jr) = state.join_result {
        let deva = to_devanagari(&jr.form);
        let result_card = container(
            column![
                text(deva).font(theme::devanagari()).size(28),
                text(jr.form.clone())
                    .font(theme::latin())
                    .size(18)
                    .color(theme::TEXT_SECONDARY),
            ]
            .spacing(4)
            .padding(4),
        )
        .style(theme::card)
        .padding(16);

        content = content.push(result_card);

        if state.join_trace.is_some() {
            let label = if state.show_trace {
                "Hide trace"
            } else {
                "Show trace"
            };
            let trace_btn = button(text(label).size(13).font(theme::latin()))
                .on_press(Message::ToggleTrace)
                .padding([6, 14])
                .style(theme::toggle_inactive);
            content = content.push(trace_btn);
        }

        if state.show_trace {
            if let Some(ref trace) = state.join_trace {
                content = content.push(trace_panel(trace));
            }
        }
    }

    content.into()
}

fn reverse_view(state: &State) -> Element<'_, Message> {
    let form_field = column![
        theme::label("Combined form (IAST)"),
        text_input("", &state.form)
            .on_input(Message::FormChanged)
            .on_submit(Message::Analyze)
            .style(theme::input_style)
            .width(300),
    ]
    .spacing(4);

    let analyze_btn = button(text("Analyze").size(14).font(theme::latin()))
        .on_press(Message::Analyze)
        .padding([8, 20])
        .style(theme::accent_btn);

    let inputs = row![form_field, analyze_btn]
        .spacing(12)
        .align_y(Center);

    let mut content = column![inputs].spacing(16);

    if !state.candidates.is_empty() {
        let status = text(format!(
            "{} — {} candidate{}",
            state.form,
            state.candidates.len(),
            if state.candidates.len() != 1 { "s" } else { "" }
        ))
        .size(14)
        .color(theme::TEXT_SECONDARY)
        .font(theme::latin());
        content = content.push(status);

        let mut list = Column::new().spacing(8);
        for c in &state.candidates {
            let first_deva = to_devanagari(&c.first);
            let second_deva = to_devanagari(&c.second);
            let card = container(
                row![
                    column![
                        text(format!("{} + {}", first_deva, second_deva))
                            .font(theme::devanagari())
                            .size(18),
                        text(format!("{} + {}", c.first, c.second))
                            .font(theme::latin())
                            .size(14)
                            .color(theme::TEXT_SECONDARY),
                    ]
                    .spacing(2)
                    .width(280),
                    text(c.rule_ref.clone().unwrap_or_default())
                        .size(13)
                        .color(theme::ACCENT)
                        .width(100),
                    text(c.rule.clone())
                        .size(13)
                        .color(theme::TEXT_SECONDARY),
                ]
                .spacing(12)
                .align_y(Center),
            )
            .style(theme::card)
            .padding([12, 16]);
            list = list.push(card);
        }
        content = content.push(scrollable(list));
    } else if !state.form.is_empty() && state.error.is_none() {
        content = content.push(
            text("No decompositions found.")
                .size(14)
                .color(theme::TEXT_SECONDARY),
        );
    }

    content.into()
}

fn trace_panel(trace: &[TraceStep]) -> Element<'_, Message> {
    let col_widths: [f32; 5] = [40.0, 260.0, 90.0, 140.0, 140.0];
    let col_labels = ["#", "RULE", "SŪTRA", "INPUT", "OUTPUT"];

    let table_header = {
        let mut r = iced::widget::Row::new().spacing(6);
        for (i, label) in col_labels.iter().enumerate() {
            r = r.push(
                text(label.to_string())
                    .size(10)
                    .font(theme::latin())
                    .color(theme::TEXT_SECONDARY)
                    .width(col_widths[i]),
            );
        }
        container(r).padding([6, 8])
    };

    let mut table_rows = Column::new().spacing(0);
    for step in trace {
        let r = row![
            text(step.step.to_string())
                .size(12)
                .color(theme::TEXT_SECONDARY)
                .width(col_widths[0]),
            text(step.rule.clone()).size(12).width(col_widths[1]),
            text(step.rule_ref.clone().unwrap_or_default())
                .size(12)
                .color(theme::ACCENT)
                .width(col_widths[2]),
            text(step.input_state.clone()).size(12).width(col_widths[3]),
            text(step.output_state.clone()).size(12).width(col_widths[4]),
        ]
        .spacing(6);
        table_rows = table_rows.push(container(r).padding([4, 8]));
    }

    container(column![table_header, table_rows].spacing(4))
        .style(theme::trace_container)
        .padding(16)
        .into()
}
