use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, Paragraph};

use crate::model::GpuState;

pub fn draw(frame: &mut Frame, area: Rect, gpu: &GpuState) {
    let block = Block::default()
        .title(" Xe GPU ")
        .borders(Borders::ALL);

    if gpu.gts.is_empty() {
        let msg = Paragraph::new("  No Xe GPU detected").block(block);
        frame.render_widget(msg, area);
        return;
    }

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = ratatui::layout::Layout::vertical(
        gpu.gts
            .iter()
            .flat_map(|_| [ratatui::layout::Constraint::Length(1), ratatui::layout::Constraint::Length(1)])
            .collect::<Vec<_>>(),
    )
    .split(inner);

    for (i, gt) in gpu.gts.iter().enumerate() {
        let row_freq = i * 2;
        let row_c6 = i * 2 + 1;

        if row_c6 >= chunks.len() {
            break;
        }

        // Frequency line
        let label = format!("GT{}", gt.id);
        let engines = if gt.id == 0 {
            "render/compute"
        } else {
            "media"
        };
        let freq_line = Line::from(vec![
            Span::styled(format!(" {label} ({engines}): "), Style::default().fg(Color::White)),
            Span::styled(
                format!("{:>4}", gt.act_freq_mhz),
                Style::default().fg(if gt.act_freq_mhz > 0 {
                    Color::Green
                } else {
                    Color::DarkGray
                }),
            ),
            Span::raw(format!("/{} MHz", gt.max_freq_mhz)),
            Span::styled(
                format!("  [{}]", gt.idle_status),
                Style::default().fg(Color::DarkGray),
            ),
        ]);
        frame.render_widget(Paragraph::new(freq_line), chunks[row_freq]);

        // C6 residency gauge
        let c6_pct = gt.c6_residency_pct.clamp(0.0, 100.0);
        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(Color::Blue))
            .label(format!(" C6: {c6_pct:5.1}%"))
            .ratio(c6_pct / 100.0);
        frame.render_widget(gauge, chunks[row_c6]);
    }
}
