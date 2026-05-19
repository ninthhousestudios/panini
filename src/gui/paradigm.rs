use std::sync::Arc;

use iced::widget::{button, column, container, pick_list, row, scrollable, text, text_input, Column, Row};
use iced::{Center, Element, Task};

use crate::engine::declension::{DeclensionInput, derive_declension};
use crate::engine::script::to_devanagari;
use crate::engine::TraceStep;
use crate::mcp::{CASES, NUMBERS, ParadigmCell};
use crate::rule_cache::RuleCache;

use super::theme;

const CASE_LABELS: [(&str, &str); 8] = [
    ("1", "prathamā (nom.)"),
    ("2", "dvitīyā (acc.)"),
    ("3", "tṛtīyā (inst.)"),
    ("4", "caturthī (dat.)"),
    ("5", "pañcamī (abl.)"),
    ("6", "ṣaṣṭhī (gen.)"),
    ("7", "saptamī (loc.)"),
    ("8", "sambodhana (voc.)"),
];

const NUMBER_LABELS: [(&str, &str); 3] = [
    ("sg", "ekavacana"),
    ("du", "dvivacana"),
    ("pl", "bahuvacana"),
];

const STEM_TYPES: &[&str] = &["a-stem-m", "a-stem-n", "aa-stem-f"];

#[derive(Default)]
pub struct State {
    pub stem: String,
    pub stem_type: Option<String>,
    pub cells: Vec<ParadigmCell>,
    pub selected_cell: Option<usize>,
    pub error: Option<String>,
    pub computing: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    StemChanged(String),
    StemTypeSelected(String),
    Generate,
    ResultReady(Result<Vec<ParadigmCell>, String>),
    SelectCell(usize),
    CloseTrace,
}

pub fn update(cache: &Arc<RuleCache>, state: &mut State, msg: Message) -> Task<Message> {
    match msg {
        Message::StemChanged(v) => {
            state.stem = v;
            Task::none()
        }
        Message::StemTypeSelected(v) => {
            state.stem_type = Some(v);
            Task::none()
        }
        Message::Generate => {
            let Some(ref stem_type) = state.stem_type else {
                return Task::none();
            };
            if state.stem.is_empty() {
                return Task::none();
            }
            state.computing = true;
            state.selected_cell = None;
            let cache = cache.clone();
            let stem = state.stem.clone();
            let stem_type = stem_type.clone();
            Task::perform(
                async move { generate_paradigm(&cache, &stem, &stem_type) },
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

fn generate_paradigm(
    cache: &RuleCache,
    stem: &str,
    stem_type: &str,
) -> Result<Vec<ParadigmCell>, String> {
    let sup = cache.get_rules("sup_suffix");
    let pratyaya = cache.get_rules("pratyaya_rule");
    let anga = cache.get_rules("anga_rule");
    let sandhi = cache.get_rules("sandhi_rule");
    let tripadi = cache.get_rules("tripadi_rule");

    let mut cells = Vec::with_capacity(24);
    for case in CASES {
        for number in NUMBERS {
            let input = DeclensionInput {
                stem: stem.into(),
                stem_type: stem_type.into(),
                case: case.into(),
                number: number.into(),
            };
            match derive_declension(sup, pratyaya, anga, sandhi, tripadi, input) {
                Ok(result) => {
                    let form = result.output["form"].as_str().map(String::from);
                    cells.push(ParadigmCell {
                        case: case.into(),
                        number: number.into(),
                        form,
                        trace: Some(result.trace),
                        error: None,
                    });
                }
                Err(e) => {
                    cells.push(ParadigmCell {
                        case: case.into(),
                        number: number.into(),
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
    let stem_field = column![
        theme::label("Stem (IAST)"),
        text_input("", &state.stem)
            .on_input(Message::StemChanged)
            .on_submit(Message::Generate)
            .style(theme::input_style)
            .width(200),
    ]
    .spacing(4);

    let options: Vec<String> = STEM_TYPES.iter().map(|s| s.to_string()).collect();
    let type_field = column![
        theme::label("Stem type"),
        pick_list(options, state.stem_type.clone(), Message::StemTypeSelected)
            .width(200),
    ]
    .spacing(4);

    let gen_button = button(text("Generate").size(14).font(theme::latin()))
        .on_press(Message::Generate)
        .padding([8, 20])
        .style(theme::accent_btn);

    let inputs = row![stem_field, type_field, gen_button]
        .spacing(12)
        .align_y(Center);

    let mut content = column![inputs].spacing(16);

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
        let st = state.stem_type.as_deref().unwrap_or("");
        let status = text(format!("{} ({})", state.stem, st))
            .size(14)
            .color(theme::TEXT_SECONDARY)
            .font(theme::latin());
        content = content.push(status);
        content = content.push(paradigm_grid(state));
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

fn paradigm_grid(state: &State) -> Element<'_, Message> {
    let cell_w: f32 = 190.0;
    let label_w: f32 = 150.0;

    let mut header = Row::new().spacing(0);
    header = header.push(
        container(text("").width(label_w))
            .style(theme::header_cell)
            .padding([8, 10]),
    );
    for (_, label) in &NUMBER_LABELS {
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

    for (ci, (_, case_label)) in CASE_LABELS.iter().enumerate() {
        let mut r = Row::new().spacing(0);
        r = r.push(
            container(
                text(case_label.to_string())
                    .size(12)
                    .font(theme::latin())
                    .color(theme::TEXT_SECONDARY)
                    .width(label_w),
            )
            .style(theme::header_cell)
            .padding([8, 10]),
        );

        for (ni, _) in NUMBER_LABELS.iter().enumerate() {
            let cell_idx = ci * 3 + ni;
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

    container(rows)
        .style(theme::card)
        .into()
}

fn trace_panel<'a>(trace: &'a [TraceStep], state: &'a State) -> Element<'a, Message> {
    let title = state
        .selected_cell
        .and_then(|i| state.cells.get(i))
        .map(|c| {
            let case_label = CASE_LABELS
                .iter()
                .find(|(k, _)| *k == c.case)
                .map(|(_, v)| *v)
                .unwrap_or(&c.case);
            let num_label = NUMBER_LABELS
                .iter()
                .find(|(k, _)| *k == c.number)
                .map(|(_, v)| *v)
                .unwrap_or(&c.number);
            let form = c.form.as_deref().unwrap_or("?");
            format!("{case_label} {num_label} — {form}")
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
            text(step.output_state.clone()).size(12).width(col_widths[4]),
        ]
        .spacing(6);
        table_rows = table_rows.push(container(r).padding([4, 8]));
    }

    container(column![header_row, table_header, table_rows].spacing(8))
        .style(theme::trace_container)
        .padding(16)
        .into()
}
