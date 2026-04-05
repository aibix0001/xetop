pub mod engine_panel;
pub mod gpu_panel;
pub mod npu_panel;
pub mod power_bar;
pub mod process_table;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::vertical([
        Constraint::Length(1),  // title/status bar
        Constraint::Length(6),  // GPU + NPU panels side by side
        Constraint::Length(1),  // power bar
        Constraint::Length(7),  // engine panel
        Constraint::Min(5),    // process table
        Constraint::Length(1),  // help bar
    ])
    .split(area);

    // Status/title bar
    let priv_level = if app.pmu_available { "PMU" } else { "sysfs" };
    let title = Line::from(vec![
        Span::styled(" xetop", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" — Intel Xe GPU & NPU Monitor "),
        Span::styled(
            format!("[{priv_level}]"),
            Style::default().fg(if app.pmu_available {
                Color::Green
            } else {
                Color::Yellow
            }),
        ),
    ]);
    frame.render_widget(Paragraph::new(title), chunks[0]);

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

    // Engine utilization panel with sparklines
    engine_panel::draw(
        frame,
        chunks[3],
        &app.engines,
        app.pmu_available,
        &app.engine_history,
    );

    // Process table
    process_table::draw(
        frame,
        chunks[4],
        &app.processes,
        app.selected_process,
        app.sort_column,
        app.sort_reverse,
    );

    // Help bar
    let help = Line::from(vec![
        Span::styled(" q", Style::default().fg(Color::Yellow)),
        Span::raw(":quit "),
        Span::styled("s", Style::default().fg(Color::Yellow)),
        Span::raw(":sort "),
        Span::styled("r", Style::default().fg(Color::Yellow)),
        Span::raw(":reverse "),
        Span::styled("↑↓/jk", Style::default().fg(Color::Yellow)),
        Span::raw(":navigate"),
    ]);
    frame.render_widget(
        Paragraph::new(help).style(Style::default().fg(Color::DarkGray)),
        chunks[5],
    );
}
