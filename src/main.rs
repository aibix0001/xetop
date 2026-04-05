mod app;
mod model;
mod ui;

use std::io;

use anyhow::Result;
use clap::Parser;
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use log::LevelFilter;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use simplelog::{Config, WriteLogger};

use crate::app::App;

#[derive(Parser)]
#[command(name = "xetop", about = "Intel Xe GPU & NPU monitor")]
struct Cli {
    /// Update interval in milliseconds
    #[arg(short, long, default_value_t = 1000)]
    interval: u64,

    /// Log file path (debug logging)
    #[arg(long)]
    log: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Set up file-based debug logging if requested.
    if let Some(log_path) = &cli.log {
        let file = std::fs::File::create(log_path)?;
        WriteLogger::init(LevelFilter::Debug, Config::default(), file)?;
    }

    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run application
    let mut app = App::new(cli.interval);
    let result = run_app(&mut terminal, &mut app);

    // Terminal teardown (always runs)
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    while app.running {
        terminal.draw(|frame| ui::draw(frame, app))?;
        app.handle_events()?;
        app.tick();
    }
    Ok(())
}
