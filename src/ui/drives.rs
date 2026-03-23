use super::theme;
use crate::drives::Drive;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, Padding};
use ratatui::Frame;

pub fn render(frame: &mut Frame, drives: &[Drive], selected: usize, area: Rect) {
    let items: Vec<ListItem> = drives
        .iter()
        .enumerate()
        .map(|(i, drive)| {
            let is_sel = i == selected;
            let size = drive.size_label();
            let path_str = drive.mount.display().to_string();

            let name_style = if is_sel {
                Style::default()
                    .fg(theme::TEXT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT_DIM)
            };

            let marker = if is_sel { "▸ " } else { "  " };
            let marker_style = Style::default().fg(if is_sel {
                theme::ACCENT_BLUE
            } else {
                theme::BORDER
            });

            let line = Line::from(vec![
                Span::styled(marker, marker_style),
                Span::styled(drive.name.clone(), name_style),
                Span::styled("  ", Style::default()),
                Span::styled(path_str, Style::default().fg(theme::TEXT_DIM)),
                Span::styled("  ", Style::default()),
                Span::styled(
                    size,
                    Style::default().fg(if is_sel {
                        theme::TEXT_DIM
                    } else {
                        theme::BORDER
                    }),
                ),
            ]);

            ListItem::new(vec![line, Line::raw("")]) // blank line between items
        })
        .collect();

    let block = theme::styled_block(" Select Drive ")
        .padding(Padding::new(2, 2, 1, 0));

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default());

    let mut state = ListState::default().with_selected(Some(selected));
    frame.render_stateful_widget(list, area, &mut state);
}
