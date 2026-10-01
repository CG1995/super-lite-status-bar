use crate::core::identity::{APP_NAME, APP_ORGANIZATION, APP_QUALIFIER};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error;

const CONFIG_VERSION: u32 = 2;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("config directory is not available")]
    MissingConfigDir,
    #[error("failed to read config: {0}")]
    Read(#[from] io::Error),
    #[error("failed to parse config: {0}")]
    Parse(#[from] serde_json::Error),
}

#[derive(Debug, Clone)]
pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    pub fn default_path() -> Result<PathBuf, ConfigError> {
        let dirs = ProjectDirs::from(APP_QUALIFIER, APP_ORGANIZATION, APP_NAME)
            .ok_or(ConfigError::MissingConfigDir)?;
        Ok(dirs.config_dir().join("config.json"))
    }

    pub fn new_default() -> Result<Self, ConfigError> {
        Ok(Self {
            path: Self::default_path()?,
        })
    }

    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn load(&self) -> AppConfig {
        match Self::load_from_path(&self.path) {
            Ok(config) => config.sanitized(),
            Err(err) => {
                if self.path.exists() {
                    let _ = self.backup_corrupt_file();
                }
                tracing::warn!(error = %err, path = %self.path.display(), "using default config");
                let default_config = AppConfig::default();
                let _ = self.save(&default_config);
                default_config
            }
        }
    }

    pub fn load_from_path(path: &Path) -> Result<AppConfig, ConfigError> {
        let raw = fs::read_to_string(path)?;
        let config = serde_json::from_str::<AppConfig>(&raw)?;
        Ok(config.sanitized())
    }

    pub fn save(&self, config: &AppConfig) -> Result<(), ConfigError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let config = config.clone().sanitized();
        let serialized = serde_json::to_string_pretty(&config)?;
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, serialized)?;
        if self.path.exists() {
            fs::remove_file(&self.path)?;
        }
        fs::rename(tmp, &self.path)?;
        Ok(())
    }

    fn backup_corrupt_file(&self) -> Result<(), ConfigError> {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        let backup = self.path.with_extension(format!("bad-{stamp}.json"));
        fs::copy(&self.path, backup)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum FontPreset {
    #[default]
    Small,
    Medium,
    Large,
    Custom,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeMode {
    #[default]
    System,
    Dark,
    Light,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SpeedUnit {
    #[default]
    Auto,
    Kb,
    Mb,
}

/// How eagerly metrics escalate to the warning / critical colors.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AlertSensitivity {
    Relaxed,
    #[default]
    Standard,
    Sensitive,
}

/// Warning and critical thresholds (percent) for one metric.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Threshold {
    pub warn: f32,
    pub critical: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Thresholds {
    pub cpu: Threshold,
    pub memory: Threshold,
    pub gpu: Threshold,
}

impl AlertSensitivity {
    pub fn thresholds(self) -> Thresholds {
        let t = |warn, critical| Threshold { warn, critical };
        match self {
            Self::Relaxed => Thresholds {
                cpu: t(80.0, 95.0),
                memory: t(88.0, 95.0),
                gpu: t(85.0, 97.0),
            },
            Self::Standard => Thresholds {
                cpu: t(70.0, 90.0),
                memory: t(80.0, 92.0),
                gpu: t(75.0, 92.0),
            },
            Self::Sensitive => Thresholds {
                cpu: t(55.0, 80.0),
                memory: t(70.0, 85.0),
                gpu: t(60.0, 85.0),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct FontConfig {
    pub preset: FontPreset,
    pub custom_px: u8,
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            preset: FontPreset::Small,
            custom_px: 12,
        }
    }
}

impl FontConfig {
    pub fn effective_px(&self) -> u8 {
        match self.preset {
            FontPreset::Small => 12,
            FontPreset::Medium => 13,
            FontPreset::Large => 15,
            FontPreset::Custom => self.custom_px.clamp(11, 20),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct FloatingBarConfig {
    pub enabled: bool,
    /// Opacity of the bar background only; text always stays fully opaque.
    pub opacity: f32,
    pub always_on_top: bool,
    pub lock_position: bool,
    pub click_through: bool,
    pub show_memory: bool,
    pub show_gpu: bool,
    pub show_network: bool,
    pub x: Option<f64>,
    pub y: Option<f64>,
}

impl Default for FloatingBarConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            opacity: 0.85,
            always_on_top: true,
            lock_position: false,
            click_through: false,
            show_memory: true,
            show_gpu: true,
            show_network: true,
            x: None,
            y: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AppConfig {
    pub version: u32,
    pub autostart: bool,
    pub refresh_interval_ms: u64,
    pub font: FontConfig,
    pub speed_unit: SpeedUnit,
    pub floating_bar: FloatingBarConfig,
    pub theme: ThemeMode,
    pub alert: AlertSensitivity,
    pub show_na: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            autostart: false,
            refresh_interval_ms: 1_000,
            font: FontConfig::default(),
            speed_unit: SpeedUnit::Auto,
            floating_bar: FloatingBarConfig::default(),
            theme: ThemeMode::System,
            alert: AlertSensitivity::Standard,
            show_na: true,
        }
    }
}

const REFRESH_STEPS_MS: [u64; 4] = [500, 1_000, 2_000, 3_000];

impl AppConfig {
    pub fn sanitized(mut self) -> Self {
        self.version = CONFIG_VERSION;
        // Snap to the nearest supported refresh step.
        self.refresh_interval_ms = REFRESH_STEPS_MS
            .into_iter()
            .min_by_key(|step| step.abs_diff(self.refresh_interval_ms))
            .unwrap_or(1_000);
        self.font.custom_px = self.font.custom_px.clamp(11, 20);
        self.speed_unit = SpeedUnit::Auto;
        self.show_na = true;
        self.floating_bar.opacity = if self.floating_bar.opacity.is_finite() {
            self.floating_bar.opacity.clamp(0.0, 1.0)
        } else {
            FloatingBarConfig::default().opacity
        };
        for coordinate in [&mut self.floating_bar.x, &mut self.floating_bar.y] {
            if coordinate.is_some_and(|value| !value.is_finite()) {
                *coordinate = None;
            }
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::identity::APP_SLUG;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_config_path(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{name}-{stamp}.json"))
    }

    #[test]
    fn saves_and_loads_config() {
        let path = temp_config_path(APP_SLUG);
        let store = ConfigStore::new(&path);
        let config = AppConfig {
            refresh_interval_ms: 5_000,
            font: FontConfig {
                preset: FontPreset::Large,
                ..FontConfig::default()
            },
            speed_unit: SpeedUnit::Mb,
            show_na: false,
            ..AppConfig::default()
        };

        store.save(&config).unwrap();
        let loaded = ConfigStore::load_from_path(&path).unwrap();

        assert_eq!(loaded.refresh_interval_ms, 3_000);
        assert_eq!(loaded.font.preset, FontPreset::Large);
        assert_eq!(loaded.speed_unit, SpeedUnit::Auto);
        assert!(loaded.show_na);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn sanitizes_unsafe_values() {
        let config = AppConfig {
            refresh_interval_ms: 50,
            font: FontConfig {
                custom_px: 4,
                ..FontConfig::default()
            },
            floating_bar: FloatingBarConfig {
                opacity: 4.0,
                ..FloatingBarConfig::default()
            },
            ..AppConfig::default()
        };

        let config = config.sanitized();

        assert_eq!(config.refresh_interval_ms, 500);
        assert_eq!(config.font.custom_px, 11);
        assert_eq!(config.floating_bar.opacity, 1.0);
    }

    #[test]
    fn loads_v1_config_with_new_defaults() {
        let raw = r#"{"version":1,"autostart":true,"floating_bar":{"enabled":true,"opacity":0.8}}"#;
        let config = serde_json::from_str::<AppConfig>(raw).unwrap().sanitized();

        assert_eq!(config.version, CONFIG_VERSION);
        assert!(config.floating_bar.enabled);
        assert!(config.floating_bar.show_gpu);
        assert_eq!(config.alert, AlertSensitivity::Standard);
        assert_eq!(config.floating_bar.opacity, 0.8);
    }
}
