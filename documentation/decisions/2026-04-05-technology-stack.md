---
title: "ADR: Technology Stack for xetop"
date: 2026-04-05
author: agent
status: draft
related_issues: []
related_mrs: []
---

## Context

We need to choose a programming language, TUI framework, and supporting libraries for xetop — a terminal-based real-time monitor for Intel Lunar Lake Xe GPU and NPU. The tool must:

- Read Linux sysfs/debugfs/fdinfo/RAPL files at ~1 Hz
- Use `perf_event_open()` for PMU counters
- Render a responsive, multi-panel TUI with gauges, sparklines, and tables
- Run with minimal resource overhead (it's a monitor — it shouldn't be the thing consuming resources)
- Be distributable as a single static binary

Candidates evaluated: **Rust + ratatui**, **Python + textual**, **C + ncurses**, **Go + bubbletea**.

## Decision

**Rust with ratatui** (v0.30+) as the TUI framework, crossterm as the terminal backend.

### Language: Rust

**Why Rust over alternatives:**

| Criterion | Rust | Python | C | Go |
|-----------|------|--------|---|-----|
| Startup time | ~5 ms | ~200 ms | ~3 ms | ~10 ms |
| Memory overhead | ~2 MB | ~30 MB | ~1 MB | ~8 MB |
| Safety for sysfs parsing | Excellent (Result types) | Good | Poor (buffer overflows) | Good |
| Single binary | Yes (musl) | No (venv) | Yes | Yes |
| perf_event_open | Via libc/nix crate | Via ctypes (clunky) | Native | Via syscall |
| Ecosystem for TUI | ratatui (excellent) | textual (good) | ncurses (dated) | bubbletea (good) |
| User's toolchain | rustc 1.94.0 installed | uv available | gcc available | not checked |

Rust gives C-level performance with memory safety, a mature TUI ecosystem, and produces a single static binary. The user already has Rust 1.94.0 installed.

### TUI Framework: ratatui 0.30

**Why ratatui:**

- Most actively maintained Rust TUI library (~600 contributors, weekly releases)
- Immediate-mode rendering: redraw entire UI each tick — perfect for a monitor
- Built-in widgets: `Gauge`, `Sparkline`, `Table`, `BarChart`, `Tabs`, `Block` — covers all our UI needs
- Modular workspace in v0.30 — fast compile times
- Crossterm backend works on Linux/macOS/Windows (though we only need Linux)
- Braille-resolution charts for sparklines
- No runtime dependency — pure Rust

**Rejected alternatives:**
- **textual** (Python): Too heavy for a system monitor, slow startup, not a single binary
- **ncurses** (C): No memory safety, manual widget implementation, dated API
- **bubbletea** (Go): Good but Elm-architecture is overkill for a polling monitor; Go GC pauses can cause render jitter

### Key Crate Dependencies

| Crate | Purpose | Version |
|-------|---------|---------|
| `ratatui` | TUI framework | 0.30+ |
| `crossterm` | Terminal backend (raw mode, events) | 0.28+ |
| `nix` | `perf_event_open()`, ioctl, sysfs | 0.29+ |
| `sysinfo` | Optional: CPU/memory context info | 0.33+ |
| `clap` | CLI argument parsing | 4.x |
| `anyhow` | Error handling | 1.x |
| `log` + `simplelog` | Debug logging (to file, not terminal) | 0.12+ |

### Architecture: Data Collection

Three data source backends, each as a separate module:

1. **`sysfs` module** — Read files under `/sys/class/drm/`, `/sys/class/accel/`, `/sys/class/powercap/`. No special privileges needed. Parse frequency, idle residency, NPU metrics, RAPL energy counters.

2. **`pmu` module** — Use `perf_event_open()` via the `nix` crate to read xe PMU events (`engine-active-ticks`, `engine-total-ticks`, `gt-actual-frequency`, `gt-c6-residency`). Requires `CAP_PERFMON`. Gracefully disabled if permission denied.

3. **`fdinfo` module** — Scan `/proc/*/fdinfo/*` for xe DRM clients. Parse `drm-cycles-<engine>` / `drm-total-cycles-<engine>` pairs. Track delta between samples for per-process utilization. Requires either same-user or root for full coverage.

### Architecture: Application Loop

```
main loop (1 Hz default):
  1. Collect: read all data sources in parallel (sysfs + pmu + fdinfo)
  2. Compute: calculate deltas, utilizations, rates from raw counters
  3. Render: build ratatui Frame with all widgets
  4. Input:  poll crossterm for keyboard events (quit, sort, scroll)
```

Ratatui uses immediate-mode rendering — the entire UI is redrawn each tick. At 1 Hz with ~20 widgets, this is trivially fast (<1 ms render time).

### Project Structure

```
xetop/
├── Cargo.toml
├── src/
│   ├── main.rs            # Entry point, arg parsing, main loop
│   ├── app.rs             # Application state
│   ├── ui/
│   │   ├── mod.rs         # Top-level layout
│   │   ├── gpu_panel.rs   # GPU overview panel
│   │   ├── npu_panel.rs   # NPU overview panel
│   │   ├── engine_panel.rs # Per-engine bars + sparklines
│   │   ├── process_table.rs # Per-process GPU usage
│   │   └── power_bar.rs   # RAPL power display
│   ├── collectors/
│   │   ├── mod.rs
│   │   ├── sysfs.rs       # Sysfs file readers
│   │   ├── pmu.rs         # perf_event_open wrapper
│   │   ├── fdinfo.rs      # /proc fdinfo scanner
│   │   └── rapl.rs        # RAPL energy counter reader
│   └── model.rs           # Shared data types (GpuState, NpuState, etc.)
└── documentation/
```

### Build & Distribution

- Build with `cargo build --release`
- Optional: static linking via `RUSTFLAGS='-C target-feature=+crt-static'`
- Single binary, no runtime deps, ~2-4 MB stripped
- Minimum kernel: 6.8+ (xe driver with fdinfo support)

## Consequences

### What becomes easier

- **Single binary distribution** — no Python venvs, no system package deps
- **Safe sysfs parsing** — Rust's type system prevents buffer overflows and panics on malformed data
- **Performance** — sub-millisecond render, <0.5% CPU at 1 Hz polling
- **Extensibility** — adding new data sources (future hwmon, future NPU perf events) is a new module behind a trait
- **The ratatui widget library** covers gauges, sparklines, tables, and bar charts out of the box

### What becomes harder

- **Compile times** — initial build ~30-60s (incremental ~3-5s), vs instant for Python
- **Prototyping** — Rust is more verbose than Python for quick experiments
- **Cross-platform** — tied to Linux sysfs/procfs (but that's inherent to the problem, not the language)

### Alternatives explicitly rejected

- **Wrapping `intel_gpu_top`**: It doesn't support the xe driver at all on this system (confirmed: "no i915 devices found"). Dead end.
- **Using nvtop as-is**: nvtop recognizes xe but can't parse the `drm-cycles-*` fdinfo format (it expects i915's `drm-engine-*`). Would need C patches upstream.
- **Using existing Rust crates** (`silicon-monitor`, `qmassa`): Too young, incomplete Xe/NPU support. Better to build purpose-built and contribute upstream later.
