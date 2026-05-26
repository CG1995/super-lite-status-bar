use serde::{Deserialize, Serialize};
use std::{
    process::Command,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct GpuInfo {
    pub name: Option<String>,
    pub usage_percent: Option<f32>,
    pub memory_used_bytes: Option<u64>,
    pub memory_total_bytes: Option<u64>,
    pub temperature_celsius: Option<f32>,
    pub available: bool,
    pub source: Option<String>,
}

#[derive(Debug)]
pub struct GpuSampler {
    cached_name: Option<String>,
    last_attempt: Option<Instant>,
}

impl Default for GpuSampler {
    fn default() -> Self {
        Self::new()
    }
}

impl GpuSampler {
    pub fn new() -> Self {
        Self {
            cached_name: None,
            last_attempt: None,
        }
    }

    pub fn sample(&mut self) -> GpuInfo {
        if let Some(last_attempt) = self.last_attempt {
            if last_attempt.elapsed() < Duration::from_secs(5) && self.cached_name.is_none() {
                return GpuInfo::default();
            }
        }
        self.last_attempt = Some(Instant::now());

        #[cfg(target_os = "windows")]
        {
            if let Some(info) = sample_nvidia_smi() {
                self.cached_name = info.name.clone();
                return info;
            }
        }

        #[cfg(target_os = "macos")]
        {
            if self.cached_name.is_none() {
                self.cached_name = sample_macos_gpu_name();
            }
            if let Some(name) = &self.cached_name {
                return GpuInfo {
                    name: Some(name.clone()),
                    available: true,
                    source: Some("system_profiler".to_string()),
                    ..GpuInfo::default()
                };
            }
        }

        GpuInfo::default()
    }
}

#[cfg(target_os = "windows")]
fn sample_nvidia_smi() -> Option<GpuInfo> {
    let mut command = Command::new("nvidia-smi");
    command.args([
        "--query-gpu=name,utilization.gpu,memory.used,memory.total,temperature.gpu",
        "--format=csv,noheader,nounits",
    ]);
    set_hidden_window(&mut command);

    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let first = stdout.lines().find(|line| !line.trim().is_empty())?;
    parse_nvidia_smi_line(first)
}

#[cfg(target_os = "windows")]
fn parse_nvidia_smi_line(line: &str) -> Option<GpuInfo> {
    let parts = line.split(',').map(|part| part.trim()).collect::<Vec<_>>();
    if parts.len() < 5 {
        return None;
    }

    let memory_used_mib = parts[2].parse::<u64>().ok();
    let memory_total_mib = parts[3].parse::<u64>().ok();

    Some(GpuInfo {
        name: Some(parts[0].to_string()),
        usage_percent: parts[1].parse::<f32>().ok(),
        memory_used_bytes: memory_used_mib.map(mib_to_bytes),
        memory_total_bytes: memory_total_mib.map(mib_to_bytes),
        temperature_celsius: parts[4].parse::<f32>().ok(),
        available: true,
        source: Some("nvidia-smi".to_string()),
    })
}

#[cfg(target_os = "windows")]
fn mib_to_bytes(value: u64) -> u64 {
    value.saturating_mul(1024 * 1024)
}

#[cfg(target_os = "windows")]
fn set_hidden_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(target_os = "macos")]
fn sample_macos_gpu_name() -> Option<String> {
    let output = Command::new("system_profiler")
        .args(["SPDisplaysDataType"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix("Chipset Model:").map(str::trim))
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "windows")]
    use super::parse_nvidia_smi_line;

    #[cfg(target_os = "windows")]
    #[test]
    fn parses_nvidia_smi_output() {
        let info = parse_nvidia_smi_line("NVIDIA RTX 4070, 12, 1024, 8192, 45").unwrap();

        assert_eq!(info.name.as_deref(), Some("NVIDIA RTX 4070"));
        assert_eq!(info.usage_percent, Some(12.0));
        assert_eq!(info.memory_used_bytes, Some(1024 * 1024 * 1024));
        assert_eq!(info.temperature_celsius, Some(45.0));
    }
}
