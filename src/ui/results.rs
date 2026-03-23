use super::theme;
use crate::app::App;
use crate::bench::{TestKind, TestResult};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

const WRITE_COLOR: Color = Color::Rgb(129, 140, 248);
const READ_COLOR: Color = Color::Rgb(52, 211, 153);
const LABEL_W: usize = 10;
const VALUE_W: usize = 14;

enum RowState<'a> {
    Pending,
    Running { progress: f64, live_mbps: f64, spinner_tick: usize },
    Done(&'a TestResult),
}

#[allow(clippy::too_many_arguments)]
pub fn render_benchmark(
    frame: &mut Frame,
    enabled: &[TestKind],
    completed: &[(TestKind, TestResult)],
    current_test: Option<&TestKind>,
    progress: f64,
    live_mbps: f64,
    spinner_tick: usize,
    drive_name: &str,
    path: &str,
    size_mb: u64,
    area: Rect,
) {
    let seq: Vec<_> = enabled.iter().filter(|k| k.is_sequential()).copied().collect();
    let rnd: Vec<_> = enabled.iter().filter(|k| !k.is_sequential()).copied().collect();

    // Global max across all tests for consistent bar scaling
    let global_max = enabled
        .iter()
        .filter_map(|kind| {
            if let Some((_, r)) = completed.iter().find(|(k, _)| k == kind) {
                Some(r.throughput_mbps)
            } else if current_test == Some(kind) {
                Some(live_mbps)
            } else {
                None
            }
        })
        .fold(0.0_f64, f64::max);

    // Each section: 2 (border) + 1 (top padding) + tests * 2 (bar + blank)
    let section_h = |n: usize| -> u16 { (2 + 1 + n * 2) as u16 };

    let (sections, sections_h): (Vec<(&[TestKind], &str)>, u16) = match (seq.is_empty(), rnd.is_empty()) {
        (false, false) => (
            vec![(&seq[..], " Sequential "), (&rnd[..], " Random 4K ")],
            section_h(seq.len()) + section_h(rnd.len()),
        ),
        (false, true) => (vec![(&seq[..], " Sequential ")], section_h(seq.len())),
        (true, false) => (vec![(&rnd[..], " Random 4K ")], section_h(rnd.len())),
        (true, true) => return,
    };

    let header_h: u16 = 2; // path/size line + blank
    let total_h = header_h + sections_h;

    // Center vertically
    let y_offset = area.height.saturating_sub(total_h) / 2;
    let centered = Rect {
        x: area.x,
        y: area.y + y_offset,
        width: area.width,
        height: total_h.min(area.height),
    };

    let [header_area, sections_area] = Layout::vertical([
        Constraint::Length(header_h),
        Constraint::Fill(1),
    ])
    .areas(centered);

    // Render path + size header
    let size_str = if size_mb >= 1024 {
        format!("{} GB", size_mb / 1024)
    } else {
        format!("{size_mb} MB")
    };
    let sep = Span::styled("  ·  ", Style::default().fg(theme::BORDER));
    let header_line = Line::from(vec![
        Span::styled(
            format!("  {drive_name}"),
            Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD),
        ),
        sep.clone(),
        Span::styled(path, Style::default().fg(theme::TEXT_DIM)),
        sep,
        Span::styled(size_str, Style::default().fg(theme::TEXT_DIM)),
    ]);
    frame.render_widget(Paragraph::new(header_line), header_area);

    let constraints: Vec<Constraint> = sections.iter().map(|(tests, _)| Constraint::Length(section_h(tests.len()))).collect();
    let areas = Layout::vertical(constraints).split(sections_area);

    for (i, (tests, title)) in sections.iter().enumerate() {
        render_section(frame, tests, title, completed, current_test, progress, live_mbps, spinner_tick, global_max, areas[i]);
    }
}

