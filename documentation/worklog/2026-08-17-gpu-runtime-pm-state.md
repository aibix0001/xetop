---
title: "Show GPU runtime PM state"
type: worklog
status: record
created: 2026-08-17
updated: 2026-08-17
related_issues: ["#18"]
related_mrs: []
---

## What changed

- Added `GpuRuntimePmState` enum (`model.rs`) with variants `On`, `Suspend`, `SuspendNoIrq`, `Resume`, `Unknown(String)` and `Display`/`from_str` implementations.
- Added `runtime_pm_state: Option<GpuRuntimePmState>` field to `GpuState`.
- `sysfs.rs::collect_gpu()` reads `/sys/class/drm/cardX/device/power/runtime_status` and maps the kernel string to the enum.
- `gpu_panel.rs` renders the PM state as a colored label on the frequency line: green for on, yellow for suspend, dark-gray for suspend-noirq/resume, blue for resume.

## Why

Kernel 7.x exposes GPU runtime PM state via sysfs. Users need this to understand when the GPU is suspended, active, or in autosuspend — useful for power debugging and understanding GPU behavior.

## How

- `runtime_status` is optional (older kernels lack the file). When absent, `runtime_pm_state` is `None` and the GPU panel falls back to a plain `GTn: ` label — no color indicator.
- The PM state is shared across all GTs (there is a single device-level runtime PM state), so it is stored on `GpuState`, not per-GT.
