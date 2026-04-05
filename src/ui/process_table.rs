use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Row, Table};

use crate::model::ProcessGpuUsage;

pub fn draw(frame: &mut Frame, area: Rect, processes: &[ProcessGpuUsage], selected: usize) {
    let header_cells = ["PID", "COMMAND", "GTT", "RCS", "VCS", "CCS", "BCS", "VECS"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1);

    let rows = processes.iter().enumerate().map(|(i, p)| {
        let gtt_str = format_memory(p.gtt_kb);

        let mut cells = vec![
            Cell::from(format!("{:>6}", p.pid)),
            Cell::from(format!("{:<14}", truncate(&p.command, 14))),
            Cell::from(format!("{:>7}", gtt_str)),
        ];

        // Engine columns in order: rcs, vcs, ccs, bcs, vecs
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
                .title(format!(" Processes ({}) ", processes.len()))
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
        s
    } else {
        &s[..max]
    }
}
