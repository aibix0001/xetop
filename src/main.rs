mod app;
mod collectors;
mod history;
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
use crate::collectors::fdinfo::FdinfoCollector;
use crate::collectors::pmu::PmuCollector;
use crate::collectors::rapl::RaplCollector;
use crate::collectors::sysfs::{SysfsCollector, find_npu_device, find_xe_device};

#[derive(Parser)]
#[command(name = "xetop", about = "Intel Xe GPU & NPU monitor")]
struct Cli {
    /// Update interval in milliseconds
    #[arg(short, long, default_value_t = 1000)]
    interval: u64,

    /// Log file path (debug logging)
    #[arg(long)]
    log: Option<String>,

    /// Print one sample to stdout and exit (for scripting)
    #[arg(long)]
    once: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Set up file-based debug logging if requested.
    if let Some(log_path) = &cli.log {
        let file = std::fs::File::create(log_path)?;
        WriteLogger::init(LevelFilter::Debug, Config::default(), file)?;
    }

    // Discover devices
    let drm_device = find_xe_device()?;
    let npu_device = find_npu_device();

    log::info!("xe device: {}", drm_device.display());
    log::info!("NPU device: {}", npu_device.display());

    // Initialize collectors
    let sysfs = SysfsCollector::new(&drm_device, &npu_device);
    let rapl = RaplCollector::new();
    let pmu = PmuCollector::new();
    let fdinfo = FdinfoCollector::new(2);

    let mut app = App::new(cli.interval, sysfs, rapl, pmu, fdinfo);

    if cli.once {
        return run_once(&mut app);
    }

    // Install panic hook to restore terminal on crash.
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(
            io::stdout(),
            LeaveAlternateScreen,
            crossterm::cursor::Show
        );
        original_hook(panic_info);
    }));

    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run application
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

/// Single-shot mode: collect two samples (for deltas), print summary, exit.
fn run_once(app: &mut App) -> Result<()> {
    // First tick for baseline.
    app.tick();
    std::thread::sleep(app.interval);
    // Second tick for deltas.
    app.tick();

    println!("xetop — Intel Xe GPU & NPU Monitor (single sample)");
    println!();

    // GPU
    for gt in &app.gpu.gts {
        let label = if gt.id == 0 { "render/compute" } else { "media" };
        println!(
            "GT{} ({}): act={} MHz, max={} MHz, util={:.1}%",
            gt.id, label, gt.act_freq_mhz, gt.max_freq_mhz, gt.utilization_pct
        );
    }
    println!();

    // NPU
    println!(
        "NPU: freq={}/{} MHz, util={:.1}%, mem={:.1} MB, state={}",
        app.npu.cur_freq_mhz,
        app.npu.max_freq_mhz,
        app.npu.utilization_pct,
        app.npu.memory_bytes as f64 / (1024.0 * 1024.0),
        app.npu.power_state
    );
    println!();

    // Power
    println!(
        "Power: pkg={:.1}W, core={:.1}W, dram={:.1}W",
        app.rapl.pkg_watts, app.rapl.core_watts, app.rapl.dram_watts
    );
    println!();

    // Engines
    if app.pmu_available {
        println!("Engines:");
        for e in &app.engines {
            println!("  {:>8} ({}): {:5.1}%", e.label, e.name, e.utilization_pct);
        }
        println!();
    }

    // Processes
    if !app.processes.is_empty() {
        println!(
            "{:<7} {:<16} {:>7}  {:>6}  {:>6}  {:>6}  {:>6}  {:>6}",
            "PID", "COMMAND", "GTT", "RCS", "VCS", "CCS", "BCS", "VECS"
        );
        for p in &app.processes {
            let gtt = if p.gtt_kb >= 1024 {
                format!("{}M", p.gtt_kb / 1024)
            } else {
                format!("{}K", p.gtt_kb)
            };
            let get_pct = |name: &str| -> f64 {
                p.engine_utils
                    .iter()
                    .find(|(n, _)| n == name)
                    .map(|(_, v)| *v)
                    .unwrap_or(0.0)
            };
            println!(
                "{:<7} {:<16} {:>7}  {:>5.1}%  {:>5.1}%  {:>5.1}%  {:>5.1}%  {:>5.1}%",
                p.pid,
                &p.command,
                gtt,
                get_pct("rcs"),
                get_pct("vcs"),
                get_pct("ccs"),
                get_pct("bcs"),
                get_pct("vecs")
            );
        }
    }

    Ok(())
}
