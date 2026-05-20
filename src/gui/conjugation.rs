use std::sync::Arc;

use iced::widget::{
    button, column, container, pick_list, row, scrollable, text, text_input, Column, Row,
};
use iced::{Center, Element, Task};

use crate::engine::conjugation::{ConjugationInput, derive_conjugation};
use crate::engine::script::to_devanagari;
use crate::engine::TraceStep;
use crate::mcp::{ParadigmCell, PURUSHAS, VACANAS};
use crate::rule_cache::RuleCache;

use super::theme;

const PURUSHA_LABELS: [(&str, &str); 3] = [
    ("prathama", "prathama (3rd)"),
    ("madhyama", "madhyama (2nd)"),
    ("uttama", "uttama (1st)"),
];

const VACANA_LABELS: [(&str, &str); 3] = [
    ("ekavacana", "ekavacana"),
    ("dvivacana", "dvivacana"),
    ("bahuvacana", "bahuvacana"),
];

const KNOWN_DHATUS: &[&str] = &["bhū", "nī", "budh", "paṭh", "ji", "hu", "dhā", "bhī"];
const GANAS: &[&str] = &["1", "2", "3", "4", "5", "6", "7", "8", "9", "10"];

#[derive(Default)]
pub struct State {
    pub dhatu: String,
    pub gana: Option<String>,
    pub cells: Vec<ParadigmCell>,
    pub selected_cell: Option<usize>,
    pub error: Option<String>,
    pub computing: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    DhatuChanged(String),
    GanaSelected(String),
    Generate,
    ResultReady(Result<Vec<ParadigmCell>, String>),
    SelectCell(usize),
    CloseTrace,
}

pub fn update(cache: &Arc<RuleCache>, state: &mut State, msg: Message) -> Task<Message> {
    match msg {
        Message::DhatuChanged(v) => {
            state.dhatu = v;
            Task::none()
        }
        Message::GanaSelected(v) => {
            state.gana = Some(v);
            Task::none()
        }
        Message::Generate => {
            let Some(ref gana) = state.gana else {
                return Task::none();
            };
            if state.dhatu.is_empty() {
                return Task::none();
            }
            state.computing = true;
            state.selected_cell = None;
            let cache = cache.clone();
            let dhatu = state.dhatu.clone();
            let gana = gana.clone();
            Task::perform(
                async move { generate_conjugation_paradigm(&cache, &dhatu, &gana) },
                Message::ResultReady,
            )
        }
        Message::ResultReady(result) => {
            state.computing = false;
            match result {
                Ok(cells) => {
                    state.cells = cells;
                    state.error = None;
                }
                Err(e) => state.error = Some(e),
            }
            Task::none()
        }
        Message::SelectCell(i) => {
            state.selected_cell = Some(i);
            Task::none()
        }
        Message::CloseTrace => {
            state.selected_cell = None;
            Task::none()
        }
    }
}

fn generate_conjugation_paradigm(
    cache: &RuleCache,
    dhatu: &str,
    gana: &str,
) -> Result<Vec<ParadigmCell>, String> {
    let tin = cache.get_rules("tin_suffix");
    let vikarana = cache.get_rules("vikarana_rule");
    let verb_anga = cache.get_rules("verb_anga_rule");
    let tripadi = cache.get_rules("tripadi_rule");

    let mut cells = Vec::with_capacity(9);
    for purusha in PURUSHAS {
        for vacana in VACANAS {
            let input = ConjugationInput {
                dhatu: dhatu.into(),
                gana: gana.into(),
                lakara: "laṭ".into(),
                pada: "parasmaipada".into(),
                purusha: purusha.into(),
                vacana: vacana.into(),
            };
            match derive_conjugation(tin, vikarana, verb_anga, tripadi, input) {
                Ok(result) => {
                    let form = result.output["form"].as_str().map(String::from);
                    cells.push(ParadigmCell {
                        case: purusha.into(),
                        number: vacana.into(),
                        form,
                        trace: Some(result.trace),
                        error: None,
                    });
                }
                Err(e) => {
                    cells.push(ParadigmCell {
                        case: purusha.into(),
                        number: vacana.into(),
                        form: None,
                        trace: None,
                        error: Some(e.to_string()),
                    });
                }
            }
        }
    }
    Ok(cells)
}

pub fn view(state: &State) -> Element<'_, Message> {
    let dhatu_field = column![
        theme::label("Dhātu (IAST)"),
        text_input("e.g. bhū", &state.dhatu)
            .on_input(Message::DhatuChanged)
            .on_submit(Message::Generate)
            .style(theme::input_style)
            .width(200),
    ]
    .spacing(4);

    let gana_options: Vec<String> = GANAS.iter().map(|s| s.to_string()).collect();
    let gana_field = column![
        theme::label("Gaṇa"),
        pick_list(gana_options, state.gana.clone(), Message::GanaSelected).width(80),
    ]
    .spacing(4);

    let lakara_label = column![
        theme::label("Lakāra"),
        text("laṭ")
            .size(14)
            .font(theme::latin())
            .color(theme::TEXT_SECONDARY),
    ]
    .spacing(4);

    let pada_label = column![
        theme::label("Pada"),
        text("parasmaipada")
            .size(14)
            .font(theme::latin())
            .color(theme::TEXT_SECONDARY),
    ]
    .spacing(4);

    let gen_button = button(text("Generate").size(14).font(theme::latin()))
        .on_press(Message::Generate)
        .padding([8, 20])
        .style(theme::accent_btn);

    let inputs = row![dhatu_field, gana_field, lakara_label, pada_label, gen_button]
        .spacing(12)
        .align_y(Center);

    let dhatu_hints = row(KNOWN_DHATUS.iter().map(|d| {
        button(text(*d).size(12).font(theme::latin()))
            .on_press(Message::DhatuChanged(d.to_string()))
            .padding([4, 8])
            .style(theme::close_btn)
            .into()
    }))
    .spacing(4);

    let mut content = column![inputs, dhatu_hints].spacing(8);

    if state.computing {
        content = content.push(
            text("Computing…")
                .size(14)
                .color(theme::TEXT_SECONDARY),
        );
    }

    if let Some(ref err) = state.error {
        content = content.push(text(err.clone()).size(14).color(theme::ERROR_TEXT));
    }

    if !state.cells.is_empty() {
        let gana_str = state.gana.as_deref().unwrap_or("");
        let status = text(format!("√{} (gaṇa {}, laṭ, parasmaipada)", state.dhatu, gana_str))
            .size(14)
            .color(theme::TEXT_SECONDARY)
            .font(theme::latin());
        content = content.push(status);
        content = content.push(conjugation_grid(state));
    }

    if let Some(trace_view) = state
        .selected_cell
        .and_then(|i| state.cells.get(i))
        .and_then(|c| c.trace.as_ref())
        .map(|t| trace_panel(t, state))
    {
        content = content.push(trace_view);
    }

    scrollable(content).into()
}

