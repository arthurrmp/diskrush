use super::theme;
use crate::app::{App, View};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let tab_style = |active: bool| -> (Color, Color) {
        if active {
            (Color::White, theme::ACCENT_BLUE)
        } else {
            (theme::TEXT_DIM, theme::TAB_INACTIVE_BG)
        }
    };

    let (bench_fg, bench_bg) = tab_style(app.view == View::Benchmark);
    let (hist_fg, hist_bg) = tab_style(app.view == View::History);
    let (set_fg, set_bg) = tab_style(app.view == View::Settings);

    // Top separator
    let sep = "─".repeat(area.width as usize);
    let sep_line = Line::from(Span::styled(&sep, Style::default().fg(theme::BORDER)));

    // Tabs + context line
    let tabs: Vec<Span> = vec![
        Span::raw("  "),
        Span::styled(
            " Benchmark ",
            Style::default()
                .fg(bench_fg)
                .bg(bench_bg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            " History ",
            Style::default()
                .fg(hist_fg)
                .bg(hist_bg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            " Settings ",
            Style::default()
                .fg(set_fg)
                .bg(set_bg)
                .add_modifier(Modifier::BOLD),
        ),
    ];
    let lines = vec![
        sep_line.clone(),
        Line::from(tabs),
        sep_line,
    ];

    let header = Paragraph::new(lines);
    frame.render_widget(header, area);
}
