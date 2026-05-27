use crate::core::{
    config::{AppConfig, SpeedUnit},
    gpu::{GpuInfo, GpuSampler},
    network_speed::{NetworkCounters, NetworkSpeed, NetworkSpeedCalculator},
};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use sysinfo::{Networks, System};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PressureLevel {
    Normal,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryInfo {
    pub used_bytes: u64,
    pub total_bytes: u64,
    pub percent: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetricsSnapshot {
    pub cpu_percent: f32,
    pub memory: MemoryInfo,
    pub network: NetworkSpeed,
    pub gpu: GpuInfo,
    pub pressure: PressureLevel,
    pub compact_text: String,
    pub full_text: String,
    pub tooltip: String,
}

pub struct SystemMetricsSampler {
    system: System,
    networks: Networks,
    network_calculator: NetworkSpeedCalculator,
    gpu_sampler: GpuSampler,
    last_network_sample: Instant,
}

impl Default for SystemMetricsSampler {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemMetricsSampler {
    pub fn new() -> Self {
        let mut system = System::new();
        system.refresh_memory();
        system.refresh_cpu_usage();
        let networks = Networks::new_with_refreshed_list();

        Self {
            system,
            networks,
            network_calculator: NetworkSpeedCalculator::new(),
            gpu_sampler: GpuSampler::new(),
            last_network_sample: Instant::now(),
        }
    }

    pub fn sample(&mut self, config: &AppConfig) -> MetricsSnapshot {
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        self.networks.refresh(true);

        let now = Instant::now();
        let elapsed = now
            .checked_duration_since(self.last_network_sample)
            .unwrap_or(Duration::from_secs(0));
        self.last_network_sample = now;

        let counters = self.network_counters();
        let network = self.network_calculator.update(counters, elapsed);
        let memory = memory_info(&self.system);
        let cpu_percent = self.system.global_cpu_usage().clamp(0.0, 100.0);
        let gpu = self.gpu_sampler.sample();
        let pressure = pressure_level(cpu_percent, memory.percent, gpu.usage_percent);

        let compact_text = format_compact(cpu_percent, &memory, network, config);
        let full_text = format_full(cpu_percent, &memory, network, &gpu, config);
        let tooltip = format_tooltip(cpu_percent, &memory, network, &gpu, config);

        MetricsSnapshot {
            cpu_percent,
            memory,
            network,
            gpu,
            pressure,
            compact_text,
            full_text,
            tooltip,
        }
    }

    fn network_counters(&self) -> NetworkCounters {
        self.networks
            .iter()
            .fold(NetworkCounters::default(), |mut counters, (_name, data)| {
                counters.received_bytes = counters
                    .received_bytes
                    .saturating_add(data.total_received());
                counters.transmitted_bytes = counters
                    .transmitted_bytes
                    .saturating_add(data.total_transmitted());
                counters
            })
    }
}

fn memory_info(system: &System) -> MemoryInfo {
    let total_bytes = system.total_memory();
    let used_bytes = system.used_memory().min(total_bytes);
    let percent = if total_bytes == 0 {
        0.0
    } else {
        (used_bytes as f32 / total_bytes as f32 * 100.0).clamp(0.0, 100.0)
    };

    MemoryInfo {
        used_bytes,
        total_bytes,
        percent,
    }
}

fn pressure_level(cpu: f32, memory: f32, gpu: Option<f32>) -> PressureLevel {
    let _gpu = gpu.unwrap_or(0.0);
    if memory >= 95.0 {
        PressureLevel::High
    } else if cpu.max(memory).max(_gpu) >= 85.0 {
        PressureLevel::Medium
    } else if cpu.max(memory).max(_gpu) >= 65.0 {
        PressureLevel::Normal
    } else {
        PressureLevel::Normal
    }
}

fn format_compact(
    cpu: f32,
    memory: &MemoryInfo,
    network: NetworkSpeed,
    config: &AppConfig,
) -> String {
    format!(
        "CPU {:.0}% | MEM {:.0}% | ↓ {} | ↑ {}",
        cpu,
        memory.percent,
        format_speed_short(network.download_bps, &config.speed_unit),
        format_speed_short(network.upload_bps, &config.speed_unit)
    )
}

fn format_full(
    cpu: f32,
    memory: &MemoryInfo,
    network: NetworkSpeed,
    gpu: &GpuInfo,
    config: &AppConfig,
) -> String {
    let lines = vec![
        format!("CPU {:.0}%", cpu),
        format!(
            "MEM {} / {}",
            format_bytes(memory.used_bytes),
            format_bytes(memory.total_bytes)
        ),
        format!(
            "GPU {} {}",
            format_optional_value(
                gpu.usage_percent.map(|value| format!("{value:.0}%")),
                config.show_na
            ),
            short_gpu_name(gpu.name.as_deref())
        ),
        format_vram(gpu, config.show_na),
        format!(
            "NET ↓ {} ↑ {}",
            format_speed(network.download_bps, &config.speed_unit),
            format_speed(network.upload_bps, &config.speed_unit)
        ),
    ];
    lines
        .into_iter()
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_tooltip(
    cpu: f32,
    memory: &MemoryInfo,
    network: NetworkSpeed,
    gpu: &GpuInfo,
    config: &AppConfig,
) -> String {
    let gpu_name = short_gpu_name(gpu.name.as_deref());
    format!(
        "CPU: {:.0}%\nMemory: {} / {} ({:.0}%)\nGPU: {} {}\nVRAM: {}\nNET: ↓ {} ↑ {}",
        cpu,
        format_bytes(memory.used_bytes),
        format_bytes(memory.total_bytes),
        memory.percent,
        format_optional_value(
            gpu.usage_percent.map(|value| format!("{value:.0}%")),
            config.show_na
        ),
        gpu_name,
        format_vram_value(gpu, config.show_na),
        format_speed(network.download_bps, &config.speed_unit),
        format_speed(network.upload_bps, &config.speed_unit)
    )
}

fn format_optional_value(value: Option<String>, show_na: bool) -> String {
    match value {
        Some(value) => value,
        None if show_na => "N/A".to_string(),
        None => String::new(),
    }
}

fn format_vram(gpu: &GpuInfo, show_na: bool) -> String {
    match (gpu.memory_used_bytes, gpu.memory_total_bytes) {
        (Some(used), Some(total)) => {
            format!("VRAM {} / {}", format_bytes(used), format_bytes(total))
        }
        _ if show_na => "VRAM N/A".to_string(),
        _ => String::new(),
    }
}

fn format_vram_value(gpu: &GpuInfo, show_na: bool) -> String {
    match (gpu.memory_used_bytes, gpu.memory_total_bytes) {
        (Some(used), Some(total)) => format!("{} / {}", format_bytes(used), format_bytes(total)),
        _ if show_na => "N/A".to_string(),
        _ => String::new(),
    }
}

pub fn short_gpu_name(name: Option<&str>) -> String {
    let Some(name) = name else {
        return "N/A".to_string();
    };
    let cleaned = name
        .replace("NVIDIA", "")
        .replace("GeForce", "")
        .replace("Laptop GPU", "")
        .replace("GPU", "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let value = if cleaned.is_empty() {
        name.trim()
    } else {
        cleaned.trim()
    };
    value
        .split_whitespace()
        .find(|part| part.chars().any(|ch| ch.is_ascii_digit()) && part.chars().count() >= 3)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| value.to_string())
}

pub fn format_bytes(bytes: u64) -> String {
    let gib = bytes as f64 / 1024.0 / 1024.0 / 1024.0;
    if gib >= 1.0 {
        format!("{gib:.1} GB")
    } else {
        let mib = bytes as f64 / 1024.0 / 1024.0;
        format!("{mib:.0} MB")
    }
}

pub fn format_speed(bytes_per_second: f64, unit: &SpeedUnit) -> String {
    match unit {
        SpeedUnit::Kb => format!("{:.0} KB/s", bytes_per_second / 1024.0),
        SpeedUnit::Mb => format!("{:.1} MB/s", bytes_per_second / 1024.0 / 1024.0),
        SpeedUnit::Auto => {
            if bytes_per_second >= 1024.0 * 1024.0 {
                format!("{:.1} MB/s", bytes_per_second / 1024.0 / 1024.0)
            } else {
                format!("{:.0} KB/s", bytes_per_second / 1024.0)
            }
        }
    }
}

pub fn format_speed_short(bytes_per_second: f64, unit: &SpeedUnit) -> String {
    let full = format_speed(bytes_per_second, unit);
    full.replace(" MB/s", "M").replace(" KB/s", "K")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::AppConfig;

    #[test]
    fn formats_speed_in_auto_units() {
        assert_eq!(format_speed(2_200.0, &SpeedUnit::Auto), "2 KB/s");
        assert_eq!(format_speed(2_621_440.0, &SpeedUnit::Auto), "2.5 MB/s");
    }

    #[test]
    fn formats_compact_metrics() {
        let config = AppConfig::default();
        let memory = MemoryInfo {
            used_bytes: 8 * 1024 * 1024 * 1024,
            total_bytes: 16 * 1024 * 1024 * 1024,
            percent: 50.0,
        };
        let text = format_compact(
            12.0,
            &memory,
            NetworkSpeed {
                download_bps: 2_097_152.0,
                upload_bps: 307_200.0,
            },
            &config,
        );

        assert_eq!(text, "CPU 12% | MEM 50% | ↓ 2.0M | ↑ 300K");
    }
}