fn conjugation_grid(state: &State) -> Element<'_, Message> {
    let cell_w: f32 = 190.0;
    let label_w: f32 = 150.0;

    let mut header = Row::new().spacing(0);
    header = header.push(
        container(text("").width(label_w))
            .style(theme::header_cell)
            .padding([8, 10]),
    );
    for (_, label) in &VACANA_LABELS {
        header = header.push(
            container(
                text(label.to_uppercase())
                    .size(11)
                    .font(theme::latin())
                    .color(theme::TEXT_SECONDARY)
                    .width(cell_w)
                    .center(),
            )
            .style(theme::header_cell)
            .padding([8, 10]),
        );
    }

    let mut rows = Column::new().spacing(0);
    rows = rows.push(header);

    for (pi, (_, purusha_label)) in PURUSHA_LABELS.iter().enumerate() {
        let mut r = Row::new().spacing(0);
        r = r.push(
            container(
                text(purusha_label.to_string())
                    .size(12)
                    .font(theme::latin())
                    .color(theme::TEXT_SECONDARY)
                    .width(label_w),
            )
            .style(theme::header_cell)
            .padding([8, 10]),
        );

        for (vi, _) in VACANA_LABELS.iter().enumerate() {
            let cell_idx = pi * 3 + vi;
            if let Some(cell) = state.cells.get(cell_idx) {
                let cell_content: Element<'_, Message> = if let Some(ref form) = cell.form {
                    let deva = to_devanagari(form);
                    let content = column![
                        text(deva).font(theme::devanagari()).size(18).center(),
                        text(form.clone())
                            .font(theme::latin())
                            .size(13)
                            .color(theme::TEXT_SECONDARY)
                            .center(),
                    ]
                    .spacing(1)
                    .width(cell_w)
                    .align_x(Center);
                    button(content)
                        .on_press(Message::SelectCell(cell_idx))
                        .padding([8, 10])
                        .style(theme::cell_btn)
                        .into()
                } else if cell.error.is_some() {
                    container(
                        text("error")
                            .size(12)
                            .color(theme::ERROR_TEXT)
                            .center()
                            .width(cell_w),
                    )
                    .style(|_theme| container::Style {
                        background: Some(iced::Background::Color(theme::ERROR_BG)),
                        border: iced::Border {
                            color: theme::BORDER_COLOR,
                            width: 0.5,
                            radius: 0.0.into(),
                        },
                        ..container::Style::default()
                    })
                    .padding([8, 10])
                    .into()
                } else {
                    container(
                        text("—")
                            .size(18)
                            .color(theme::TEXT_SECONDARY)
                            .center()
                            .width(cell_w),
                    )
                    .style(theme::grid_cell)
                    .padding([8, 10])
                    .into()
                };
                r = r.push(cell_content);
            }
        }
        rows = rows.push(r);
    }

    container(rows).style(theme::card).into()
}

fn trace_panel<'a>(trace: &'a [TraceStep], state: &'a State) -> Element<'a, Message> {
    let title = state
        .selected_cell
        .and_then(|i| state.cells.get(i))
        .map(|c| {
            let purusha_label = PURUSHA_LABELS
                .iter()
                .find(|(k, _)| *k == c.case)
                .map(|(_, v)| *v)
                .unwrap_or(&c.case);
            let vacana_label = VACANA_LABELS
                .iter()
                .find(|(k, _)| *k == c.number)
                .map(|(_, v)| *v)
                .unwrap_or(&c.number);
            let form = c.form.as_deref().unwrap_or("?");
            format!("{purusha_label} {vacana_label} — {form}")
        })
        .unwrap_or_default();

    let header_row = row![
        text(title).size(14).font(theme::latin()),
        iced::widget::Space::new().width(iced::Fill),
        button(text("×").size(18))
            .on_press(Message::CloseTrace)
            .padding([2, 8])
            .style(theme::close_btn),
    ]
    .align_y(Center);

    let col_widths: [f32; 5] = [40.0, 260.0, 90.0, 140.0, 140.0];
    let col_labels = ["#", "RULE", "SŪTRA", "INPUT", "OUTPUT"];

    let table_header = {
        let mut r = Row::new().spacing(6);
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
            text(step.output_state.clone())
                .size(12)
                .width(col_widths[4]),
        ]
        .spacing(6);
        table_rows = table_rows.push(container(r).padding([4, 8]));
    }

    container(column![header_row, table_header, table_rows].spacing(8))
        .style(theme::trace_container)
        .padding(16)
        .into()
}
