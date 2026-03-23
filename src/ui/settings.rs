use super::theme;
use crate::app::{Settings, SettingsField};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub fn render(frame: &mut Frame, settings: &Settings, area: Rect) {
    let block = theme::styled_block(" Settings ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::vertical([
        Constraint::Length(1), // padding
        Constraint::Length(1), // test size
        Constraint::Length(1), // fullscreen
        Constraint::Length(1), // blank
        Constraint::Length(1), // section header
        Constraint::Length(1), // seq write
        Constraint::Length(1), // seq read
        Constraint::Length(1), // rand write
        Constraint::Length(1), // rand read
        Constraint::Length(1), // blank
        Constraint::Length(1), // hint
        Constraint::Fill(1),
    ])
    .areas::<12>(inner);

    // Test size row
    let size_focused = settings.focused == SettingsField::TestSize;
    let size_line = Line::from(vec![
        Span::raw("  "),
        cursor(size_focused),
        Span::styled(
            " Test size    ",
            label_style(size_focused),
        ),
        Span::styled(
            format!("  {}  ", format_size(settings.test_size_mb)),
            if size_focused {
                Style::default()
                    .fg(Color::White)
                    .bg(theme::ACCENT_BLUE)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT)
            },
        ),
    ]);
    frame.render_widget(Paragraph::new(size_line), rows[1]);

    // Fullscreen row
    let fs_focused = settings.focused == SettingsField::Fullscreen;
    let fs_line = Line::from(vec![
        Span::raw("  "),
        cursor(fs_focused),
        Span::styled(
            " Fullscreen   ",
            label_style(fs_focused),
        ),
        checkbox(settings.fullscreen),
    ]);
    frame.render_widget(Paragraph::new(fs_line), rows[2]);

    // Section header
    let header = Line::from(vec![
        Span::raw("   "),
        Span::styled(
            "Tests",
            Style::default()
                .fg(theme::TEXT_DIM)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    frame.render_widget(Paragraph::new(header), rows[4]);

    // Toggle rows
    let toggles = [
        (SettingsField::SeqWrite, "Sequential Write", settings.seq_write),
        (SettingsField::SeqRead, "Sequential Read", settings.seq_read),
        (SettingsField::RandWrite, "Random Write 4K", settings.rand_write),
        (SettingsField::RandRead, "Random Read 4K", settings.rand_read),
    ];

    for (i, (field, label, enabled)) in toggles.iter().enumerate() {
        let focused = settings.focused == *field;
        let line = Line::from(vec![
            Span::raw("  "),
            cursor(focused),
            Span::styled(
                format!(" {label:<17}"),
                label_style(focused),
            ),
            checkbox(*enabled),
        ]);
        frame.render_widget(Paragraph::new(line), rows[5 + i]);
    }

    // Hint
    let hint = Line::from(vec![
        Span::raw("  "),
        Span::styled(
            "  space/enter to toggle  |  esc back",
            Style::default().fg(theme::TEXT_DIM),
        ),
    ]);
    frame.render_widget(Paragraph::new(hint), rows[10]);
}

fn cursor(focused: bool) -> Span<'static> {
    if focused {
        Span::styled(
            ">",
            Style::default()
                .fg(theme::ACCENT_BLUE)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::raw(" ")
    }
}

fn label_style(focused: bool) -> Style {
    if focused {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::TEXT)
    }
}

fn checkbox(enabled: bool) -> Span<'static> {
    if enabled {
        Span::styled(
            " [x]",
            Style::default().fg(theme::CHECK_GREEN),
        )
    } else {
        Span::styled(
            " [ ]",
            Style::default().fg(theme::TEXT_DIM),
        )
    }
}

fn format_size(mb: u64) -> String {
    if mb >= 1024 {
        format!("{} GB", mb / 1024)
    } else {
        format!("{mb} MB")
    }
}
