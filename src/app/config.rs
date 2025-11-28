use std::{
    fs,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
};

use config::{Config as ConfigBuilder, Environment, File as ConfigFile};
use once_cell::sync::OnceCell;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::ConfigError;

// Global configuration manager instance
static CONFIG_MANAGER: OnceCell<ConfigManager> = OnceCell::new();

// Default configuration path: current working directory
// or AYIAH_DATA_DIR environment variable for Docker deployment
fn default_config_path() -> PathBuf {
    std::env::var("AYIAH_DATA_DIR").map_or_else(
        |_| {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join("ayiah.toml")
        },
        |data_dir| PathBuf::from(data_dir).join("ayiah.toml"),
    )
}

const ENVIRONMENT_PREFIX: &str = "AYIAH";

/// Configuration manager
#[derive(Debug, Clone)]
pub struct ConfigManager {
    config: Arc<RwLock<AppConfig>>,
    config_path: PathBuf,
}

// Application configuration structure
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub server: ServerConfig,

    #[serde(default)]
    pub logging: LoggingConfig,

    #[serde(default)]
    pub scraper: ScraperConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default)]
    pub host: String,

    #[serde(default)]
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 7590,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default)]
    pub level: String,

    #[serde(default)]
    pub file_path: Option<String>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            file_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScraperConfig {
    #[serde(default)]
    pub tmdb_api_key: Option<String>,

    #[serde(default)]
    pub tvdb_api_key: Option<String>,

    #[serde(default)]
    pub cache_ttl_seconds: u64,
}

impl Default for ScraperConfig {
    fn default() -> Self {
        Self {
            tmdb_api_key: None,
            tvdb_api_key: None,
            cache_ttl_seconds: 86400, // 24 hours
        }
    }
}

impl ConfigManager {
    /// Create a new configuration manager instance
    pub fn new<P: AsRef<Path>>(config_path: Option<P>) -> Result<Self, ConfigError> {
        let config_path =
            config_path.map_or_else(default_config_path, |p| p.as_ref().to_path_buf());

        let config = Self::load_config(&config_path)?;
        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            config_path,
        })
    }

    /// Initialize the global configuration manager instance
    pub fn init<P: AsRef<Path>>(config_path: Option<P>) -> Result<&'static Self, ConfigError> {
        let config_path =
            config_path.map_or_else(default_config_path, |p| p.as_ref().to_path_buf());

        info!("Initializing configuration from {:?}", config_path);

        let manager = CONFIG_MANAGER.get_or_init(|| match Self::new(Some(&config_path)) {
            Ok(manager) => manager,
            Err(e) => {
                panic!("Failed to initialize configuration: {e}");
            }
        });

        Ok(manager)
    }

    pub fn socket_addr(&self) -> Result<SocketAddr, ConfigError> {
        let config = self.config.read();
        let addr = format!("{}:{}", config.server.host, config.server.port)
            .parse::<SocketAddr>()
            .expect("Invalid server address configuration");
        drop(config);
        Ok(addr)
    }

    /// Get the global configuration manager instance
    pub fn instance() -> Result<&'static Self, ConfigError> {
        CONFIG_MANAGER.get().ok_or(ConfigError::NotInitialized)
    }

    /// Get a read lock on the configuration
    pub fn read(&self) -> parking_lot::RwLockReadGuard<'_, AppConfig> {
        self.config.read()
    }

    /// Get a write lock on the configuration
    pub fn write(&self) -> parking_lot::RwLockWriteGuard<'_, AppConfig> {
        self.config.write()
    }

    /// Reload the configuration
    pub fn reload(&self) -> Result<(), ConfigError> {
        let new_config = Self::load_config(&self.config_path)?;
        *self.config.write() = new_config;
        info!("Configuration reloaded successfully");
        Ok(())
    }

    /// Reload the configuration from a specific path
    pub fn reload_from<P: AsRef<Path>>(&self, config_path: P) -> Result<(), ConfigError> {
        let new_config = Self::load_config(config_path)?;
        *self.config.write() = new_config;
        info!("Configuration reloaded successfully");
        Ok(())
    }

    /// Load configuration from file and environment variables
    fn load_config<P: AsRef<Path>>(config_path: P) -> Result<AppConfig, ConfigError> {
        let config_path = config_path.as_ref();

        // Check if the configuration file exists, if not, create default configuration
        if !config_path.exists() {
            info!(
                "Configuration file not found, creating default configuration at {:?}",
                config_path
            );
            if let Some(parent) = config_path.parent()
                && !parent.exists()
            {
                fs::create_dir_all(parent).map_err(|e| {
                    ConfigError::WriteError(format!(
                        "Failed to create configuration directory: {e}"
                    ))
                })?;
            }

            let default_config = AppConfig::default();
            let toml_str = toml::to_string_pretty(&default_config)
                .map_err(|e| ConfigError::ParseError(e.to_string()))?;

            fs::write(config_path, toml_str).map_err(|e| {
                ConfigError::WriteError(format!("Failed to write configuration file: {e}"))
            })?;
        }

        // Build configuration, combining file and environment variables
        let config = ConfigBuilder::builder()
            // Load from default file
            .add_source(ConfigFile::from(config_path))
            // Load from environment variables with higher priority
            .add_source(
                Environment::with_prefix(ENVIRONMENT_PREFIX)
                    .separator("__")
                    .try_parsing(true),
            )
            .build()?;

        // Deserialize the configuration
        let app_config: AppConfig = config.try_deserialize()?;
        Ok(app_config)
    }
}
