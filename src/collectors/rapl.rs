use std::path::{Path, PathBuf};
use std::time::Instant;

use log::warn;

use super::read_sysfs_value;
use crate::model::RaplState;

pub struct RaplCollector {
    pkg_path: PathBuf,
    core_path: PathBuf,
    dram_path: PathBuf,
    prev: Option<RaplSnapshot>,
    available: bool,
}

struct RaplSnapshot {
    pkg_uj: u64,
    core_uj: u64,
    dram_uj: u64,
    timestamp: Instant,
}

impl RaplCollector {
    pub fn new() -> Self {
        let base = Path::new("/sys/class/powercap/intel-rapl:0");
        let available = base.join("energy_uj").exists();
        if !available {
            warn!("RAPL not available at {}", base.display());
        }

        Self {
            pkg_path: base.join("energy_uj"),
            core_path: base.join("intel-rapl:0:0/energy_uj"),
            dram_path: base.join("intel-rapl:0:1/energy_uj"),
            prev: None,
            available,
        }
    }

    pub fn collect(&mut self) -> RaplState {
        if !self.available {
            return RaplState::default();
        }

        let now = Instant::now();
        let pkg_uj: u64 = read_sysfs_value(&self.pkg_path).unwrap_or(0);
        let core_uj: u64 = read_sysfs_value(&self.core_path).unwrap_or(0);
        let dram_uj: u64 = read_sysfs_value(&self.dram_path).unwrap_or(0);

        let (pkg_watts, core_watts, dram_watts) = if let Some(prev) = &self.prev {
            let dt = now.duration_since(prev.timestamp).as_secs_f64();
            if dt > 0.0 {
                (
                    delta_watts(pkg_uj, prev.pkg_uj, dt),
                    delta_watts(core_uj, prev.core_uj, dt),
                    delta_watts(dram_uj, prev.dram_uj, dt),
                )
            } else {
                (0.0, 0.0, 0.0)
            }
        } else {
            (0.0, 0.0, 0.0)
        };

        self.prev = Some(RaplSnapshot {
            pkg_uj,
            core_uj,
            dram_uj,
            timestamp: now,
        });

        RaplState {
            pkg_watts,
            core_watts,
            dram_watts,
            pkg_energy_uj: pkg_uj,
            core_energy_uj: core_uj,
            dram_energy_uj: dram_uj,
        }
    }
}

/// Compute watts from energy counter delta, handling overflow of the RAPL register.
fn delta_watts(current: u64, previous: u64, dt_secs: f64) -> f64 {
    let delta = if current >= previous {
        current - previous
    } else {
        // RAPL counter wrapped around (32-bit or implementation-specific max).
        current.wrapping_sub(previous)
    };
    delta as f64 / 1_000_000.0 / dt_secs
}
