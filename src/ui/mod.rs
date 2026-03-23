pub mod drives;
pub mod footer;
pub mod header;
pub mod results;
pub mod settings;
pub mod theme;

use crate::app::{App, AppState, View};
use ratatui::layout::{Alignment, Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders};
use ratatui::Frame;

pub fn draw(frame: &mut Frame, app: &App) {
    let full = if app.settings.fullscreen {
        frame.area()
    } else {
        theme::centered_box(frame.area(), theme::MAX_WIDTH, theme::MAX_HEIGHT)
    };

    let outer = Block::default()
        .title(" diskrush ")
        .title_style(
            Style::default()
                .fg(Color::White)
                .bg(theme::ACCENT_BLUE)
                .add_modifier(Modifier::BOLD),
        )
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::BORDER));
    let inner = outer.inner(full);
    frame.render_widget(outer, full);

    let [tab_area, main_area, footer_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(inner);

    header::render(frame, app, tab_area);

    let enabled = app.settings.enabled_tests();

    match app.view {
        View::Benchmark => match &app.state {
            AppState::SelectDrive { drives, selected } => {
                drives::render(frame, drives, *selected, main_area);
            }
            AppState::Running {
                current_test,
                progress,
                live_mbps,
                completed,
                ..
            } => {
                let path = app.display_path.display().to_string();
                results::render_benchmark(
                    frame,
                    &enabled,
                    completed,
                    Some(current_test),
                    *progress,
                    *live_mbps,
                    app.spinner_tick,
                    &app.drive_name,
                    &path,
                    app.settings.test_size_mb,
                    main_area,
                );
            }
            AppState::Complete { results } => {
                let path = app.display_path.display().to_string();
                results::render_benchmark(
                    frame,
                    &enabled,
                    results,
                    None,
                    0.0,
                    0.0,
                    0,
                    &app.drive_name,
                    &path,
                    app.settings.test_size_mb,
                    main_area,
                );
            }
        },
        View::History => {
            results::render_history(frame, app, main_area);
        }
        View::Settings => {
            settings::render(frame, &app.settings, main_area);
        }
    }

    footer::render(frame, app, footer_area);
}
