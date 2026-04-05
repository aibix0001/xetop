# xetop

A terminal-based real-time monitor for Intel Lunar Lake Xe GPU and NPU, inspired by nvtop/nvitop. Built with Rust and ratatui.

## Why

Existing tools don't work well on Lunar Lake:
- `intel_gpu_top` doesn't support the xe driver at all
- `nvtop` recognizes xe but can't parse its fdinfo format (`drm-cycles-*` vs i915's `drm-engine-*`)
- No existing tool monitors the Intel NPU

xetop fills this gap with a purpose-built monitor for the xe + intel_vpu driver stack.

## Planned Features

### GPU (Xe)
- Per-engine utilization (render, compute, blitter, video codec, video enhance)
- Actual vs requested frequency, min/max range
- GT idle (C6) residency
- GTT memory usage
- Per-process GPU usage breakdown

### NPU
- Utilization (busy time)
- Current and max frequency
- Memory utilization
- Power state (D0/D3cold)

### System
- RAPL power draw (package, core, DRAM)
- Sparkline history graphs
- Sortable/filterable process table

## Requirements

- Linux kernel 6.8+ with `xe` and `intel_vpu` drivers
- Intel Lunar Lake (or compatible Xe GPU + NPU platform)
- Rust 1.75+ toolchain

### Privilege levels

| Feature | Unprivileged | CAP_PERFMON | Root |
|---------|:---:|:---:|:---:|
| GPU frequency & idle | x | x | x |
| NPU metrics | x | x | x |
| RAPL power | x | x | x |
| Per-engine utilization | | x | x |
| Per-process usage (own) | x | x | x |
| Per-process usage (all) | | | x |

## Build

```sh
cargo build --release
```

## Status

Early design phase. See [PDR](documentation/plans/xetop-project-design.md) and [ADR](documentation/decisions/2026-04-05-technology-stack.md) for details.

## License

MIT
