use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders};

pub const TEXT: Color = Color::Rgb(220, 220, 220);
pub const TEXT_DIM: Color = Color::Rgb(120, 120, 130);
pub const BORDER: Color = Color::Rgb(60, 60, 70);

pub const ACCENT_BLUE: Color = Color::Rgb(99, 102, 241);
pub const GAUGE_BG: Color = Color::Rgb(30, 30, 40);

pub const FOOTER_BG: Color = Color::Rgb(25, 25, 35);
pub const TAB_INACTIVE_BG: Color = Color::Rgb(40, 40, 50);
pub const CHECK_GREEN: Color = Color::Rgb(100, 220, 100);

pub const MAX_WIDTH: u16 = 76;
pub const MAX_HEIGHT: u16 = 24;

pub fn styled_block(title: &str) -> Block<'_> {
    Block::default()
        .title(title)
        .title_style(Style::default().fg(TEXT).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(BORDER))
}

/// Center a rect both horizontally and vertically with max dimensions.
pub fn centered_box(area: Rect, max_width: u16, max_height: u16) -> Rect {
    let w = max_width.min(area.width);
    let h = max_height.min(area.height);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect {
        x,
        y,
        width: w,
        height: h,
    }
}
