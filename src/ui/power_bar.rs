use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::model::RaplState;

pub fn draw(frame: &mut Frame, area: Rect, rapl: &RaplState) {
    let line = Line::from(vec![
        Span::styled(" Power: ", Style::default().fg(Color::White)),
        Span::styled("Pkg ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{:5.1}W", rapl.pkg_watts), Style::default().fg(Color::Yellow)),
        Span::raw("  "),
        Span::styled("Core ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{:5.1}W", rapl.core_watts), Style::default().fg(Color::Cyan)),
        Span::raw("  "),
        Span::styled("DRAM ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{:5.1}W", rapl.dram_watts), Style::default().fg(Color::Green)),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}
