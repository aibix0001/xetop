use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};

use crate::collectors::pmu::PmuCollector;
use crate::collectors::rapl::RaplCollector;
use crate::collectors::sysfs::SysfsCollector;
use crate::model::{EngineMetrics, GpuState, NpuState, ProcessGpuUsage, RaplState};

pub struct App {
    pub running: bool,
    pub tick_count: u64,
    pub interval: Duration,
    pub gpu: GpuState,
    pub npu: NpuState,
    pub rapl: RaplState,
    pub processes: Vec<ProcessGpuUsage>,
    pub engines: Vec<EngineMetrics>,
    pub pmu_available: bool,
    sysfs: SysfsCollector,
    rapl_collector: RaplCollector,
    pmu: PmuCollector,
}

impl App {
    pub fn new(
        interval_ms: u64,
        sysfs: SysfsCollector,
        rapl_collector: RaplCollector,
        pmu: PmuCollector,
    ) -> Self {
        let pmu_available = pmu.available;
        Self {
            running: true,
            tick_count: 0,
            interval: Duration::from_millis(interval_ms),
            gpu: GpuState::default(),
            npu: NpuState::default(),
            rapl: RaplState::default(),
            processes: Vec::new(),
            engines: Vec::new(),
            pmu_available,
            sysfs,
            rapl_collector,
            pmu,
        }
    }

    pub fn tick(&mut self) {
        self.tick_count += 1;
        self.gpu = self.sysfs.collect_gpu();
        self.npu = self.sysfs.collect_npu();
        self.rapl = self.rapl_collector.collect();
        self.engines = self.pmu.collect();
    }

    pub fn handle_events(&mut self) -> Result<()> {
        let timeout = self
            .interval
            .checked_sub(Duration::from_millis(10))
            .unwrap_or(self.interval);

        let deadline = Instant::now() + timeout;

        while Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            if event::poll(remaining)?
                && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => self.running = false,
                    _ => {}
                }
            }
        }
        Ok(())
    }
}
