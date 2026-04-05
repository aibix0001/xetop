---
title: "PDR: xetop — Intel Lunar Lake Xe GPU & NPU Monitor"
date: 2026-04-05
author: agent
status: draft
related_issues: []
related_mrs: []
---

## Objective

Build a terminal-based monitoring tool (like nvtop/nvitop) for the Intel Lunar Lake platform, covering both the Xe integrated GPU and the NPU (Neural Processing Unit). The tool should provide real-time visibility into utilization, frequency, power state, memory usage, and per-process breakdowns.

## Target Hardware

- **CPU**: Intel Core Ultra 7 258V (Lunar Lake)
- **GPU**: Intel Lunar Lake Xe Graphics (PCI `0000:00:02.0`, device ID `0x64a0`)
  - Kernel driver: `xe` (graphics_ver 20.04, media_ver 20.00)
  - Two GT (Graphics Technology) blocks: GT0 (render/compute), GT1 (media)
- **NPU**: Intel Lunar Lake NPU (PCI `0000:00:0b.0`)
  - Kernel driver: `intel_vpu`
  - Exposed as `/dev/accel/accel0`
- **Kernel**: 6.19.3 (xe driver in-tree, intel_vpu in-tree)
- **Memory**: 32 GB shared (no dedicated VRAM — integrated GPU uses system memory via GTT)

## Data Sources — What Can We Actually Read

### Xe GPU

#### Sysfs (no privileges needed)

| Metric | Path | Notes |
|--------|------|-------|
| Current frequency (GT0) | `/sys/class/drm/card0/device/tile0/gt0/freq0/cur_freq` | MHz, requested |
| Actual frequency (GT0) | `/sys/class/drm/card0/device/tile0/gt0/freq0/act_freq` | MHz, actual (0 when idle) |
| Min/Max frequency (GT0) | `.../freq0/min_freq`, `.../freq0/max_freq` | Tunable |
| GT0 idle status | `.../gt0/gtidle/idle_status` | "gt-c6" when idle |
| GT0 idle residency | `.../gt0/gtidle/idle_residency_ms` | Cumulative ms in C6 |
| Same for GT1 | `.../tile0/gt1/...` | Media engines |

**GT0 engines**: `rcs` (render), `ccs` (compute), `bcs` (blitter)
**GT1 engines**: `vcs` (video codec), `vecs` (video enhance)

#### PMU / perf_event (needs CAP_PERFMON or root)

Device: `xe_0000_00_02.0` under `/sys/bus/event_source/devices/`

| Event | ID | Unit | Description |
|-------|----|------|-------------|
| `gt-c6-residency` | 0x01 | ms | Time spent in GT C6 power state |
| `engine-active-ticks` | 0x02 | ticks | Per-engine active cycles |
| `engine-total-ticks` | 0x03 | ticks | Per-engine total cycles |
| `gt-actual-frequency` | 0x04 | MHz | Actual GT frequency |
| `gt-requested-frequency` | 0x05 | MHz | Requested GT frequency |

Format fields: `event`, `gt`, `engine_class`, `engine_instance`, `function`

Engine utilization = `engine-active-ticks / engine-total-ticks` over a sample interval.

#### fdinfo (per-process, needs read access to `/proc/<pid>/fdinfo/`)

The xe driver exposes per-client DRM stats via fdinfo. Format (confirmed on this system):

```
drm-driver:         xe
drm-client-id:      25
drm-pdev:           0000:00:02.0
drm-total-system:   0
drm-total-gtt:      377400 KiB
drm-shared-gtt:     116992 KiB
drm-active-gtt:     0
drm-resident-gtt:   377400 KiB
drm-cycles-rcs:     534601666
drm-total-cycles-rcs: 35989466732
drm-cycles-vcs:     0
drm-total-cycles-vcs: 35989466732
drm-cycles-vecs:    0
drm-total-cycles-vecs: 35989466732
drm-cycles-bcs:     2880
drm-total-cycles-bcs: 35989466732
drm-cycles-ccs:     3737
drm-total-cycles-ccs: 35989466732
```

**Key insight**: xe uses `drm-cycles-<engine>` / `drm-total-cycles-<engine>` (NOT `drm-engine-<name>` like i915). This is why nvtop doesn't fully support Xe yet — it expects the i915 fdinfo format.

Per-process engine utilization = delta(`drm-cycles-<engine>`) / delta(`drm-total-cycles-<engine>`)

#### debugfs (root only)

- `/sys/kernel/debug/dri/0/clients` — active DRM clients with PID, command name, uid
- `/sys/kernel/debug/dri/0/gt0/stats` — SVM/pagefault/TLB statistics
- `/sys/kernel/debug/dri/0/gt0/hw_engines` — register-level engine state
- `/sys/kernel/debug/dri/0/gt0/powergate_info` — render power gate status
- `/sys/kernel/debug/dri/0/gtt_mm` — GTT memory manager stats (total/used)

### Intel NPU

#### Sysfs (no privileges needed)

| Metric | Path | Notes |
|--------|------|-------|
| Busy time | `/sys/class/accel/accel0/device/npu_busy_time_us` | Cumulative microseconds |
| Current frequency | `.../npu_current_frequency_mhz` | MHz (observed: 950) |
| Max frequency | `.../npu_max_frequency_mhz` | MHz (observed: 1900) |
| Memory utilization | `.../npu_memory_utilization` | Bytes (observed: ~1.6 GB) |
| Power state | `.../power_state` | D0/D3cold etc. |

NPU utilization = delta(`npu_busy_time_us`) / delta(wall_time_us)

#### debugfs (root only)

