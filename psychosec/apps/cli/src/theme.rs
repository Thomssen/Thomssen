use ratatui::style::{Color, Modifier, Style};

pub const BG: Color = Color::Rgb(0x08, 0x09, 0x0B);
pub const FG: Color = Color::Rgb(0xF2, 0xF2, 0xF2);
pub const DIM: Color = Color::Rgb(0x85, 0x85, 0x85);
pub const BORDER: Color = Color::Rgb(0x29, 0x2B, 0x2F);
pub const ACCENT: Color = Color::Rgb(0x87, 0xBD, 0xBB);
pub const OK: Color = Color::Rgb(0x91, 0xC5, 0xA5);
pub const ERR: Color = Color::Rgb(0xD7, 0x85, 0x85);
pub const WARN: Color = Color::Rgb(0xD6, 0xBF, 0x8A);

pub fn base() -> Style {
    Style::default().fg(FG).bg(BG)
}
pub fn dim() -> Style {
    base().fg(DIM)
}
pub fn accent() -> Style {
    base().fg(ACCENT)
}
pub fn bold() -> Style {
    base().add_modifier(Modifier::BOLD)
}
pub fn ok() -> Style {
    base().fg(OK)
}
pub fn err() -> Style {
    base().fg(ERR)
}
pub fn warn() -> Style {
    base().fg(WARN)
}
pub fn border() -> Style {
    base().fg(BORDER)
}
