pub mod conjugation;
pub mod paradigm;
pub mod sandhi;
pub mod sutra;
pub mod theme;
pub mod verify;

use std::sync::Arc;

use iced::widget::{button, column, container, row, rule, text};
use iced::{Center, Element, Fill, Size, Task, Theme};

use crate::rule_cache::RuleCache;

const NOTO_DEVA: &[u8] = include_bytes!("../../fonts/NotoSansDevanagari-Regular.ttf");
const NOTO_SANS: &[u8] = include_bytes!("../../fonts/NotoSans-Regular.ttf");

const SCALE_STEP: f32 = 0.1;
const SCALE_MIN: f32 = 0.6;
const SCALE_MAX: f32 = 2.0;
const SCALE_DEFAULT: f32 = 1.1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Paradigm,
    Conjugation,
    Sandhi,
    Sutras,
    Verify,
}

struct State {
    cache: Arc<RuleCache>,
    active_tab: Tab,
    scale: f32,
    paradigm: paradigm::State,
    conjugation: conjugation::State,
    sandhi: sandhi::State,
    sutra: sutra::State,
    verify: verify::State,
}

#[derive(Debug, Clone)]
enum Message {
    SwitchTab(Tab),
    ZoomIn,
    ZoomOut,
    ZoomReset,
    Paradigm(paradigm::Message),
    Conjugation(conjugation::Message),
    Sandhi(sandhi::Message),
    Sutra(sutra::Message),
    Verify(verify::Message),
}

fn boot(cache: Arc<RuleCache>) -> (State, Task<Message>) {
    let state = State {
        cache,
        active_tab: Tab::Paradigm,
        scale: SCALE_DEFAULT,
        paradigm: paradigm::State::default(),
        conjugation: conjugation::State::default(),
        sandhi: sandhi::State::default(),
        sutra: sutra::State::default(),
        verify: verify::State::default(),
    };
    (state, Task::none())
}

fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::SwitchTab(tab) => {
            state.active_tab = tab;
            match tab {
                Tab::Sutras if !state.sutra.loaded => {
                    state.sutra.loaded = true;
                    sutra::load(&state.cache).map(Message::Sutra)
                }
                Tab::Verify if !state.verify.loaded && !state.verify.loading => {
                    state.verify.loading = true;
                    verify::load(&state.cache).map(Message::Verify)
                }
                _ => Task::none(),
            }
        }
        Message::ZoomIn => {
            state.scale = (state.scale + SCALE_STEP).min(SCALE_MAX);
            Task::none()
        }
        Message::ZoomOut => {
            state.scale = (state.scale - SCALE_STEP).max(SCALE_MIN);
            Task::none()
        }
        Message::ZoomReset => {
            state.scale = SCALE_DEFAULT;
            Task::none()
        }
        Message::Paradigm(msg) => {
            paradigm::update(&state.cache, &mut state.paradigm, msg).map(Message::Paradigm)
        }
        Message::Conjugation(msg) => {
            conjugation::update(&state.cache, &mut state.conjugation, msg)
                .map(Message::Conjugation)
        }
        Message::Sandhi(msg) => {
            sandhi::update(&state.cache, &mut state.sandhi, msg).map(Message::Sandhi)
        }
        Message::Sutra(msg) => sutra::update(&mut state.sutra, msg).map(Message::Sutra),
        Message::Verify(msg) => verify::update(&mut state.verify, msg).map(Message::Verify),
    }
}

fn view(state: &State) -> Element<'_, Message> {
    let heading = text("Pāṇini")
        .size(24)
        .color(theme::ACCENT)
        .font(theme::latin());

    let pct = format!("{}%", (state.scale * 100.0).round() as i32);
    let zoom_controls = row![
        button(text("\u{2212}").size(14))
            .on_press(Message::ZoomOut)
            .padding([2, 8])
            .style(theme::close_btn),
        button(text(pct).size(11).font(theme::latin()).color(theme::TEXT_SECONDARY))
            .on_press(Message::ZoomReset)
            .padding([2, 6])
            .style(theme::close_btn),
        button(text("+").size(14))
            .on_press(Message::ZoomIn)
            .padding([2, 8])
            .style(theme::close_btn),
    ]
    .spacing(2)
    .align_y(Center);

    let header = row![
        heading,
        iced::widget::Space::new().width(Fill),
        zoom_controls,
    ]
    .align_y(Center);

    let tab_bar = row![
        tab_button("Paradigms", Tab::Paradigm, state.active_tab),
        tab_button("Conjugation", Tab::Conjugation, state.active_tab),
        tab_button("Sandhi", Tab::Sandhi, state.active_tab),
        tab_button("Sūtras", Tab::Sutras, state.active_tab),
        tab_button("Verification", Tab::Verify, state.active_tab),
    ]
    .spacing(4);

    let divider = rule::horizontal(2).style(theme::tab_rule);

    let body: Element<'_, Message> = match state.active_tab {
        Tab::Paradigm => paradigm::view(&state.paradigm).map(Message::Paradigm),
        Tab::Conjugation => conjugation::view(&state.conjugation).map(Message::Conjugation),
        Tab::Sandhi => sandhi::view(&state.sandhi).map(Message::Sandhi),
        Tab::Sutras => sutra::view(&state.sutra).map(Message::Sutra),
        Tab::Verify => verify::view(&state.verify).map(Message::Verify),
    };

    let content = column![header, tab_bar, divider, body]
        .spacing(12)
        .padding(24)
        .max_width(960);

    container(content)
        .center_x(Fill)
        .into()
}

fn tab_button(label: &str, tab: Tab, active: Tab) -> Element<'_, Message> {
    let is_active = tab == active;
    let t = text(label).size(14).font(theme::latin());
    button(t)
        .on_press(Message::SwitchTab(tab))
        .padding([6, 16])
        .style(if is_active {
            theme::tab_active
        } else {
            theme::tab_inactive
        })
        .into()
}

fn app_theme(_state: &State) -> Theme {
    theme::panini_theme()
}

pub fn run(cache: Arc<RuleCache>) -> iced::Result {
    iced::application(move || boot(cache.clone()), update, view)
        .title("Pāṇini")
        .theme(app_theme)
        .scale_factor(|state: &State| state.scale)
        .font(NOTO_DEVA)
        .font(NOTO_SANS)
        .default_font(theme::latin())
        .window_size(Size::new(1100.0, 800.0))
        .centered()
        .antialiasing(true)
        .run()
}
