/// Shared data types for xetop.

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum GpuRuntimePmState {
    #[default]
    On,
    Suspend,
    SuspendNoIrq,
    Resume,
    Unknown(String),
}

impl GpuRuntimePmState {
    pub fn from_str(s: &str) -> Self {
        match s {
            "on" => Self::On,
            "suspend" => Self::Suspend,
            "suspend-noirq" => Self::SuspendNoIrq,
            "resume" => Self::Resume,
            _ => Self::Unknown(s.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::On => "on",
            Self::Suspend => "suspend",
            Self::SuspendNoIrq => "suspend-noirq",
            Self::Resume => "resume",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl std::fmt::Display for GpuRuntimePmState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}]", self.as_str())
    }
}

#[derive(Debug, Default)]
pub struct GpuState {
    /// Per-GT state (GT0 = render/compute, GT1 = media).
    pub gts: Vec<GtState>,
    /// Runtime PM state from device/power/runtime_status (kernel 7.x+).
    pub runtime_pm_state: Option<GpuRuntimePmState>,
}

#[derive(Debug, Default, Clone)]
pub struct GtState {
    pub id: u32,
    pub cur_freq_mhz: u32,
    pub act_freq_mhz: u32,
    pub max_freq_mhz: u32,
    /// Efficient frequency from freq0/rp_eff_freq (kHz → MHz).
    pub eff_freq_mhz: u32,
    /// P0 turbo frequency from freq0/rp0_freq (kHz → MHz).
    pub p0_freq_mhz: u32,
    /// Current power profile from power (e.g. "default", [performance], [power saver]).
    pub power_profile: String,
    /// GT utilization percentage (best available source).
    pub utilization_pct: f64,
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
    pub memory_bytes: u64,
    pub power_state: String,
}

#[derive(Debug, Default, Clone)]
pub struct EngineMetrics {
    pub name: String,
    pub label: String,
    pub utilization_pct: f64,
}

#[derive(Debug, Default, Clone)]
pub struct EngineSchedulerParams {
    pub name: String,
    pub timeslice_ms: u64,
    pub preempt_timeout_ms: u64,
    pub job_timeout_ms: u64,
}

#[derive(Debug, Default)]
pub struct RaplState {
    pub pkg_watts: f64,
    pub core_watts: f64,
    pub dram_watts: f64,
}

#[derive(Debug, Default, Clone)]
pub struct ProcessGpuUsage {
    pub pid: u32,
    pub command: String,
    pub gtt_kb: u64,
    /// Per-engine utilization: engine_name -> percentage.
    pub engine_utils: Vec<(String, f64)>,
}
