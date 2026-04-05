use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, Paragraph};

use crate::model::NpuState;

pub fn draw(frame: &mut Frame, area: Rect, npu: &NpuState) {
    let block = Block::default()
        .title(" NPU ")
        .borders(Borders::ALL);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::vertical([
        Constraint::Length(1), // freq
        Constraint::Length(1), // utilization gauge
        Constraint::Length(1), // memory
        Constraint::Length(1), // power state
    ])
    .split(inner);

    // Frequency
    let freq_line = Line::from(vec![
        Span::raw(" Freq: "),
        Span::styled(
            format!("{:>4}", npu.cur_freq_mhz),
            Style::default().fg(if npu.cur_freq_mhz > 0 {
                Color::Green
            } else {
                Color::DarkGray
            }),
        ),
        Span::raw(format!("/{} MHz", npu.max_freq_mhz)),
    ]);
    frame.render_widget(Paragraph::new(freq_line), chunks[0]);

    // Utilization gauge
    let util_pct = npu.utilization_pct.clamp(0.0, 100.0);
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(Color::Magenta))
        .label(format!(" Util: {util_pct:5.1}%"))
        .ratio(util_pct / 100.0);
    frame.render_widget(gauge, chunks[1]);

    // Memory
    let mem_gb = npu.memory_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    let mem_line = Line::from(vec![
        Span::raw(" Mem:  "),
        Span::styled(format!("{mem_gb:.1} GB"), Style::default().fg(Color::Yellow)),
    ]);
    frame.render_widget(Paragraph::new(mem_line), chunks[2]);

    // Power state
    let state_color = if npu.power_state.contains("D0") {
        Color::Green
    } else {
        Color::DarkGray
    };
    let state_line = Line::from(vec![
        Span::raw(" State: "),
        Span::styled(&npu.power_state, Style::default().fg(state_color)),
    ]);
    frame.render_widget(Paragraph::new(state_line), chunks[3]);
}
