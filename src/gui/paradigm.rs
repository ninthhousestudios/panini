use std::sync::Arc;

use iced::widget::{button, column, row, scrollable, text, text_input, Column, Row};
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

#[derive(Default)]
pub struct State {
    pub stem: String,
    pub stem_type: String,
    pub cells: Vec<ParadigmCell>,
    pub selected_cell: Option<usize>,
    pub error: Option<String>,
    pub computing: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    StemChanged(String),
    StemTypeChanged(String),
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
        Message::StemTypeChanged(v) => {
            state.stem_type = v;
            Task::none()
        }
        Message::Generate => {
            if state.stem.is_empty() || state.stem_type.is_empty() {
                return Task::none();
            }
            state.computing = true;
            state.selected_cell = None;
            let cache = cache.clone();
            let stem = state.stem.clone();
            let stem_type = state.stem_type.clone();
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
    let stem_input = text_input("Stem (IAST), e.g. deva", &state.stem)
        .on_input(Message::StemChanged)
        .on_submit(Message::Generate)
        .width(200);

    let type_input = text_input("Stem type, e.g. a-stem-m", &state.stem_type)
        .on_input(Message::StemTypeChanged)
        .on_submit(Message::Generate)
        .width(200);

    let gen_button = button(text("Generate")).on_press(Message::Generate);

    let inputs = row![stem_input, type_input, gen_button]
        .spacing(8)
        .align_y(Center);

    let mut content = column![inputs].spacing(12);

    if state.computing {
        content = content.push(text("Computing…").size(16));
    }

    if let Some(ref err) = state.error {
        content = content.push(text(err.clone()).size(14));
    }

    if !state.cells.is_empty() {
        let status = format!(
            "{} ({})",
            state.stem,
            state.stem_type
        );
        content = content.push(text(status).size(14).font(theme::latin()));
        content = content.push(paradigm_grid(state));
    }

    if let Some(trace_view) = state
        .selected_cell
        .and_then(|i| state.cells.get(i))
        .and_then(|c| c.trace.as_ref())
        .map(|t| trace_panel(t))
    {
        content = content.push(trace_view);
    }

    scrollable(content).into()
}

fn paradigm_grid(state: &State) -> Element<'_, Message> {
    let cell_w: f32 = 200.0;
    let label_w: f32 = 160.0;
    let sz: f32 = 18.0;

    let mut header = Row::new().spacing(4);
    header = header.push(text("").width(label_w));
    for (_, label) in &NUMBER_LABELS {
        header = header.push(
            text(label.to_string())
                .size(sz - 2.0)
                .font(theme::latin())
                .width(cell_w),
        );
    }

    let mut rows = Column::new().spacing(2);
    rows = rows.push(header);

    for (ci, (_, case_label)) in CASE_LABELS.iter().enumerate() {
        let mut r = Row::new().spacing(4);
        r = r.push(
            text(case_label.to_string())
                .size(sz - 2.0)
                .font(theme::latin())
                .width(label_w),
        );

        for (ni, _) in NUMBER_LABELS.iter().enumerate() {
            let cell_idx = ci * 3 + ni;
            if let Some(cell) = state.cells.get(cell_idx) {
                let cell_content: Element<'_, Message> = if let Some(ref form) = cell.form {
                    let deva = to_devanagari(form);
                    let content = column![
                        text(deva).font(theme::devanagari()).size(sz),
                        text(form.clone()).font(theme::latin()).size(sz - 4.0),
                    ]
                    .spacing(1);
                    button(content)
                        .on_press(Message::SelectCell(cell_idx))
                        .width(cell_w)
                        .into()
                } else if cell.error.is_some() {
                    text("error").size(sz - 4.0).width(cell_w).into()
                } else {
                    text("—").size(sz).width(cell_w).into()
                };
                r = r.push(cell_content);
            }
        }
        rows = rows.push(r);
    }

    rows.into()
}

fn trace_panel(trace: &[TraceStep]) -> Element<'_, Message> {
    let close = button(text("Close trace")).on_press(Message::CloseTrace);

    let header = row![
        text("Step").size(14).font(theme::latin()).width(50),
        text("Rule").size(14).font(theme::latin()).width(300),
        text("Sūtra").size(14).font(theme::latin()).width(100),
        text("Input").size(14).font(theme::latin()).width(150),
        text("Output").size(14).font(theme::latin()).width(150),
    ]
    .spacing(8);

    let mut rows = Column::new().spacing(2);
    rows = rows.push(header);
    for step in trace {
        let r = row![
            text(step.step.to_string()).size(13).width(50),
            text(step.rule.clone()).size(13).width(300),
            text(step.rule_ref.clone().unwrap_or_default())
                .size(13)
                .width(100),
            text(step.input_state.clone()).size(13).width(150),
            text(step.output_state.clone()).size(13).width(150),
        ]
        .spacing(8);
        rows = rows.push(r);
    }

    column![close, rows].spacing(8).into()
}
