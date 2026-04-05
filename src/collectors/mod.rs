pub mod pmu;
pub mod rapl;
pub mod sysfs;

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

/// Read a sysfs file and return its trimmed content.
pub fn read_sysfs(path: &Path) -> Result<String> {
    fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))
        .map(|s| s.trim().to_string())
}

/// Read a sysfs file and parse it as the given numeric type.
pub fn read_sysfs_value<T: std::str::FromStr>(path: &Path) -> Result<T>
where
    T::Err: std::fmt::Display,
{
    let content = read_sysfs(path)?;
    content
        .parse::<T>()
        .map_err(|e| anyhow::anyhow!("failed to parse '{}' from {}: {}", content, path.display(), e))
}