pub fn render_history(frame: &mut Frame, app: &App, area: Rect) {
    use ratatui::widgets::{Block, BorderType, Borders};

    if app.history.is_empty() {
        let msg = Paragraph::new(Line::from(Span::styled(
            "No history yet",
            Style::default().fg(theme::TEXT_DIM),
        )))
        .alignment(Alignment::Center);
        let centered = Rect {
            x: area.x,
            y: area.y + area.height / 2,
            width: area.width,
            height: 1,
        };
        frame.render_widget(msg, centered);
        return;
    }

    let card_h: u16 = 5; // border + seq + separator + 4k + border
    let max_visible = area.height as usize / card_h as usize;
    let scroll = app.history_idx.saturating_sub(max_visible.saturating_sub(1));

    let mut y = area.y;
    for (i, entry) in app.history.iter().enumerate().skip(scroll) {
        if y + card_h > area.y + area.height {
            break;
        }

        let is_sel = i == app.history_idx;
        let border_color = if is_sel {
            theme::ACCENT_BLUE
        } else {
            theme::BORDER
        };
        let emphasis_style = if is_sel {
            Style::default()
                .fg(theme::TEXT)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::TEXT_DIM)
        };
        let label_style = if is_sel {
            Style::default().fg(theme::TEXT_DIM)
        } else {
            Style::default().fg(theme::BORDER)
        };

        let card_area = Rect {
            x: area.x,
            y,
            width: area.width,
            height: card_h,
        };

        let seq_w = find_speed(&entry.results, TestKind::SeqWrite);
        let seq_r = find_speed(&entry.results, TestKind::SeqRead);
        let rnd_w = find_speed(&entry.results, TestKind::RandWrite);
        let rnd_r = find_speed(&entry.results, TestKind::RandRead);

        let sep_style = Style::default().fg(theme::BORDER);
        let inner_w = card_area.width.saturating_sub(2) as usize;
        let sep_line = "─".repeat(inner_w);

        let block = Block::default()
            .title(format!(" {} · {} ", entry.drive, entry.label))
            .title_style(emphasis_style)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color));

        let lines = vec![
            Line::from(vec![
                Span::styled("  SEQ   ", label_style),
                Span::styled("Write ", label_style),
                Span::styled(format!("{seq_w:>5.0} MB/s"), emphasis_style),
                Span::styled("     Read ", label_style),
                Span::styled(format!("{seq_r:>5.0} MB/s"), emphasis_style),
            ]),
            Line::from(Span::styled(sep_line, sep_style)),
            Line::from(vec![
                Span::styled("  4K    ", label_style),
                Span::styled("Write ", label_style),
                Span::styled(format!("{rnd_w:>5.0} MB/s"), emphasis_style),
                Span::styled("     Read ", label_style),
                Span::styled(format!("{rnd_r:>5.0} MB/s"), emphasis_style),
            ]),
        ];

        let para = Paragraph::new(lines).block(block);
        frame.render_widget(para, card_area);

        y += card_h;
    }
}

fn find_speed(results: &[(TestKind, TestResult)], kind: TestKind) -> f64 {
    results
        .iter()
        .find(|(k, _)| *k == kind)
        .map(|(_, r)| r.throughput_mbps)
        .unwrap_or(0.0)
}

