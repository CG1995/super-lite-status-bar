use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct NetworkCounters {
    pub received_bytes: u64,
    pub transmitted_bytes: u64,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub struct NetworkSpeed {
    pub download_bps: f64,
    pub upload_bps: f64,
}

#[derive(Debug, Clone, Default)]
pub struct NetworkSpeedCalculator {
    previous: Option<NetworkCounters>,
}

impl NetworkSpeedCalculator {
    pub fn new() -> Self {
        Self { previous: None }
    }

    pub fn update(&mut self, current: NetworkCounters, elapsed: Duration) -> NetworkSpeed {
        let speed = match self.previous {
            Some(previous) => Self::calculate(previous, current, elapsed),
            None => NetworkSpeed::default(),
        };
        self.previous = Some(current);
        speed
    }

    pub fn calculate(
        previous: NetworkCounters,
        current: NetworkCounters,
        elapsed: Duration,
    ) -> NetworkSpeed {
        let elapsed_seconds = elapsed.as_secs_f64();
        if !(0.1..=3_600.0).contains(&elapsed_seconds) {
            return NetworkSpeed::default();
        }

        let down_delta = current.received_bytes.checked_sub(previous.received_bytes);
        let up_delta = current
            .transmitted_bytes
            .checked_sub(previous.transmitted_bytes);

        match (down_delta, up_delta) {
            (Some(down), Some(up)) => NetworkSpeed {
                download_bps: clamp_implausible_speed(down as f64 / elapsed_seconds),
                upload_bps: clamp_implausible_speed(up as f64 / elapsed_seconds),
            },
            _ => NetworkSpeed::default(),
        }
    }
}

fn clamp_implausible_speed(value: f64) -> f64 {
    const MAX_REASONABLE_BPS: f64 = 100.0 * 1024.0 * 1024.0 * 1024.0;
    if value.is_finite() && value <= MAX_REASONABLE_BPS {
        value.max(0.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculates_speed_from_counter_delta() {
        let previous = NetworkCounters {
            received_bytes: 1_000,
            transmitted_bytes: 2_000,
        };
        let current = NetworkCounters {
            received_bytes: 5_000,
            transmitted_bytes: 6_000,
        };

        let speed = NetworkSpeedCalculator::calculate(previous, current, Duration::from_secs(2));

        assert_eq!(speed.download_bps, 2_000.0);
        assert_eq!(speed.upload_bps, 2_000.0);
    }

    #[test]
    fn handles_counter_reset_without_negative_speed() {
        let previous = NetworkCounters {
            received_bytes: 10_000,
            transmitted_bytes: 10_000,
        };
        let current = NetworkCounters {
            received_bytes: 1_000,
            transmitted_bytes: 2_000,
        };

        let speed = NetworkSpeedCalculator::calculate(previous, current, Duration::from_secs(1));

        assert_eq!(speed, NetworkSpeed::default());
    }
}
