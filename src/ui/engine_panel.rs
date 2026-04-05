use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Gauge};

use crate::model::EngineMetrics;

pub fn draw(frame: &mut Frame, area: Rect, engines: &[EngineMetrics], pmu_available: bool) {
    let block = Block::default()
        .title(" Engine Utilization ")
        .borders(Borders::ALL);

    if !pmu_available {
        let msg = ratatui::widgets::Paragraph::new(
            "  Requires CAP_PERFMON or root (run: sudo setcap cap_perfmon+ep ./xetop)",
        )
        .style(Style::default().fg(Color::DarkGray))
        .block(block);
        frame.render_widget(msg, area);
        return;
    }

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if engines.is_empty() {
        return;
    }

    let constraints: Vec<Constraint> = engines
        .iter()
        .map(|_| Constraint::Length(1))
        .collect();

    let chunks = Layout::vertical(constraints).split(inner);

    for (i, engine) in engines.iter().enumerate() {
        if i >= chunks.len() {
            break;
        }

        let pct = engine.utilization_pct.clamp(0.0, 100.0);
        let color = utilization_color(pct);

        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(color))
            .label(format!(
                " {:>8} ({:>3}): {:5.1}%",
                engine.label, engine.name, pct
            ))
            .ratio(pct / 100.0);

        frame.render_widget(gauge, chunks[i]);
    }
}

fn utilization_color(pct: f64) -> Color {
    if pct >= 80.0 {
        Color::Red
    } else if pct >= 50.0 {
        Color::Yellow
    } else {
        Color::Green
    }
}
