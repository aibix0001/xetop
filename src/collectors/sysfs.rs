use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::Result;
use log::warn;

use super::{read_sysfs, read_sysfs_value};
use crate::model::{GpuState, GtState, NpuState};

pub struct SysfsCollector {
    drm_device_path: PathBuf,
    npu_device_path: PathBuf,
    gt_ids: Vec<u32>,
    prev_gt_residency: Vec<(u32, u64)>,
    prev_gpu_timestamp: Option<Instant>,
    prev_npu_busy_us: Option<u64>,
    prev_npu_timestamp: Option<Instant>,
    npu_available: bool,
}

impl SysfsCollector {
    pub fn new(drm_device_path: &Path, npu_device_path: &Path) -> Self {
        // Discover available GTs by scanning tile0/gt* directories.
        let mut gt_ids = Vec::new();
        let tile_path = drm_device_path.join("tile0");
        if let Ok(entries) = std::fs::read_dir(&tile_path) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if let Some(id_str) = name.strip_prefix("gt")
                    && let Ok(id) = id_str.parse::<u32>()
                {
                    gt_ids.push(id);
                }
            }
        }
        gt_ids.sort();

        let npu_available = npu_device_path.exists();
        if !npu_available {
            warn!("NPU device path not found: {}", npu_device_path.display());
        }

        Self {
            drm_device_path: drm_device_path.to_path_buf(),
            npu_device_path: npu_device_path.to_path_buf(),
            gt_ids,
            prev_gt_residency: Vec::new(),
            prev_gpu_timestamp: None,
            prev_npu_busy_us: None,
            prev_npu_timestamp: None,
            npu_available,
        }
    }

    pub fn collect_gpu(&mut self) -> GpuState {
        let mut gts = Vec::new();

        for &gt_id in &self.gt_ids {
            let gt_path = self.drm_device_path.join(format!("tile0/gt{gt_id}"));
            let freq_path = gt_path.join("freq0");
            let idle_path = gt_path.join("gtidle");

            let gt = GtState {
                id: gt_id,
                cur_freq_mhz: read_sysfs_value(&freq_path.join("cur_freq")).unwrap_or(0),
                act_freq_mhz: read_sysfs_value(&freq_path.join("act_freq")).unwrap_or(0),
                min_freq_mhz: read_sysfs_value(&freq_path.join("min_freq")).unwrap_or(0),
                max_freq_mhz: read_sysfs_value(&freq_path.join("max_freq")).unwrap_or(0),
                idle_status: read_sysfs(&idle_path.join("idle_status")).unwrap_or_default(),
                idle_residency_ms: read_sysfs_value(&idle_path.join("idle_residency_ms"))
                    .unwrap_or(0),
                utilization_pct: 0.0,
                c6_residency_pct: 0.0,
            };

            gts.push(gt);
        }

        self.compute_c6_deltas(&mut gts);

        GpuState { gts }
    }

    fn compute_c6_deltas(&mut self, gts: &mut [GtState]) {
        let now = Instant::now();

        let Some(prev_ts) = self.prev_gpu_timestamp else {
            // First sample — store baseline.
            self.prev_gt_residency =
                gts.iter().map(|gt| (gt.id, gt.idle_residency_ms)).collect();
            self.prev_gpu_timestamp = Some(now);
            return;
        };

        let elapsed_ms = now.duration_since(prev_ts).as_millis() as u64;
        if elapsed_ms == 0 {
            return;
        }

        for gt in gts.iter_mut() {
            if let Some(prev) = self
                .prev_gt_residency
                .iter()
                .find(|(id, _)| *id == gt.id)
            {
                let delta = gt.idle_residency_ms.saturating_sub(prev.1);
                gt.c6_residency_pct =
                    (delta as f64 / elapsed_ms as f64 * 100.0).clamp(0.0, 100.0);
            }
        }

        self.prev_gt_residency =
            gts.iter().map(|gt| (gt.id, gt.idle_residency_ms)).collect();
        self.prev_gpu_timestamp = Some(now);
    }

    pub fn collect_npu(&mut self) -> NpuState {
        if !self.npu_available {
            return NpuState::default();
        }

        let dev = &self.npu_device_path;

        let busy_time_us: u64 = read_sysfs_value(&dev.join("npu_busy_time_us")).unwrap_or(0);
        let cur_freq_mhz: u32 =
            read_sysfs_value(&dev.join("npu_current_frequency_mhz")).unwrap_or(0);
        let max_freq_mhz: u32 =
            read_sysfs_value(&dev.join("npu_max_frequency_mhz")).unwrap_or(0);
        let memory_bytes: u64 =
            read_sysfs_value(&dev.join("npu_memory_utilization")).unwrap_or(0);
        let power_state: String = read_sysfs(&dev.join("power_state")).unwrap_or_default();

        let now = Instant::now();
        let utilization_pct =
            if let (Some(prev_busy), Some(prev_ts)) = (self.prev_npu_busy_us, self.prev_npu_timestamp)
            {
                let delta_busy = busy_time_us.saturating_sub(prev_busy);
                let delta_time = now.duration_since(prev_ts).as_micros() as u64;
                if delta_time > 0 {
                    (delta_busy as f64 / delta_time as f64 * 100.0).clamp(0.0, 100.0)
                } else {
                    0.0
                }
            } else {
                0.0
            };

        self.prev_npu_busy_us = Some(busy_time_us);
        self.prev_npu_timestamp = Some(now);

        NpuState {
            cur_freq_mhz,
            max_freq_mhz,
            utilization_pct,
            busy_time_us,
            memory_bytes,
            power_state,
        }
    }
}

/// Find the xe DRM device path (e.g. /sys/class/drm/card0/device).
pub fn find_xe_device() -> Result<PathBuf> {
    for entry in std::fs::read_dir("/sys/class/drm")? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("card") || name.contains('-') {
            continue;
        }
        let device_path = entry.path().join("device");
        let driver_link = device_path.join("driver");
        if let Ok(target) = std::fs::read_link(&driver_link)
            && target.to_string_lossy().ends_with("/xe")
        {
            return Ok(device_path);
        }
    }
    anyhow::bail!("no xe GPU device found in /sys/class/drm/")
}

/// Find the NPU device path (e.g. /sys/class/accel/accel0/device).
pub fn find_npu_device() -> PathBuf {
    let base = Path::new("/sys/class/accel");
    if let Ok(entries) = std::fs::read_dir(base) {
        for entry in entries.flatten() {
            let device_path = entry.path().join("device");
            let driver_link = device_path.join("driver");
            if let Ok(target) = std::fs::read_link(&driver_link) {
                let target_str = target.to_string_lossy();
                if target_str.ends_with("/intel_vpu") || target_str.ends_with("/ivpu") {
                    return device_path;
                }
            }
        }
    }
    // Return a path that won't exist — collector will disable NPU.
    PathBuf::from("/sys/class/accel/accel0/device")
}
