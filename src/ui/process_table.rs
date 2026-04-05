use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Row, Table};

use crate::app::SortColumn;
use crate::model::ProcessGpuUsage;

pub fn draw(
    frame: &mut Frame,
    area: Rect,
    processes: &[ProcessGpuUsage],
    selected: usize,
    sort_column: SortColumn,
    sort_reverse: bool,
) {
    let sort_indicator = if sort_reverse { "▼" } else { "▲" };

    let header_names = [
        ("PID", SortColumn::Pid),
        ("COMMAND", SortColumn::Command),
        ("GTT", SortColumn::Gtt),
        ("RCS", SortColumn::Rcs),
        ("VCS", SortColumn::Vcs),
        ("CCS", SortColumn::Ccs),
        ("BCS", SortColumn::Bcs),
        ("VECS", SortColumn::Vecs),
    ];

    let header_cells = header_names.iter().map(|(name, col)| {
        let label = if *col == sort_column {
            format!("{name}{sort_indicator}")
        } else {
            name.to_string()
        };
        let style = if *col == sort_column {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        };
        Cell::from(label).style(style)
    });
    let header = Row::new(header_cells).height(1);

    let rows = processes.iter().enumerate().map(|(i, p)| {
        let gtt_str = format_memory(p.gtt_kb);

        let mut cells = vec![
            Cell::from(format!("{:>6}", p.pid)),
            Cell::from(format!("{:<14}", truncate(&p.command, 14))),
            Cell::from(format!("{:>7}", gtt_str)),
        ];

        for name in &["rcs", "vcs", "ccs", "bcs", "vecs"] {
            let pct = p
                .engine_utils
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, v)| *v)
                .unwrap_or(0.0);

            let color = if pct >= 50.0 {
                Color::Red
            } else if pct >= 10.0 {
                Color::Yellow
            } else {
                Color::White
            };

            cells.push(Cell::from(format!("{pct:5.1}%")).style(Style::default().fg(color)));
        }

        let style = if i == selected {
            Style::default().bg(Color::DarkGray)
        } else {
            Style::default()
        };

        Row::new(cells).style(style)
    });

    let widths = [
        ratatui::layout::Constraint::Length(7),  // PID
        ratatui::layout::Constraint::Length(15), // COMMAND
        ratatui::layout::Constraint::Length(8),  // GTT
        ratatui::layout::Constraint::Length(7),  // RCS
        ratatui::layout::Constraint::Length(7),  // VCS
        ratatui::layout::Constraint::Length(7),  // CCS
        ratatui::layout::Constraint::Length(7),  // BCS
        ratatui::layout::Constraint::Length(7),  // VECS
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .title(format!(
                    " Processes ({}) [s:sort r:reverse] ",
                    processes.len()
                ))
                .borders(Borders::ALL),
        );

    frame.render_widget(table, area);
}

fn format_memory(kb: u64) -> String {
    if kb >= 1_048_576 {
        format!("{:.1}G", kb as f64 / 1_048_576.0)
    } else if kb >= 1024 {
        format!("{}M", kb / 1024)
    } else {
        format!("{}K", kb)
    }
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}
