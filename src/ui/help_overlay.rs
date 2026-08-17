use ratatui::Frame;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

const HELP_TEXT: &str = "\
  q / Esc     Quit
  ?           Toggle this help
  ↑ / k       Move selection up
  ↓ / j       Move selection down
  s           Cycle sort column
  r           Reverse sort order

  Data sources:
    sysfs     GPU freq, C6 idle, NPU metrics, scheduler params
    PMU       Per-engine utilization (needs CAP_PERFMON)
    fdinfo    Per-process GPU usage (own procs or root)
    RAPL      Package/core/DRAM power (no privileges)";

pub fn draw(frame: &mut Frame) {
    let area = centered_rect(50, 16, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Help ")
        .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let help = Paragraph::new(HELP_TEXT)
        .block(block)
        .style(Style::default().fg(Color::White));

    frame.render_widget(help, area);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .split(area);
    Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .split(vertical[0])[0]
}
