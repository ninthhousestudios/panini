pub mod paradigm;
pub mod sandhi;
pub mod sutra;
pub mod theme;
pub mod verify;

use std::sync::Arc;

use iced::widget::{button, column, row, text};
use iced::{Element, Size, Task};

use crate::rule_cache::RuleCache;

const NOTO_DEVA: &[u8] = include_bytes!("../../fonts/NotoSansDevanagari-Regular.ttf");
const NOTO_SANS: &[u8] = include_bytes!("../../fonts/NotoSans-Regular.ttf");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Paradigm,
    Sandhi,
    Sutras,
    Verify,
}

struct State {
    cache: Arc<RuleCache>,
    active_tab: Tab,
    paradigm: paradigm::State,
    sandhi: sandhi::State,
    sutra: sutra::State,
    verify: verify::State,
}

#[derive(Debug, Clone)]
enum Message {
    SwitchTab(Tab),
    Paradigm(paradigm::Message),
    Sandhi(sandhi::Message),
    Sutra(sutra::Message),
    Verify(verify::Message),
}

fn boot(cache: Arc<RuleCache>) -> (State, Task<Message>) {
    let state = State {
        cache,
        active_tab: Tab::Paradigm,
        paradigm: paradigm::State::default(),
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
        Message::Paradigm(msg) => {
            paradigm::update(&state.cache, &mut state.paradigm, msg).map(Message::Paradigm)
        }
        Message::Sandhi(msg) => {
            sandhi::update(&state.cache, &mut state.sandhi, msg).map(Message::Sandhi)
        }
        Message::Sutra(msg) => {
            sutra::update(&mut state.sutra, msg).map(Message::Sutra)
        }
        Message::Verify(msg) => {
            verify::update(&mut state.verify, msg).map(Message::Verify)
        }
    }
}

fn view(state: &State) -> Element<'_, Message> {
    let tab_bar = row![
        tab_button("Paradigms", Tab::Paradigm, state.active_tab),
        tab_button("Sandhi", Tab::Sandhi, state.active_tab),
        tab_button("Sūtras", Tab::Sutras, state.active_tab),
        tab_button("Verification", Tab::Verify, state.active_tab),
    ]
    .spacing(4);

    let body: Element<'_, Message> = match state.active_tab {
        Tab::Paradigm => paradigm::view(&state.paradigm).map(Message::Paradigm),
        Tab::Sandhi => sandhi::view(&state.sandhi).map(Message::Sandhi),
        Tab::Sutras => sutra::view(&state.sutra).map(Message::Sutra),
        Tab::Verify => verify::view(&state.verify).map(Message::Verify),
    };

    column![tab_bar, body]
        .spacing(12)
        .padding(20)
        .into()
}

fn tab_button(label: &str, tab: Tab, active: Tab) -> Element<'_, Message> {
    let t = if tab == active {
        text(format!("> {label}")).size(15).font(theme::latin())
    } else {
        text(format!("  {label}")).size(15).font(theme::latin())
    };
    button(t).on_press(Message::SwitchTab(tab)).into()
}

pub fn run(cache: Arc<RuleCache>) -> iced::Result {
    iced::application(move || boot(cache.clone()), update, view)
        .title("Panini")
        .font(NOTO_DEVA)
        .font(NOTO_SANS)
        .default_font(theme::latin())
        .window_size(Size::new(1100.0, 800.0))
        .centered()
        .antialiasing(true)
        .run()
}
