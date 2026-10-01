use crate::core::{
    config::{AppConfig, SpeedUnit, Threshold},
    gpu::{GpuInfo, GpuSampler},
    network_speed::{NetworkCounters, NetworkSpeed, NetworkSpeedCalculator},
};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use sysinfo::{Networks, System};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum PressureLevel {
    #[default]
    Normal,
    Medium,
    High,
}

/// Points a value must fall below a threshold before the level steps back down,
/// so colors do not flicker when a metric hovers around a boundary.
const HYSTERESIS: f32 = 4.0;

impl PressureLevel {
    fn classify(value: f32, threshold: Threshold, previous: Self) -> Self {
        let raw = if value >= threshold.critical {
            Self::High
        } else if value >= threshold.warn {
            Self::Medium
        } else {
            Self::Normal
        };
        if raw >= previous {
            return raw;
        }
        // De-escalate only once the value clears the threshold by the hysteresis margin.
        let held = match previous {
            Self::High if value >= threshold.critical - HYSTERESIS => Self::High,
            Self::High | Self::Medium if value >= threshold.warn - HYSTERESIS => Self::Medium,
            _ => Self::Normal,
        };
        held.max(raw)
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct MetricLevels {
    pub cpu: PressureLevel,
    pub memory: PressureLevel,
    pub gpu: PressureLevel,
}

impl MetricLevels {
    pub fn overall(self) -> PressureLevel {
        self.cpu.max(self.memory).max(self.gpu)
    }
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
    pub levels: MetricLevels,
    pub pressure: PressureLevel,
    /// Value (0-100) of the metric that drives the overall level; drawn as the tray ring arc.
    pub focus_percent: f32,
    pub compact_text: String,
}

pub struct SystemMetricsSampler {
    system: System,
    networks: Networks,
    network_calculator: NetworkSpeedCalculator,
    gpu_sampler: GpuSampler,
    last_network_sample: Instant,
    levels: MetricLevels,
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
            levels: MetricLevels::default(),
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
        let thresholds = config.alert.thresholds();
        let previous = self.levels;
        let levels = MetricLevels {
            cpu: PressureLevel::classify(cpu_percent, thresholds.cpu, previous.cpu),
            memory: PressureLevel::classify(memory.percent, thresholds.memory, previous.memory),
            gpu: gpu
                .usage_percent
                .map(|usage| PressureLevel::classify(usage, thresholds.gpu, previous.gpu))
                .unwrap_or_default(),
        };
        self.levels = levels;
        let pressure = levels.overall();
        let focus_percent = focus_percent(cpu_percent, memory.percent, gpu.usage_percent, levels);
        let compact_text = format_compact(cpu_percent, &memory, network, config);

        MetricsSnapshot {
            cpu_percent,
            memory,
            network,
            gpu,
            levels,
            pressure,
            focus_percent,
            compact_text,
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

fn focus_percent(cpu: f32, memory: f32, gpu: Option<f32>, levels: MetricLevels) -> f32 {
    let overall = levels.overall();
    if overall == PressureLevel::Normal || levels.cpu == overall {
        return cpu;
    }
    if levels.memory == overall {
        return memory;
    }
    gpu.unwrap_or(cpu)
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
    fn levels_escalate_immediately_and_recover_with_hysteresis() {
        let t = Threshold {
            warn: 70.0,
            critical: 90.0,
        };
        let level = PressureLevel::classify(91.0, t, PressureLevel::Normal);
        assert_eq!(level, PressureLevel::High);
        // Small dip below critical keeps the level.
        assert_eq!(PressureLevel::classify(88.0, t, level), PressureLevel::High);
        // Clear drop steps down to warning, then to normal.
        assert_eq!(
            PressureLevel::classify(80.0, t, level),
            PressureLevel::Medium
        );
        assert_eq!(
            PressureLevel::classify(68.0, t, PressureLevel::Medium),
            PressureLevel::Medium
        );
        assert_eq!(
            PressureLevel::classify(60.0, t, PressureLevel::Medium),
            PressureLevel::Normal
        );
    }

    #[test]
    fn focus_follows_the_hottest_metric() {
        let levels = MetricLevels {
            cpu: PressureLevel::Normal,
            memory: PressureLevel::High,
            gpu: PressureLevel::Medium,
        };
        assert_eq!(focus_percent(20.0, 93.0, Some(80.0), levels), 93.0);
        assert_eq!(
            focus_percent(20.0, 50.0, None, MetricLevels::default()),
            20.0
        );
    }

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
