use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};

use crate::model::{GpuState, NpuState, RaplState};

pub struct App {
    pub running: bool,
    pub tick_count: u64,
    pub interval: Duration,
    pub gpu: GpuState,
    pub npu: NpuState,
    pub rapl: RaplState,
    pub processes: Vec<crate::model::ProcessGpuUsage>,
    pub engines: Vec<crate::model::EngineMetrics>,
}

impl App {
    pub fn new(interval_ms: u64) -> Self {
        Self {
            running: true,
            tick_count: 0,
            interval: Duration::from_millis(interval_ms),
            gpu: GpuState::default(),
            npu: NpuState::default(),
            rapl: RaplState::default(),
            processes: Vec::new(),
            engines: Vec::new(),
        }
    }

    pub fn tick(&mut self) {
        self.tick_count += 1;
    }

    pub fn handle_events(&mut self) -> Result<()> {
        let timeout = self
            .interval
            .checked_sub(Duration::from_millis(10))
            .unwrap_or(Duration::from_millis(50));

        let deadline = Instant::now() + timeout;

        while Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            if event::poll(remaining)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => self.running = false,
                            _ => {}
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
