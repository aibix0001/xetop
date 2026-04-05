/// Shared data types for xetop.

#[derive(Debug, Default)]
pub struct GpuState {
    /// Per-GT state (GT0 = render/compute, GT1 = media).
    pub gts: Vec<GtState>,
}

#[derive(Debug, Default, Clone)]
pub struct GtState {
    pub id: u32,
    pub cur_freq_mhz: u32,
    pub act_freq_mhz: u32,
    pub min_freq_mhz: u32,
    pub max_freq_mhz: u32,
    pub idle_status: String,
    /// C6 residency percentage (computed from deltas).
    pub c6_residency_pct: f64,
    /// Raw cumulative idle_residency_ms for delta computation.
    pub idle_residency_ms: u64,
}

#[derive(Debug, Default)]
pub struct NpuState {
    pub cur_freq_mhz: u32,
    pub max_freq_mhz: u32,
    /// NPU utilization percentage (computed from busy_time deltas).
    pub utilization_pct: f64,
    /// Raw cumulative busy_time_us for delta computation.
    pub busy_time_us: u64,
    pub memory_bytes: u64,
    pub power_state: String,
}

#[derive(Debug, Default, Clone)]
pub struct EngineMetrics {
    pub name: String,
    pub label: String,
    pub active_ticks: u64,
    pub total_ticks: u64,
    pub utilization_pct: f64,
}

#[derive(Debug, Default)]
pub struct RaplState {
    pub pkg_watts: f64,
    pub core_watts: f64,
    pub dram_watts: f64,
    /// Raw cumulative energy_uj values for delta computation.
    pub pkg_energy_uj: u64,
    pub core_energy_uj: u64,
    pub dram_energy_uj: u64,
}

#[derive(Debug, Default, Clone)]
pub struct ProcessGpuUsage {
    pub pid: u32,
    pub command: String,
    pub client_id: u32,
    pub gtt_kb: u64,
    /// Per-engine utilization: engine_name -> percentage.
    pub engine_utils: Vec<(String, f64)>,
}
