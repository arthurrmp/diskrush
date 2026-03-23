use super::theme;
use crate::app::{App, AppState, View};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let shortcuts = match app.view {
        View::Settings => vec![
            shortcut("↑↓", "navigate"),
            shortcut("space", "toggle"),
            shortcut("←", "back"),
            shortcut("q", "quit"),
        ],
        View::History => vec![
            shortcut("↑↓", "browse"),
            shortcut("←→", "switch tab"),
            shortcut("q", "quit"),
        ],
        View::Benchmark => match &app.state {
            AppState::SelectDrive { .. } => {
                let mut s = vec![
                    shortcut("↑↓", "navigate"),
                    shortcut("enter", "select"),
                ];
                if !app.history.is_empty() {
                    s.push(shortcut("→", "history"));
                }
                s.push(shortcut("q", "quit"));
                s
            }
            AppState::Running { .. } => vec![shortcut("esc", "cancel"), shortcut("q", "quit")],
            AppState::Complete { .. } => {
                let mut s = vec![shortcut("r", "rerun")];
                if !app.history.is_empty() {
                    s.push(shortcut("→", "history"));
                }
                s.push(shortcut("esc", "back"));
                s.push(shortcut("q", "quit"));
                s
            }
        },
    };

    let spans: Vec<Span> = shortcuts.into_iter().flatten().collect();
    let footer =
        Paragraph::new(Line::from(spans)).style(Style::default().bg(theme::FOOTER_BG));
    frame.render_widget(footer, area);
}

fn shortcut<'a>(key: &'a str, action: &'a str) -> Vec<Span<'a>> {
    vec![
        Span::styled(
            format!(" {key} "),
            Style::default()
                .fg(Color::White)
                .bg(theme::BORDER)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!(" {action}  "), Style::default().fg(theme::TEXT_DIM)),
    ]
}
