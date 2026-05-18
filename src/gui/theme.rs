use iced::font::Family;
use iced::theme::Palette;
use iced::widget::{button, column, container, rule, text, text_input, Text};
use iced::{Background, Border, Color, Element, Font, Theme};

use crate::engine::script::to_devanagari;

pub const BG: Color = Color { r: 0.200, g: 0.180, b: 0.160, a: 1.0 };
pub const SURFACE: Color = Color { r: 0.250, g: 0.230, b: 0.210, a: 1.0 };
pub const BORDER_COLOR: Color = Color { r: 0.350, g: 0.320, b: 0.290, a: 1.0 };
pub const TEXT_COLOR: Color = Color { r: 0.900, g: 0.880, b: 0.850, a: 1.0 };
pub const TEXT_SECONDARY: Color = Color { r: 0.620, g: 0.580, b: 0.530, a: 1.0 };
pub const ACCENT: Color = Color { r: 0.780, g: 0.530, b: 0.280, a: 1.0 };
pub const ACCENT_LIGHT: Color = Color { r: 0.280, g: 0.255, b: 0.230, a: 1.0 };
pub const ERROR_BG: Color = Color { r: 0.300, g: 0.180, b: 0.180, a: 1.0 };
pub const ERROR_TEXT: Color = Color { r: 0.950, g: 0.450, b: 0.450, a: 1.0 };
pub const CLEAN_BG: Color = Color { r: 0.180, g: 0.280, b: 0.200, a: 1.0 };
pub const CLEAN_TEXT: Color = Color { r: 0.450, g: 0.850, b: 0.550, a: 1.0 };
pub const TRACE_BG: Color = Color { r: 0.230, g: 0.210, b: 0.190, a: 1.0 };

pub fn panini_theme() -> Theme {
    Theme::custom(
        "Panini",
        Palette {
            background: BG,
            text: TEXT_COLOR,
            primary: ACCENT,
            success: CLEAN_TEXT,
            warning: Color { r: 0.718, g: 0.494, b: 0.200, a: 1.0 },
            danger: ERROR_TEXT,
        },
    )
}

pub fn latin() -> Font {
    Font {
        family: Family::Name("Noto Sans"),
        ..Font::DEFAULT
    }
}

pub fn devanagari() -> Font {
    Font {
        family: Family::Name("Noto Sans Devanagari"),
        ..Font::DEFAULT
    }
}

pub fn dual_script<'a, M: 'a>(iast: &str, sz: f32) -> Element<'a, M> {
    let deva = to_devanagari(iast);
    column![
        text(deva).font(devanagari()).size(sz),
        text(iast.to_string())
            .font(latin())
            .size(sz - 4.0)
            .color(TEXT_SECONDARY),
    ]
    .spacing(1)
    .into()
}

pub fn accent_btn(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered | button::Status::Pressed => Color {
            r: ACCENT.r * 0.85,
            g: ACCENT.g * 0.85,
            b: ACCENT.b * 0.85,
            a: 1.0,
        },
        button::Status::Disabled => Color {
            a: 0.5,
            ..ACCENT
        },
        _ => ACCENT,
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: Color::WHITE,
        border: Border {
            radius: 4.0.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

pub fn tab_active(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(ACCENT)),
        text_color: Color::WHITE,
        border: Border {
            radius: 4.0.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

pub fn tab_inactive(_theme: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: match status {
            button::Status::Hovered => Some(Background::Color(ACCENT_LIGHT)),
            _ => None,
        },
        text_color: match status {
            button::Status::Hovered => TEXT_COLOR,
            _ => TEXT_SECONDARY,
        },
        border: Border {
            radius: 4.0.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

pub fn toggle_active(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(ACCENT)),
        text_color: Color::WHITE,
        border: Border {
            color: ACCENT,
            width: 1.0,
            radius: 4.0.into(),
        },
        ..button::Style::default()
    }
}

pub fn toggle_inactive(_theme: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(match status {
            button::Status::Hovered => ACCENT_LIGHT,
            _ => SURFACE,
        })),
        text_color: TEXT_SECONDARY,
        border: Border {
            color: BORDER_COLOR,
            width: 1.0,
            radius: 4.0.into(),
        },
        ..button::Style::default()
    }
}

pub fn card(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(SURFACE)),
        border: Border {
            color: BORDER_COLOR,
            width: 1.0,
            radius: 4.0.into(),
        },
        ..container::Style::default()
    }
}

pub fn header_cell(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(ACCENT_LIGHT)),
        border: Border {
            color: BORDER_COLOR,
            width: 0.5,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    }
}

pub fn grid_cell(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(SURFACE)),
        border: Border {
            color: BORDER_COLOR,
            width: 0.5,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    }
}

pub fn cell_btn(_theme: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(match status {
            button::Status::Hovered => ACCENT_LIGHT,
            _ => SURFACE,
        })),
        text_color: TEXT_COLOR,
        border: Border {
            color: BORDER_COLOR,
            width: 0.5,
            radius: 0.0.into(),
        },
        ..button::Style::default()
    }
}

pub fn trace_container(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(TRACE_BG)),
        border: Border {
            color: BORDER_COLOR,
            width: 1.0,
            radius: 4.0.into(),
        },
        ..container::Style::default()
    }
}

pub fn verdict_clean(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(CLEAN_BG)),
        border: Border {
            color: Color { r: 0.525, g: 0.937, b: 0.675, a: 1.0 },
            width: 1.0,
            radius: 6.0.into(),
        },
        ..container::Style::default()
    }
}

pub fn verdict_issues(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(ERROR_BG)),
        border: Border {
            color: Color { r: 0.988, g: 0.647, b: 0.647, a: 1.0 },
            width: 1.0,
            radius: 6.0.into(),
        },
        ..container::Style::default()
    }
}

pub fn tag_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(ACCENT_LIGHT)),
        border: Border {
            radius: 3.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

pub fn tab_rule(_theme: &Theme) -> rule::Style {
    rule::Style {
        color: BORDER_COLOR,
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }
}

pub fn input_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let palette = theme.extended_palette();
    let active = text_input::Style {
        background: Background::Color(SURFACE),
        border: Border {
            color: BORDER_COLOR,
            width: 1.0,
            radius: 4.0.into(),
        },
        icon: TEXT_SECONDARY,
        placeholder: TEXT_SECONDARY,
        value: TEXT_COLOR,
        selection: palette.primary.weak.color,
    };
    match status {
        text_input::Status::Active => active,
        text_input::Status::Hovered => text_input::Style {
            border: Border {
                color: ACCENT,
                ..active.border
            },
            ..active
        },
        text_input::Status::Focused { .. } => text_input::Style {
            border: Border {
                color: ACCENT,
                width: 2.0,
                ..active.border
            },
            ..active
        },
        text_input::Status::Disabled => text_input::Style {
            background: Background::Color(BG),
            value: TEXT_SECONDARY,
            ..active
        },
    }
}

pub fn label(s: &str) -> Text<'_> {
    text(s.to_uppercase())
        .size(11)
        .font(latin())
        .color(TEXT_SECONDARY)
}

pub fn close_btn(_theme: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: match status {
            button::Status::Hovered => Some(Background::Color(ACCENT_LIGHT)),
            _ => None,
        },
        text_color: TEXT_SECONDARY,
        border: Border {
            radius: 4.0.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}
