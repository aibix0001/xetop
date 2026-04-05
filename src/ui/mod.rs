pub mod engine_panel;
pub mod gpu_panel;
pub mod npu_panel;
pub mod power_bar;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::vertical([
        Constraint::Length(1),  // title bar
        Constraint::Length(6),  // GPU + NPU panels side by side
        Constraint::Length(1),  // power bar
        Constraint::Length(7),  // engine panel
        Constraint::Min(1),    // placeholder for process table
    ])
    .split(area);

    // Title bar
    let title = Paragraph::new(format!(
        " xetop — Intel Xe GPU & NPU Monitor | tick: {}",
        app.tick_count
    ))
    .style(Style::default().fg(Color::Cyan));
    frame.render_widget(title, chunks[0]);

    // GPU + NPU panels side by side
    let panels = Layout::horizontal([
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ])
    .split(chunks[1]);

    gpu_panel::draw(frame, panels[0], &app.gpu);
    npu_panel::draw(frame, panels[1], &app.npu);

    // Power bar
    power_bar::draw(frame, chunks[2], &app.rapl);

    // Engine utilization panel
    engine_panel::draw(frame, chunks[3], &app.engines, app.pmu_available);

    // Placeholder for process table (Phase 4)
    let placeholder = Paragraph::new("  Per-process GPU usage coming soon...")
        .block(
            Block::default()
                .title(" Processes ")
                .borders(Borders::ALL),
        );
    frame.render_widget(placeholder, chunks[4]);
}
