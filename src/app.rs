use std::collections::HashMap;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};

use crate::collectors::fdinfo::FdinfoCollector;
use crate::collectors::pmu::PmuCollector;
use crate::collectors::rapl::RaplCollector;
use crate::collectors::sysfs::SysfsCollector;
use crate::history::RingBuffer;
use crate::model::{EngineMetrics, GpuState, NpuState, ProcessGpuUsage, RaplState};

const HISTORY_LEN: usize = 60;

#[derive(Clone, Copy, PartialEq)]
pub enum SortColumn {
    Pid,
    Command,
    Gtt,
    Rcs,
    Vcs,
    Ccs,
    Bcs,
    Vecs,
}

impl SortColumn {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Pid => "PID",
            Self::Command => "CMD",
            Self::Gtt => "GTT",
            Self::Rcs => "RCS",
            Self::Vcs => "VCS",
            Self::Ccs => "CCS",
            Self::Bcs => "BCS",
            Self::Vecs => "VECS",
        }
    }

    fn next(&self) -> Self {
        match self {
            Self::Pid => Self::Command,
            Self::Command => Self::Gtt,
            Self::Gtt => Self::Rcs,
            Self::Rcs => Self::Vcs,
            Self::Vcs => Self::Ccs,
            Self::Ccs => Self::Bcs,
            Self::Bcs => Self::Vecs,
            Self::Vecs => Self::Pid,
        }
    }
}

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
    pub selected_process: usize,
    pub sort_column: SortColumn,
    pub sort_reverse: bool,
    pub show_help: bool,
    /// Per-engine sparkline history, keyed by engine name.
    pub engine_history: HashMap<String, RingBuffer>,
    /// NPU utilization history.
    pub npu_history: RingBuffer,
    sysfs: SysfsCollector,
    rapl_collector: RaplCollector,
    pmu: PmuCollector,
    fdinfo: FdinfoCollector,
}

impl App {
    pub fn new(
        interval_ms: u64,
        sysfs: SysfsCollector,
        rapl_collector: RaplCollector,
        pmu: PmuCollector,
        fdinfo: FdinfoCollector,
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
            selected_process: 0,
            sort_column: SortColumn::Rcs,
            sort_reverse: true, // highest first by default
            show_help: false,
            engine_history: HashMap::new(),
            npu_history: RingBuffer::new(HISTORY_LEN),
            sysfs,
            rapl_collector,
            pmu,
            fdinfo,
        }
    }

    pub fn tick(&mut self) {
        self.tick_count += 1;
        self.gpu = self.sysfs.collect_gpu();
        self.npu = self.sysfs.collect_npu();
        self.rapl = self.rapl_collector.collect();
        self.engines = self.pmu.collect();
        self.processes = self.fdinfo.collect();

        // Compute per-GT utilization from best available source.
        self.compute_gt_utilization();

        // Update sparkline histories.
        for engine in &self.engines {
            self.engine_history
                .entry(engine.name.clone())
                .or_insert_with(|| RingBuffer::new(HISTORY_LEN))
                .push(engine.utilization_pct);
        }
        self.npu_history.push(self.npu.utilization_pct);

        // Sort processes and clamp selection.
        self.sort_processes();
        if !self.processes.is_empty() {
            self.selected_process = self.selected_process.min(self.processes.len() - 1);
        } else {
            self.selected_process = 0;
        }
    }

    /// Compute per-GT utilization from PMU engines or aggregated fdinfo.
    fn compute_gt_utilization(&mut self) {
        // GT0 engines: rcs, ccs, bcs. GT1 engines: vcs, vecs.
        let gt0_engines = ["rcs", "ccs", "bcs"];
        let gt1_engines = ["vcs", "vecs"];

        for gt in &mut self.gpu.gts {
            let engine_names: &[&str] = if gt.id == 0 {
                &gt0_engines
            } else {
                &gt1_engines
            };

            // Try PMU data first (most accurate).
            if self.pmu_available {
                let max_util = self
                    .engines
                    .iter()
                    .filter(|e| engine_names.contains(&e.name.as_str()))
                    .map(|e| e.utilization_pct)
                    .fold(0.0_f64, f64::max);
                gt.utilization_pct = max_util;
            } else {
                // Fallback: aggregate per-process fdinfo utilization.
                // Sum each engine across all processes, take the max engine.
                let mut engine_totals: std::collections::HashMap<&str, f64> =
                    std::collections::HashMap::new();
                for proc in &self.processes {
                    for (name, pct) in &proc.engine_utils {
                        if engine_names.contains(&name.as_str()) {
                            *engine_totals.entry(name.as_str()).or_default() += pct;
                        }
                    }
                }
                gt.utilization_pct = engine_totals
                    .values()
                    .copied()
                    .fold(0.0_f64, f64::max)
                    .clamp(0.0, 100.0);
            }
        }
    }

    fn sort_processes(&mut self) {
        let col = self.sort_column;
        let rev = self.sort_reverse;

        self.processes.sort_by(|a, b| {
            let cmp = match col {
                SortColumn::Pid => a.pid.cmp(&b.pid),
                SortColumn::Command => a.command.cmp(&b.command),
                SortColumn::Gtt => a.gtt_kb.cmp(&b.gtt_kb),
                _ => {
                    let name = match col {
                        SortColumn::Rcs => "rcs",
                        SortColumn::Vcs => "vcs",
                        SortColumn::Ccs => "ccs",
                        SortColumn::Bcs => "bcs",
                        SortColumn::Vecs => "vecs",
                        _ => unreachable!(),
                    };
                    let va = engine_pct(a, name);
                    let vb = engine_pct(b, name);
                    va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
                }
            };
            if rev { cmp.reverse() } else { cmp }
        });
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
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.selected_process = self.selected_process.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if !self.processes.is_empty() {
                            self.selected_process =
                                (self.selected_process + 1).min(self.processes.len() - 1);
                        }
                    }
                    KeyCode::Char('?') => {
                        self.show_help = !self.show_help;
                    }
                    KeyCode::Char('s') => {
                        self.sort_column = self.sort_column.next();
                        self.selected_process = 0;
                    }
                    KeyCode::Char('r') => {
                        self.sort_reverse = !self.sort_reverse;
                        self.selected_process = 0;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

fn engine_pct(p: &ProcessGpuUsage, name: &str) -> f64 {
    p.engine_utils
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, v)| *v)
        .unwrap_or(0.0)
}
