use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::vertical([
        Constraint::Length(3), // title bar
        Constraint::Min(1),   // main content placeholder
    ])
    .split(area);

    // Title bar
    let title = Paragraph::new(format!(
        " xetop — Intel Xe GPU & NPU Monitor | tick: {}",
        app.tick_count
    ))
    .style(Style::default().fg(Color::Cyan))
    .block(Block::default().borders(Borders::BOTTOM));

    frame.render_widget(title, chunks[0]);

    // Placeholder
    let placeholder = Paragraph::new("  Waiting for data collectors... (press 'q' to quit)")
        .block(
            Block::default()
                .title(" Status ")
                .borders(Borders::ALL),
        );
    frame.render_widget(placeholder, chunks[1]);
}
