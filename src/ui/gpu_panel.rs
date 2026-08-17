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
            .flat_map(|_| {
                [
                    ratatui::layout::Constraint::Length(1),
                    ratatui::layout::Constraint::Length(1),
                ]
            })
            .collect::<Vec<_>>(),
    )
    .split(inner);

    for (i, gt) in gpu.gts.iter().enumerate() {
        let row_freq = i * 2;
        let row_util = i * 2 + 1;

        if row_util >= chunks.len() {
            break;
        }

        // Frequency line
        let label = format!("GT{}", gt.id);
        let engines = if gt.id == 0 {
            "render/compute"
        } else {
            "media"
        };

        // Profile range (eff_freq to p0_freq) if available
        let mut freq_line = Line::from(vec![
            Span::styled(
                format!(" {label} ({engines}): "),
                Style::default().fg(Color::White),
            ),
        ]);

        if gt.eff_freq_mhz > 0 && gt.p0_freq_mhz > 0 {
            freq_line.spans.push(Span::raw(format!(
                "{}-{} ",
                gt.eff_freq_mhz, gt.p0_freq_mhz
            )));
        }

        freq_line.spans.push(Span::styled(
            format!("{:>4} MHz", gt.cur_freq_mhz),
            Style::default().fg(if gt.cur_freq_mhz > 0 {
                Color::Green
            } else {
                Color::DarkGray
            }),
        ));

        if !gt.power_profile.is_empty() {
            freq_line.spans.push(Span::raw(format!(" [{}]", gt.power_profile)));
        }
        frame.render_widget(Paragraph::new(freq_line), chunks[row_freq]);

        // Utilization gauge (100% - C6 idle residency)
        let util_pct = gt.utilization_pct.clamp(0.0, 100.0);
        let color = if util_pct >= 80.0 {
            Color::Red
        } else if util_pct >= 50.0 {
            Color::Yellow
        } else {
            Color::Green
        };
        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(color))
            .label(format!(" Util: {util_pct:5.1}%"))
            .ratio(util_pct / 100.0);
        frame.render_widget(gauge, chunks[row_util]);
    }
}
