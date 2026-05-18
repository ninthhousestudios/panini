use iced::font::Family;
use iced::widget::{column, container, row, rule, scrollable, text};
use iced::{Center, Element, Fill, Font, Size, Task};

const NOTO_DEVANAGARI: &[u8] = include_bytes!("../fonts/NotoSansDevanagari-Regular.ttf");
const NOTO_MALAYALAM: &[u8] = include_bytes!("../fonts/NotoSansMalayalam-Regular.ttf");
const NOTO_SANS: &[u8] = include_bytes!("../fonts/NotoSans-Regular.ttf");

fn dev() -> Font {
    Font { family: Family::Name("Noto Sans Devanagari"), ..Font::DEFAULT }
}

fn mal() -> Font {
    Font { family: Family::Name("Noto Sans Malayalam"), ..Font::DEFAULT }
}

fn lat() -> Font {
    Font { family: Family::Name("Noto Sans"), ..Font::DEFAULT }
}

#[derive(Default)]
struct State;

#[derive(Debug, Clone)]
enum Message {}

fn update(_state: &mut State, _message: Message) {}

fn view(_state: &State) -> Element<'_, Message> {
    let sz: f32 = 22.0;
    let hdr_sz: f32 = 18.0;
    let cell_w: f32 = 180.0;
    let label_w: f32 = 100.0;

    let content = column![
        text("Panini — Script Rendering Test").size(28).font(lat()),
        rule::horizontal(1),
        // --- Devanāgarī ---
        section_header("Devanāgarī — Conjuncts"),
        text("क्ष  त्र  ज्ञ  श्र  द्ध  क्त  ष्ट  ङ्क").font(dev()).size(sz),
        section_header("Devanāgarī — Words"),
        text("देवः  देवेषु  संस्कृतम्  धर्मः  ऋषिः  क्षत्रियः").font(dev()).size(sz),
        rule::horizontal(1),
        // --- Malayalam ---
        section_header("Malayalam — Conjuncts"),
        text("ക്ഷ  ത്ര  ജ്ഞ  ശ്ര  ദ്ധ  ക്ത  ഷ്ട  ങ്ക").font(mal()).size(sz),
        section_header("Malayalam — Words"),
        text("ദേവൻ  സംസ്കൃതം  മലയാളം  ധർമ്മം  ഋഷി  ക്ഷത്രിയൻ").font(mal()).size(sz),
        rule::horizontal(1),
        // --- IAST ---
        section_header("IAST — Diacritics"),
        text("devaḥ  deveṣu  saṃskṛtam  dharmaḥ  ṛṣiḥ  kṣatriyaḥ").font(lat()).size(sz),
        text("ā ī ū ṛ ṝ ḷ ṃ ḥ ñ ṅ ś ṣ ṭ ḍ ṇ").font(lat()).size(sz),
        rule::horizontal(1),
        // --- Mixed-script lines (font fallback test) ---
        section_header("Mixed Script — Explicit Fonts"),
        mixed_row("देवः", "ദേവൻ", "devaḥ", "nominative singular", sz),
        mixed_row("देवम्", "ദേവനെ", "devam", "accusative singular", sz),
        mixed_row("देवेन", "ദേവനാൽ", "devena", "instrumental singular", sz),
        rule::horizontal(1),
        // --- Font fallback test: single text widget, mixed scripts ---
        section_header("Font Fallback — Single Text Widget"),
        text("देवः / ദേവൻ / devaḥ — nominative singular").size(sz),
        text("संस्कृतम् / സംസ്കൃതം / saṃskṛtam").size(sz),
        text("क्षत्रियः / ക്ഷത്രിയൻ / kṣatriyaḥ").size(sz),
        rule::horizontal(1),
        // --- Paradigm table ---
        section_header("Paradigm Table — deva (m.)"),
        table_header(label_w, cell_w, hdr_sz),
        table_row("Nom.", "देवः", "ദേവൻ", "devaḥ", label_w, cell_w, sz),
        table_row("Acc.", "देवम्", "ദേവനെ", "devam", label_w, cell_w, sz),
        table_row("Inst.", "देवेन", "ദേവനാൽ", "devena", label_w, cell_w, sz),
        table_row("Dat.", "देवाय", "ദേവനു", "devāya", label_w, cell_w, sz),
        table_row("Abl.", "देवात्", "ദേവനിൽനിന്ന്", "devāt", label_w, cell_w, sz),
        table_row("Gen.", "देवस्य", "ദേവന്റെ", "devasya", label_w, cell_w, sz),
        table_row("Loc.", "देवे", "ദേവനിൽ", "deve", label_w, cell_w, sz),
        table_row("Voc.", "देव", "ദേവ", "deva", label_w, cell_w, sz),
    ]
    .spacing(8)
    .padding(24);

    scrollable(container(content).center_x(Fill)).into()
}

fn section_header(label: &str) -> Element<'_, Message> {
    text(label.to_string()).size(16).font(lat()).into()
}

fn mixed_row<'a>(
    devanagari: &'a str,
    malayalam: &'a str,
    iast: &'a str,
    gloss: &'a str,
    sz: f32,
) -> Element<'a, Message> {
    row![
        text(devanagari.to_string()).font(dev()).size(sz).width(140),
        text(malayalam.to_string()).font(mal()).size(sz).width(160),
        text(iast.to_string()).font(lat()).size(sz).width(120),
        text(format!("— {gloss}")).font(lat()).size(sz - 4.0),
    ]
    .spacing(12)
    .align_y(Center)
    .into()
}

fn table_header<'a>(label_w: f32, cell_w: f32, sz: f32) -> Element<'a, Message> {
    row![
        text("Case").size(sz).font(lat()).width(label_w),
        text("Devanāgarī").size(sz).font(lat()).width(cell_w),
        text("Malayalam").size(sz).font(lat()).width(cell_w),
        text("IAST").size(sz).font(lat()).width(cell_w),
    ]
    .spacing(8)
    .into()
}

fn table_row<'a>(
    case: &'a str,
    devanagari: &'a str,
    malayalam: &'a str,
    iast: &'a str,
    label_w: f32,
    cell_w: f32,
    sz: f32,
) -> Element<'a, Message> {
    row![
        text(case.to_string()).size(sz).font(lat()).width(label_w),
        text(devanagari.to_string()).size(sz).font(dev()).width(cell_w),
        text(malayalam.to_string()).size(sz).font(mal()).width(cell_w),
        text(iast.to_string()).size(sz).font(lat()).width(cell_w),
    ]
    .spacing(8)
    .align_y(Center)
    .into()
}

fn boot() -> (State, Task<Message>) {
    (State, Task::none())
}

fn main() -> iced::Result {
    iced::application(boot, update, view)
        .title("Panini — Script Rendering Test")
        .font(NOTO_DEVANAGARI)
        .font(NOTO_MALAYALAM)
        .font(NOTO_SANS)
        .default_font(lat())
        .window_size(Size::new(950.0, 800.0))
        .centered()
        .antialiasing(true)
        .run()
}
