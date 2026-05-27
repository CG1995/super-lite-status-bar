use crate::core::identity::{APP_NAME, APP_ORGANIZATION, APP_QUALIFIER, APP_SLUG};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error;

const CONFIG_VERSION: u32 = 1;

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum FontPreset {
    Small,
    Medium,
    Large,
    Custom,
}

impl Default for FontPreset {
    fn default() -> Self {
        Self::Small
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeMode {
    System,
    Dark,
    Light,
}

impl Default for ThemeMode {
    fn default() -> Self {
        Self::System
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SpeedUnit {
    Auto,
    Kb,
    Mb,
}

impl Default for SpeedUnit {
    fn default() -> Self {
        Self::Auto
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
            FontPreset::Medium => 14,
            FontPreset::Large => 16,
            FontPreset::Custom => self.custom_px.clamp(12, 28),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct FloatingBarConfig {
    pub enabled: bool,
    pub opacity: f32,
    pub always_on_top: bool,
    pub lock_position: bool,
    pub click_through: bool,
    pub x: Option<f64>,
    pub y: Option<f64>,
}

impl Default for FloatingBarConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            opacity: 0.92,
            always_on_top: true,
            lock_position: false,
            click_through: false,
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
            show_na: true,
        }
    }
}

impl AppConfig {
    pub fn sanitized(mut self) -> Self {
        self.version = CONFIG_VERSION;
        self.refresh_interval_ms = 1_000;
        self.font.preset = FontPreset::Small;
        self.font.custom_px = self.font.custom_px.clamp(12, 28);
        self.speed_unit = SpeedUnit::Auto;
        self.show_na = true;
        self.floating_bar.opacity = self.floating_bar.opacity.clamp(0.35, 1.0);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let mut config = AppConfig::default();
        config.refresh_interval_ms = 5_000;
        config.font.preset = FontPreset::Large;
        config.speed_unit = SpeedUnit::Mb;
        config.show_na = false;

        store.save(&config).unwrap();
        let loaded = ConfigStore::load_from_path(&path).unwrap();

        assert_eq!(loaded.refresh_interval_ms, 1_000);
        assert_eq!(loaded.font.preset, FontPreset::Small);
        assert_eq!(loaded.speed_unit, SpeedUnit::Auto);
        assert!(loaded.show_na);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn sanitizes_unsafe_values() {
        let mut config = AppConfig::default();
        config.refresh_interval_ms = 50;
        config.font.custom_px = 4;
        config.floating_bar.opacity = 4.0;

        let config = config.sanitized();

        assert_eq!(config.refresh_interval_ms, 1_000);
        assert_eq!(config.font.custom_px, 12);
        assert_eq!(config.floating_bar.opacity, 1.0);
    }
}