- `/sys/kernel/debug/accel/0000:00:0b.0/dvfs_mode` — DVFS mode (0 = auto)
- `.../fw_log` — firmware log
- `.../fw_trace_level` — trace verbosity
- `.../firewall_irq_counter` — error counter

### Power (RAPL)

| Domain | Path | Description |
|--------|------|-------------|
| Package | `/sys/class/powercap/intel-rapl:0/energy_uj` | Total SoC power |
| Core | `/sys/class/powercap/intel-rapl:0:0/energy_uj` | CPU cores only |
| DRAM | `/sys/class/powercap/intel-rapl:0:1/energy_uj` | Memory subsystem |

Note: No separate GPU RAPL domain exposed on Lunar Lake (GPU power is part of package). Delta(energy_uj) / delta(time) = watts.

## Proposed UI Layout

```
┌─────────────────────────── xetop ────────────────────────────┐
│ Xe GPU (Lunar Lake)                    NPU (Lunar Lake)      │
│ ┌──────────────────────────┐  ┌────────────────────────────┐ │
│ │ Freq: 1200/1950 MHz      │  │ Freq: 950/1900 MHz         │ │
│ │ GT0: ████████░░ 78%      │  │ Util: ██░░░░░░░░ 18%       │ │
│ │ GT1: ██░░░░░░░░ 15%      │  │ Mem:  1.5 / 32.0 GB        │ │
│ │ C6:  ░░░░░░░░░░  2%      │  │ State: D0 (active)         │ │
│ │ GTT:  1.3 / 8.0 GB       │  │                            │ │
│ └──────────────────────────┘  └────────────────────────────┘ │
│                                                              │
│ Power: Pkg 12.3W | Core 4.1W | DRAM 2.8W                    │
│──────────────────────────────────────────────────────────────│
│ Engine Breakdown          ▼ sparkline history (60s)          │
│  RCS (render)  ████████░░ 78%  ▁▂▃▅▇█▇▅▃▂                  │
│  CCS (compute) ░░░░░░░░░░  0%  ▁▁▁▁▁▁▁▁▁▁                  │
│  BCS (blitter) █░░░░░░░░░  5%  ▁▁▁▂▁▁▁▁▁▁                  │
│  VCS (video)   ██░░░░░░░░ 15%  ▁▁▃▅▃▁▁▁▁▁                  │
│  VECS (venhance)░░░░░░░░░  0%  ▁▁▁▁▁▁▁▁▁▁                  │
│──────────────────────────────────────────────────────────────│
│ Per-Process GPU Usage                                        │
│  PID   COMMAND        GTT     RCS   VCS   CCS   BCS  VECS   │
│  3660  Xwayland      48 MB   12%    0%    0%    1%    0%    │
│  4738  Discord      128 MB    8%    2%    0%    0%    0%    │
│  5948  chrome       256 MB   45%   10%    0%    3%    0%    │
│  4356  npu-blurr     32 MB    2%    0%    0%    0%    0%    │
│  3000  ovms          12 MB    0%    0%    0%    0%    0%    │
└──────────────────────────────────────────────────────────────┘
```

## Feature Priorities

### P0 — MVP

1. GPU overall utilization (GT0/GT1 via PMU or sysfs idle residency)
2. GPU frequency (actual vs requested vs min/max)
3. Per-engine utilization bars (rcs, ccs, bcs, vcs, vecs)
4. NPU utilization, frequency, power state
5. Per-process GPU usage table (from fdinfo)
6. RAPL power readings

### P1 — Polish

7. Sparkline history graphs (60-second rolling window)
8. GTT memory usage (global + per-process)
9. NPU memory utilization
10. Sorting/filtering process table
11. Color-coded thresholds

### P2 — Nice to Have

12. GPU power gate status
13. Temperature (if hwmon becomes available in future kernels)
14. NPU firmware trace/error counters
15. Export to JSON for scripting
16. Mouse support for interactive exploration

## Privilege Model

| Feature | Unprivileged | CAP_PERFMON | Root |
|---------|-------------|-------------|------|
| GPU sysfs freq/idle | Yes | Yes | Yes |
| NPU sysfs metrics | Yes | Yes | Yes |
| RAPL power | Yes | Yes | Yes |
| PMU engine ticks | No | Yes | Yes |
| fdinfo (own processes) | Yes | Yes | Yes |
| fdinfo (all processes) | No | No | Yes |
| debugfs | No | No | Yes |

The tool should gracefully degrade: show what's available at the current privilege level, with hints about what requires elevation.

## Sampling Strategy

- **Poll interval**: 1 second default, configurable (250ms–5s)
- **sysfs reads**: Direct file reads, ~10 files per tick — negligible overhead
- **fdinfo scanning**: Walk `/proc/*/fdinfo/*` looking for `drm-driver: xe` — heavier, but nvtop does the same
- **PMU events**: Use `perf_event_open()` syscall for engine tick counters — most accurate for utilization
- **RAPL**: Read `energy_uj` counters, compute delta watts

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| xe fdinfo format changes between kernel versions | Per-process stats break | Version-detect, parse defensively |
| No hwmon for Xe GPU on Lunar Lake (no temp/power sensors) | Can't show GPU-specific power/temp | Use RAPL package power, note limitation |
| PMU requires CAP_PERFMON | Unprivileged mode has no engine util | Fall back to GT idle residency from sysfs |
| intel_vpu driver is young, sysfs may change | NPU metrics break | Pin known sysfs paths, handle missing gracefully |
| fdinfo scanning expensive with many processes | High CPU usage | Scan every 2nd tick, cache PID→fd mappings |