#[allow(clippy::too_many_arguments)]
fn render_section(
    frame: &mut Frame,
    tests: &[TestKind],
    title: &str,
    completed: &[(TestKind, TestResult)],
    current_test: Option<&TestKind>,
    progress: f64,
    live_mbps: f64,
    spinner_tick: usize,
    max_mbps: f64,
    area: Rect,
) {
    let inner_w = area.width.saturating_sub(2) as usize;
    let bar_max = inner_w.saturating_sub(LABEL_W + VALUE_W + 2);

    // Determine row states
    let rows: Vec<(TestKind, RowState)> = tests
        .iter()
        .map(|&kind| {
            if let Some((_, result)) = completed.iter().find(|(k, _)| *k == kind) {
                (kind, RowState::Done(result))
            } else if current_test == Some(&kind) {
                (kind, RowState::Running { progress, live_mbps, spinner_tick })
            } else {
                (kind, RowState::Pending)
            }
        })
        .collect();

    let mut lines: Vec<Line> = vec![Line::raw("")];

    for (kind, state) in &rows {
        let label = if kind.is_write() { "Write" } else { "Read" };
        let bar_color = if kind.is_write() {
            WRITE_COLOR
        } else {
            READ_COLOR
        };

        match state {
            RowState::Done(result) => {
                let mbps = result.throughput_mbps;
                let bar_len = if max_mbps > 0.0 {
                    ((mbps / max_mbps) * bar_max as f64).round().max(1.0) as usize
                } else {
                    1
                };

                let bar = "█".repeat(bar_len);
                let remaining = bar_max.saturating_sub(bar_len);
                let dot_pairs = remaining / 2;
                let extra = remaining % 2;
                let leader = format!("{}{}", " ".repeat(extra), " ·".repeat(dot_pairs));

                let raw_value = format!("{mbps:.0} MB/s");
                let value = format!("{raw_value:>VALUE_W$}");

                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {label:<7} "),
                        Style::default().fg(theme::TEXT_DIM),
                    ),
                    Span::styled(bar, Style::default().fg(bar_color)),
                    Span::styled(leader, Style::default().fg(theme::TEXT_DIM)),
                    Span::styled(
                        value,
                        Style::default()
                            .fg(theme::TEXT)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                ]));
            }
            RowState::Running { progress, live_mbps, spinner_tick } => {
                let ratio = progress.clamp(0.0, 1.0);

                // Bar length based on live speed relative to completed results
                let bar_len = if max_mbps > 0.0 && *live_mbps > 0.0 {
                    ((*live_mbps / max_mbps) * bar_max as f64).round().max(1.0) as usize
                } else if *live_mbps > 0.0 {
                    // No completed results yet — show proportional to bar_max
                    (ratio * bar_max as f64).round().max(1.0) as usize
                } else {
                    0
                };
                let bar_len = bar_len.min(bar_max);
                let remaining = bar_max.saturating_sub(bar_len);

                // Split background into progress track and remaining
                let progress_len = (ratio * bar_max as f64).round() as usize;
                let progress_bg_len = progress_len.saturating_sub(bar_len).min(remaining);
                let rest_bg_len = remaining.saturating_sub(progress_bg_len);

                let bar = "█".repeat(bar_len);
                let progress_bg = "░".repeat(progress_bg_len);
                let rest_bg = "░".repeat(rest_bg_len);

                // Pulse the bar color between bright and dim
                let pulse = (*spinner_tick % 12) as f64 / 12.0;
                let wave = (pulse * std::f64::consts::TAU).sin() * 0.5 + 0.5; // 0.0..1.0
                let pulsed_color = pulse_color(bar_color, wave);
                let progress_track = dim_color(bar_color, 0.25);

                let speed_str = if *live_mbps > 0.0 {
                    format!("{live_mbps:.0} MB/s")
                } else {
                    format!("{:>3.0}%", ratio * 100.0)
                };
                let value = format!("{speed_str:>VALUE_W$}");

                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {label:<7} "),
                        Style::default().fg(bar_color),
                    ),
                    Span::styled(bar, Style::default().fg(pulsed_color)),
                    Span::styled(progress_bg, Style::default().fg(progress_track)),
                    Span::styled(rest_bg, Style::default().fg(theme::GAUGE_BG)),
                    Span::styled(
                        value,
                        Style::default().fg(bar_color),
                    ),
                    Span::raw("  "),
                ]));
            }
            RowState::Pending => {
                let dots = "·".repeat(bar_max);
                let value = format!("{:>VALUE_W$}", "—");

                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {label:<7} "),
                        Style::default().fg(theme::BORDER),
                    ),
                    Span::styled(dots, Style::default().fg(theme::BORDER)),
                    Span::styled(
                        value,
                        Style::default().fg(theme::BORDER),
                    ),
                    Span::raw("  "),
                ]));
            }
        }
        lines.push(Line::raw(""));
    }

    let paragraph = Paragraph::new(lines).block(theme::styled_block(title));
    frame.render_widget(paragraph, area);
}

fn pulse_color(base: Color, t: f64) -> Color {
    if let Color::Rgb(r, g, b) = base {
        let dim = 0.45;
        let bright = 1.0;
        let factor = dim + (bright - dim) * t;
        Color::Rgb(
            (r as f64 * factor).min(255.0) as u8,
            (g as f64 * factor).min(255.0) as u8,
            (b as f64 * factor).min(255.0) as u8,
        )
    } else {
        base
    }
}

fn dim_color(base: Color, factor: f64) -> Color {
    if let Color::Rgb(r, g, b) = base {
        Color::Rgb(
            (r as f64 * factor) as u8,
            (g as f64 * factor) as u8,
            (b as f64 * factor) as u8,
        )
    } else {
        base
    }
}
