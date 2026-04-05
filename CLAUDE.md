# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

xetop is a terminal-based real-time monitor for Intel Lunar Lake Xe GPU and NPU, built with Rust and ratatui. It reads sysfs/PMU/fdinfo/RAPL data sources to display GPU engine utilization, frequencies, NPU metrics, power draw, and per-process GPU usage.

Target: Linux kernel 6.8+ with `xe` and `intel_vpu` drivers. Single static binary.

## Build & Run

```sh
cargo build --release
cargo run -- --interval 1000    # interval in ms
cargo test
cargo clippy -- -D warnings
```

## Issue & PR Workflow (GitHub)

**This project uses GitHub** (`gh` CLI), not GitLab MCP. This overrides the global CLAUDE.md rule.

- Use `gh issue create` to track work before coding
- Use `gh pr create` for pull requests
- Branch naming: `feature/*` for features, `issue/*` for issues
- Never code on master — all work in dedicated branches

## Architecture

- **Language**: Rust (1.75+), **TUI**: ratatui 0.30+ with crossterm backend
- **Immediate-mode rendering**: entire UI redrawn each tick (1 Hz default)

### Data Collectors (`src/collectors/`)

| Module | Data Source | Privilege |
|--------|-----------|-----------|
| `sysfs.rs` | GPU freq, GT idle residency, NPU metrics via `/sys/class/drm/`, `/sys/class/accel/` | None |
| `pmu.rs` | Per-engine active/total ticks via `perf_event_open()` | CAP_PERFMON |
| `fdinfo.rs` | Per-process GPU usage via `/proc/*/fdinfo/*` (xe `drm-cycles-*` format) | None (own), Root (all) |
| `rapl.rs` | Package/core/DRAM power via `/sys/class/powercap/intel-rapl:*` | None |

All collectors gracefully degrade when permissions are insufficient.

### Key xe Driver Details

- xe fdinfo uses `drm-cycles-<engine>` / `drm-total-cycles-<engine>` (NOT i915's `drm-engine-*`)
- GT0 engines: rcs (render, class=0), ccs (compute, class=1), bcs (blitter, class=4)
- GT1 engines: vcs (video codec, class=2), vecs (video enhance, class=3)
- PMU event config layout: `event[0:11] | engine_instance[12:19] | engine_class[20:27] | gt[60:63]`

### Application Loop

```
1. Collect: read all data sources (sysfs + pmu + fdinfo + rapl)
2. Compute: calculate deltas, utilizations, rates
3. Render: build ratatui Frame
4. Input: poll crossterm for keyboard events
```

## Key Dependencies

ratatui, crossterm, nix (for perf_event_open/ioctl), clap, anyhow

## Documentation

All docs under `documentation/` with YAML frontmatter. Start content at h2 (title comes from frontmatter). Run doc-writer after non-trivial changes.
