use iced::font::Family;
use iced::widget::{column, text};
use iced::{Element, Font};

use crate::engine::script::to_devanagari;

pub fn latin() -> Font {
    Font { family: Family::Name("Noto Sans"), ..Font::DEFAULT }
}

pub fn devanagari() -> Font {
    Font { family: Family::Name("Noto Sans Devanagari"), ..Font::DEFAULT }
}

pub fn dual_script<'a, M: 'a>(iast: &str, sz: f32) -> Element<'a, M> {
    let deva = to_devanagari(iast);
    column![
        text(deva).font(devanagari()).size(sz),
        text(iast.to_string()).font(latin()).size(sz - 4.0),
    ]
    .spacing(1)
    .into()
}
