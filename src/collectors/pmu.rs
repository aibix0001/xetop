use std::fs;
use std::io;
use std::os::fd::RawFd;
use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result};
use log::{info, warn};

use crate::model::EngineMetrics;

/// xe PMU event IDs (from sysfs events/).
const EVENT_ACTIVE_TICKS: u64 = 0x02;
const EVENT_TOTAL_TICKS: u64 = 0x03;

/// Engine definitions: (name, label, gt_id, engine_class, engine_instance).
const ENGINES: &[(&str, &str, u32, u64, u64)] = &[
    ("rcs", "Render", 0, 0, 0),
    ("ccs", "Compute", 0, 1, 0),
    ("bcs", "Blitter", 0, 4, 0),
    ("vcs", "Video", 1, 2, 0),
    ("vecs", "VideoEnh", 1, 3, 0),
];

struct PerfEventFd {
    fd: RawFd,
}

impl PerfEventFd {
    fn read_value(&self) -> io::Result<u64> {
        // perf event fds reset position on each read — just read directly.
        let mut buf = [0u8; 8];
        let ret = unsafe {
            libc::read(
                self.fd,
                buf.as_mut_ptr() as *mut libc::c_void,
                buf.len(),
            )
        };
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }
        if ret != 8 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "short read"));
        }
        Ok(u64::from_ne_bytes(buf))
    }
}

impl Drop for PerfEventFd {
    fn drop(&mut self) {
        unsafe { libc::close(self.fd) };
    }
}

struct EngineCounters {
    name: String,
    label: String,
    gt_id: u32,
    active_fd: PerfEventFd,
    total_fd: PerfEventFd,
}

pub struct PmuCollector {
    engines: Vec<EngineCounters>,
    prev: Vec<(u64, u64)>,
    prev_timestamp: Option<Instant>,
    pub available: bool,
}

impl PmuCollector {
    pub fn new() -> Self {
        match Self::try_init() {
            Ok(collector) => collector,
            Err(e) => {
                warn!("PMU not available: {e}");
                Self {
                    engines: Vec::new(),
                    prev: Vec::new(),
                    prev_timestamp: None,
                    available: false,
                }
            }
        }
    }

    fn try_init() -> Result<Self> {
        let pmu_type = find_pmu_type()?;
        info!("xe PMU type: {pmu_type}");

        let mut engines = Vec::new();

        for &(name, label, gt_id, engine_class, engine_instance) in ENGINES {
            let active_config = build_config(EVENT_ACTIVE_TICKS, engine_class, engine_instance, gt_id as u64);
            let total_config = build_config(EVENT_TOTAL_TICKS, engine_class, engine_instance, gt_id as u64);

            let active_fd = PerfEventFd {
                fd: perf_event_open(pmu_type, active_config)
                    .with_context(|| format!("failed to open PMU event for {name} active"))?,
            };
            let total_fd = PerfEventFd {
                fd: perf_event_open(pmu_type, total_config)
                    .with_context(|| format!("failed to open PMU event for {name} total"))?,
            };

            engines.push(EngineCounters {
                name: name.to_string(),
                label: label.to_string(),
                gt_id,
                active_fd,
                total_fd,
            });
        }

        info!("PMU initialized with {} engines", engines.len());

        Ok(Self {
            prev: vec![(0, 0); engines.len()],
            engines,
            prev_timestamp: None,
            available: true,
        })
    }

    pub fn collect(&mut self) -> Vec<EngineMetrics> {
        if !self.available {
            return ENGINES
                .iter()
                .map(|&(name, label, gt_id, _, _)| EngineMetrics {
                    name: name.to_string(),
                    label: label.to_string(),
                    gt_id,
                    ..Default::default()
                })
                .collect();
        }

        let now = Instant::now();
        let mut metrics = Vec::with_capacity(self.engines.len());

        for (i, engine) in self.engines.iter().enumerate() {
            let active = engine.active_fd.read_value().unwrap_or(0);
            let total = engine.total_fd.read_value().unwrap_or(0);

            let utilization_pct = if self.prev_timestamp.is_some() {
                let (prev_active, prev_total) = self.prev[i];
                let d_active = active.saturating_sub(prev_active);
                let d_total = total.saturating_sub(prev_total);
                if d_total > 0 {
                    (d_active as f64 / d_total as f64 * 100.0).clamp(0.0, 100.0)
                } else {
                    0.0
                }
            } else {
                0.0
            };

            self.prev[i] = (active, total);

            metrics.push(EngineMetrics {
                name: engine.name.clone(),
                label: engine.label.clone(),
                gt_id: engine.gt_id,
                active_cycles: active,
                total_cycles: total,
                utilization_pct,
            });
        }

        self.prev_timestamp = Some(now);
        metrics
    }
}

/// Build the PMU config value from event, engine_class, engine_instance, and gt.
/// Format: event[0:11] | engine_instance[12:19] | engine_class[20:27] | gt[60:63]
fn build_config(event: u64, engine_class: u64, engine_instance: u64, gt: u64) -> u64 {
    (event & 0xFFF)
        | ((engine_instance & 0xFF) << 12)
        | ((engine_class & 0xFF) << 20)
        | ((gt & 0xF) << 60)
}

/// Find the xe PMU type by scanning /sys/bus/event_source/devices/xe_*/type.
fn find_pmu_type() -> Result<u32> {
    let base = Path::new("/sys/bus/event_source/devices");
    for entry in fs::read_dir(base)? {
        let entry = entry?;
        let name = entry.file_name();
        if name.to_string_lossy().starts_with("xe_") {
            let type_path = entry.path().join("type");
            let type_str = fs::read_to_string(&type_path)
                .with_context(|| format!("failed to read {}", type_path.display()))?;
            return type_str
                .trim()
                .parse::<u32>()
                .with_context(|| "failed to parse PMU type");
        }
    }
    anyhow::bail!("no xe PMU device found")
}

/// Call perf_event_open syscall.
fn perf_event_open(pmu_type: u32, config: u64) -> Result<RawFd> {
    // perf_event_attr struct — we only need a few fields.
    // Total size is 136 bytes for the version we target.
    #[repr(C)]
    struct PerfEventAttr {
        type_: u32,
        size: u32,
        config: u64,
        sample_period_or_freq: u64,
        sample_type: u64,
        read_format: u64,
        flags: u64,
        wakeup_events_or_watermark: u32,
        bp_type: u32,
        config1_or_bp_addr: u64,
        config2_or_bp_len: u64,
        branch_sample_type: u64,
        sample_regs_user: u64,
        sample_stack_user: u32,
        clockid: i32,
        sample_regs_intr: u64,
        aux_watermark: u32,
        sample_max_stack: u16,
        __reserved_2: u16,
        aux_sample_size: u32,
        __reserved_3: u32,
        sig_data: u64,
        config3: u64,
    }

    let mut attr: PerfEventAttr = unsafe { std::mem::zeroed() };
    attr.type_ = pmu_type;
    attr.size = std::mem::size_of::<PerfEventAttr>() as u32;
    attr.config = config;

    let fd = unsafe {
        libc::syscall(
            libc::SYS_perf_event_open,
            &attr as *const PerfEventAttr,
            -1i32, // pid: -1 = all processes
            0i32,  // cpu: 0 (system-wide needs one CPU, kernel aggregates)
            -1i32, // group_fd: -1 = no group
            0u64,  // flags
        )
    };

    if fd < 0 {
        let err = io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::EACCES) {
            anyhow::bail!("permission denied — run with CAP_PERFMON or as root");
        }
        anyhow::bail!("perf_event_open failed: {err}");
    }

    Ok(fd as RawFd)
}
