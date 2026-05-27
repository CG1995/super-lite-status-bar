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
    memory_alert_fired: bool,
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
            memory_alert_fired: false,
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
            if self.cached_name.is_none() {
                if let Some(info) = sample_wmi_gpu() {
                    self.cached_name = info.name.clone();
                    return info;
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            if self.cached_name.is_none() {
                self.cached_name = sample_macos_gpu_name();
            }
            if let Some(info) = sample_macos_gpu_full() {
                return info;
            }
        }

        GpuInfo::default()
    }
}

// ── Windows: NVIDIA via nvidia-smi ──────────────────────────────────────────

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

// ── Windows: AMD / Intel via WMI ────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn sample_wmi_gpu() -> Option<GpuInfo> {
    let script = r#"
$gpu = Get-CimInstance Win32_VideoController | Where-Object { $_.Name -notmatch 'NVIDIA|nvidia|nv' } | Select-Object -First 1
if (-not $gpu) { exit 1 }
$vram = [uint64]$gpu.AdapterRAM
$name = $gpu.Name -replace '\s*\(R\)\s*',' ' -replace '\s+',' '
Write-Output "$name|0|$vram|$vram|0"
"#;

    let mut command = Command::new("powershell");
    set_hidden_window(&mut command);
    command.args(["-NoProfile", "-Command", script]);
    let output = command.output().ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.lines().find(|l| !l.trim().is_empty())?;
    let parts: Vec<&str> = line.split('|').collect();

    if parts.len() < 5 {
        return None;
    }

    let memory_bytes: u64 = parts[2].trim().parse().unwrap_or(0);

    Some(GpuInfo {
        name: Some(parts[0].trim().to_string()),
        usage_percent: None, // WMI VideoController doesn't give real-time usage
        memory_used_bytes: None,
        memory_total_bytes: if memory_bytes > 0 {
            Some(memory_bytes)
        } else {
            None
        },
        temperature_celsius: None,
        available: true,
        source: Some("wmi".to_string()),
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

// ── macOS: system_profiler for name + VRAM ──────────────────────────────────

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

#[cfg(target_os = "macos")]
fn sample_macos_gpu_full() -> Option<GpuInfo> {
    let output = Command::new("system_profiler")
        .args(["SPDisplaysDataType"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let name = stdout
        .lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix("Chipset Model:").map(str::trim))
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned);

    let vram_total = stdout
        .lines()
        .map(str::trim)
        .find_map(|line| {
            line.strip_prefix("VRAM")
                .or_else(|| line.strip_prefix("VRAM (Dynamic, Max):"))
                .or_else(|| line.strip_prefix("VRAM (Total):"))
                .map(str::trim)
        })
        .and_then(parse_vram_string);

    let usage = sample_macos_gpu_usage();

    let has_data = name.is_some() || vram_total.is_some() || usage.is_some();
    if !has_data {
        return None;
    }

    Some(GpuInfo {
        name,
        usage_percent: usage,
        memory_used_bytes: None,
        memory_total_bytes: vram_total,
        temperature_celsius: None,
        available: true,
        source: Some("system_profiler".to_string()),
    })
}

#[cfg(target_os = "macos")]
fn parse_vram_string(s: &str) -> Option<u64> {
    let s = s.trim();
    if let Some(gb) = s.strip_suffix(" GB") {
        gb.trim().parse::<f64>().ok().map(|v| (v * 1024.0 * 1024.0 * 1024.0) as u64)
    } else if let Some(mb) = s.strip_suffix(" MB") {
        mb.trim().parse::<f64>().ok().map(|v| (v * 1024.0 * 1024.0) as u64)
    } else {
        None
    }
}

#[cfg(target_os = "macos")]
fn sample_macos_gpu_usage() -> Option<f32> {
    let output = Command::new("ioreg")
        .args(["-r", "-d0", "-w0", "-c", "IOAccelerator"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .find_map(|line| {
            let trimmed = line.trim();
            trimmed
                .strip_prefix("\"Device Utilization %\" = ")
                .or_else(|| trimmed.strip_prefix("\"GPU Core Utilization\" = "))
                .or_else(|| trimmed.strip_prefix("\"GPU Utilization\" = "))
                .and_then(|v| v.trim().parse::<f32>().ok())
        })
        .map(|v| v.clamp(0.0, 100.0))
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

    #[cfg(target_os = "macos")]
    mod macos_tests {
        use super::super::parse_vram_string;

        #[test]
        fn parses_vram_gb() {
            assert_eq!(parse_vram_string("16 GB"), Some(16 * 1024 * 1024 * 1024));
        }

        #[test]
        fn parses_vram_mb() {
            assert_eq!(
                parse_vram_string("1536 MB"),
                Some(1536u64 * 1024 * 1024)
            );
        }

        #[test]
        fn rejects_bad_vram() {
            assert_eq!(parse_vram_string("N/A"), None);
        }
    }
}
