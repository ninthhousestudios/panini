use std::sync::Arc;

use iced::widget::{button, column, row, scrollable, text, text_input, Column};
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
        Message::FirstChanged(v) => { state.first = v; Task::none() }
        Message::SecondChanged(v) => { state.second = v; Task::none() }
        Message::FormChanged(v) => { state.form = v; Task::none() }
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
    let fwd_label = if state.mode == Mode::Forward { "> Forward (join)" } else { "  Forward (join)" };
    let rev_label = if state.mode == Mode::Reverse { "> Reverse (split)" } else { "  Reverse (split)" };

    let mode_bar = row![
        button(text(fwd_label)).on_press(Message::SetMode(Mode::Forward)),
        button(text(rev_label)).on_press(Message::SetMode(Mode::Reverse)),
    ]
    .spacing(4);

    let body: Element<'_, Message> = match state.mode {
        Mode::Forward => forward_view(state),
        Mode::Reverse => reverse_view(state),
    };

    let mut content = column![mode_bar, body].spacing(12);

    if let Some(ref err) = state.error {
        content = content.push(text(err.clone()).size(14));
    }

    content.into()
}

fn forward_view(state: &State) -> Element<'_, Message> {
    let first_input = text_input("First word (IAST)", &state.first)
        .on_input(Message::FirstChanged)
        .on_submit(Message::Join)
        .width(200);
    let second_input = text_input("Second word (IAST)", &state.second)
        .on_input(Message::SecondChanged)
        .on_submit(Message::Join)
        .width(200);
    let join_btn = button(text("Join")).on_press(Message::Join);

    let inputs = row![first_input, text("+").size(18), second_input, join_btn]
        .spacing(8)
        .align_y(Center);

    let mut content = column![inputs].spacing(12);

    if let Some(ref jr) = state.join_result {
        let deva = to_devanagari(&jr.form);
        let result_card = column![
            text(deva).font(theme::devanagari()).size(28),
            text(jr.form.clone()).font(theme::latin()).size(20),
        ]
        .spacing(4);
        content = content.push(result_card);

        if state.join_trace.is_some() {
            let trace_btn = button(text(if state.show_trace {
                "Hide trace"
            } else {
                "Show trace"
            }))
            .on_press(Message::ToggleTrace);
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
    let form_input = text_input("Combined form (IAST)", &state.form)
        .on_input(Message::FormChanged)
        .on_submit(Message::Analyze)
        .width(300);
    let analyze_btn = button(text("Analyze")).on_press(Message::Analyze);

    let inputs = row![form_input, analyze_btn].spacing(8).align_y(Center);

    let mut content = column![inputs].spacing(12);

    if !state.candidates.is_empty() {
        let status = format!("{} — {} candidates", state.form, state.candidates.len());
        content = content.push(text(status).size(14).font(theme::latin()));

        let mut list = Column::new().spacing(6);
        for c in &state.candidates {
            let first_deva = to_devanagari(&c.first);
            let second_deva = to_devanagari(&c.second);
            let card = row![
                column![
                    text(format!("{} + {}", first_deva, second_deva))
                        .font(theme::devanagari())
                        .size(18),
                    text(format!("{} + {}", c.first, c.second))
                        .font(theme::latin())
                        .size(14),
                ]
                .spacing(1)
                .width(300),
                text(c.rule_ref.clone().unwrap_or_default())
                    .size(13)
                    .width(100),
                text(c.rule.clone()).size(13),
            ]
            .spacing(12)
            .align_y(Center);
            list = list.push(card);
        }
        content = content.push(scrollable(list));
    } else if !state.form.is_empty() && state.error.is_none() {
        content = content.push(text("No decompositions found.").size(14));
    }

    content.into()
}

fn trace_panel(trace: &[TraceStep]) -> Element<'_, Message> {
    let mut rows = Column::new().spacing(2);
    let header = row![
        text("Step").size(14).font(theme::latin()).width(50),
        text("Rule").size(14).font(theme::latin()).width(300),
        text("Sūtra").size(14).font(theme::latin()).width(100),
        text("Input").size(14).font(theme::latin()).width(150),
        text("Output").size(14).font(theme::latin()).width(150),
    ]
    .spacing(8);
    rows = rows.push(header);

    for step in trace {
        let r = row![
            text(step.step.to_string()).size(13).width(50),
            text(step.rule.clone()).size(13).width(300),
            text(step.rule_ref.clone().unwrap_or_default()).size(13).width(100),
            text(step.input_state.clone()).size(13).width(150),
            text(step.output_state.clone()).size(13).width(150),
        ]
        .spacing(8);
        rows = rows.push(r);
    }

    rows.into()
}
