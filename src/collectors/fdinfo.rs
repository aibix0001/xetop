use std::collections::HashMap;
use std::fs;
use std::path::Path;

use log::debug;

use crate::model::ProcessGpuUsage;

const ENGINE_NAMES: &[&str] = &["rcs", "ccs", "bcs", "vcs", "vecs"];

struct ClientSnapshot {
    pid: u32,
    command: String,
    client_id: u32,
    gtt_kb: u64,
    /// engine_name -> (cycles, total_cycles)
    engine_cycles: HashMap<String, (u64, u64)>,
}

pub struct FdinfoCollector {
    /// Previous snapshot keyed by drm-client-id.
    prev: HashMap<u32, ClientSnapshot>,
    tick_counter: u64,
    /// Only scan every Nth tick to reduce overhead.
    scan_interval: u64,
    /// Cached results from last scan.
    cached: Vec<ProcessGpuUsage>,
}

impl FdinfoCollector {
    pub fn new(scan_interval: u64) -> Self {
        Self {
            prev: HashMap::new(),
            tick_counter: 0,
            scan_interval: scan_interval.max(1),
            cached: Vec::new(),
        }
    }

    pub fn collect(&mut self) -> Vec<ProcessGpuUsage> {
        self.tick_counter += 1;
        if self.tick_counter % self.scan_interval != 0 && !self.cached.is_empty() {
            return self.cached.clone();
        }

        let snapshots = scan_fdinfo();
        let mut results = Vec::new();

        for snap in &snapshots {
            if let Some(prev) = self.prev.get(&snap.client_id) {
                let mut engine_utils = Vec::new();

                for name in ENGINE_NAMES {
                    let name = name.to_string();
                    let util = match (
                        snap.engine_cycles.get(&name),
                        prev.engine_cycles.get(&name),
                    ) {
                        (Some(&(cur_active, cur_total)), Some(&(prev_active, prev_total))) => {
                            let d_active = cur_active.saturating_sub(prev_active);
                            let d_total = cur_total.saturating_sub(prev_total);
                            if d_total > 0 {
                                (d_active as f64 / d_total as f64 * 100.0).clamp(0.0, 100.0)
                            } else {
                                0.0
                            }
                        }
                        _ => 0.0,
                    };
                    engine_utils.push((name, util));
                }

                results.push(ProcessGpuUsage {
                    pid: snap.pid,
                    command: snap.command.clone(),
                    client_id: snap.client_id,
                    gtt_kb: snap.gtt_kb,
                    engine_utils,
                });
            }
        }

        // Update previous snapshots.
        self.prev.clear();
        for snap in snapshots {
            self.prev.insert(snap.client_id, snap);
        }

        // Sort by total utilization descending.
        results.sort_by(|a, b| {
            let sum_a: f64 = a.engine_utils.iter().map(|(_, v)| v).sum();
            let sum_b: f64 = b.engine_utils.iter().map(|(_, v)| v).sum();
            sum_b.partial_cmp(&sum_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        self.cached = results.clone();
        results
    }
}

fn scan_fdinfo() -> Vec<ClientSnapshot> {
    let mut snapshots = Vec::new();

    let proc = Path::new("/proc");
    let Ok(entries) = fs::read_dir(proc) else {
        return snapshots;
    };

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let Ok(pid) = name.parse::<u32>() else {
            continue;
        };

        let fdinfo_dir = entry.path().join("fdinfo");
        let Ok(fd_entries) = fs::read_dir(&fdinfo_dir) else {
            continue;
        };

        for fd_entry in fd_entries.flatten() {
            if let Some(snap) = parse_fdinfo_file(pid, &fd_entry.path()) {
                // Deduplicate: keep the snapshot with the most GTT if same client_id.
                if let Some(existing) = snapshots
                    .iter_mut()
                    .find(|s: &&mut ClientSnapshot| s.client_id == snap.client_id)
                {
                    if snap.gtt_kb > existing.gtt_kb {
                        *existing = snap;
                    }
                } else {
                    snapshots.push(snap);
                }
            }
        }
    }

    snapshots
}

fn parse_fdinfo_file(pid: u32, path: &Path) -> Option<ClientSnapshot> {
    let content = fs::read_to_string(path).ok()?;

    // Quick check: must be an xe DRM fd.
    if !content.contains("drm-driver:\txe") && !content.contains("drm-driver: xe") {
        return None;
    }

    debug!("Found xe fdinfo for pid {pid}: {}", path.display());

    let mut client_id = 0u32;
    let mut gtt_kb = 0u64;
    let mut engine_cycles: HashMap<String, (u64, u64)> = HashMap::new();

    for line in content.lines() {
        let line = line.trim();

        if let Some(val) = line.strip_prefix("drm-client-id:") {
            client_id = val.trim().parse().unwrap_or(0);
        } else if let Some(val) = line.strip_prefix("drm-total-gtt:") {
            // Format: "377400 KiB"
            gtt_kb = val
                .trim()
                .split_whitespace()
                .next()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
        } else if let Some(rest) = line.strip_prefix("drm-cycles-") {
            // "drm-cycles-rcs:\t534601666"
            if let Some((engine, val)) = rest.split_once(':') {
                let cycles: u64 = val.trim().parse().unwrap_or(0);
                engine_cycles
                    .entry(engine.to_string())
                    .and_modify(|e| e.0 = cycles)
                    .or_insert((cycles, 0));
            }
        } else if let Some(rest) = line.strip_prefix("drm-total-cycles-") {
            // "drm-total-cycles-rcs:\t35989466732"
            if let Some((engine, val)) = rest.split_once(':') {
                let total: u64 = val.trim().parse().unwrap_or(0);
                engine_cycles
                    .entry(engine.to_string())
                    .and_modify(|e| e.1 = total)
                    .or_insert((0, total));
            }
        }
    }

    if client_id == 0 {
        return None;
    }

    // Read process command name.
    let command = fs::read_to_string(format!("/proc/{pid}/comm"))
        .unwrap_or_default()
        .trim()
        .to_string();

    Some(ClientSnapshot {
        pid,
        command,
        client_id,
        gtt_kb,
        engine_cycles,
    })
}
