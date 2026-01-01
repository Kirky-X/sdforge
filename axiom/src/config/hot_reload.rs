//! Configuration hot reload support
//!
//! This module provides configuration hot-reload functionality using file system watching.

#[cfg(feature = "hot-reload")]
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
#[cfg(feature = "hot-reload")]
use std::path::{Path, PathBuf};
#[cfg(feature = "hot-reload")]
use std::sync::Arc;
#[cfg(feature = "hot-reload")]
use std::sync::RwLock;
#[cfg(feature = "hot-reload")]
use tokio::sync::broadcast;

#[cfg(feature = "hot-reload")]
use crate::AppConfig;

#[cfg(feature = "hot-reload")]
/// Configuration event type
#[derive(Debug, Clone)]
pub enum ConfigEvent {
    /// Configuration was successfully reloaded
    ///
    /// Contains the new configuration that was loaded.
    Reloaded(Box<AppConfig>),
    /// An error occurred during reload
    ///
    /// Contains the error message describing what went wrong.
    Error(String),
}

#[cfg(feature = "hot-reload")]
/// Configuration error
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Configuration file not found
    #[error("Configuration file not found: {0}")]
    NotFound(PathBuf),
    /// Configuration path is not a file
    #[error("Configuration path is not a file: {0}")]
    NotAFile(PathBuf),
    /// Configuration file is outside allowed directory
    #[error("Configuration file is outside allowed directory")]
    OutsideAllowedDirectory,
}

#[cfg(feature = "hot-reload")]
/// Configuration watcher for hot reload
pub struct ConfigWatcher {
    config_path: PathBuf,
    current_config: Arc<RwLock<AppConfig>>,
    event_sender: broadcast::Sender<ConfigEvent>,
}

#[cfg(feature = "hot-reload")]
impl ConfigWatcher {
    /// Validate configuration file path
    fn validate_config_path(path: &Path) -> Result<(), ConfigError> {
        if !path.exists() {
            return Err(ConfigError::NotFound(path.to_path_buf()));
        }
        if !path.is_file() {
            return Err(ConfigError::NotAFile(path.to_path_buf()));
        }

        // Check if path is within a reasonable directory structure
        // (prevent path traversal attacks)
        let path_str = path.to_string_lossy();
        if path_str.contains("..") {
            return Err(ConfigError::OutsideAllowedDirectory);
        }

        Ok(())
    }

    /// Create a new configuration watcher
    pub fn new(
        config_path: PathBuf,
    ) -> Result<(Self, broadcast::Receiver<ConfigEvent>), Box<dyn std::error::Error>> {
        // Validate configuration path
        Self::validate_config_path(&config_path)?;

        let (event_sender, event_receiver) = broadcast::channel(100);

        // Load initial configuration
        let config = crate::config::ConfigLoader::new(config_path.clone(), "AXIOM").load()?;

        let watcher = Self {
            config_path,
            current_config: Arc::new(RwLock::new(config)),
            event_sender,
        };

        Ok((watcher, event_receiver))
    }

    /// Get current configuration
    pub fn get(&self) -> AppConfig {
        self.current_config
            .read()
            .expect("Config lock poisoned")
            .clone()
    }

    /// Reload configuration
    pub async fn reload(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config = crate::config::ConfigLoader::new(self.config_path.clone(), "AXIOM").load()?;
        *self.current_config.write().expect("Config lock poisoned") = config.clone();
        let _ = self
            .event_sender
            .send(ConfigEvent::Reloaded(Box::new(config)));
        Ok(())
    }

    /// Start watching the configuration file for changes
    ///
    /// This method spawns a background task that monitors the configuration file
    /// and automatically reloads it when changes are detected.
    ///
    /// Returns a `RecommendedWatcher` that should be kept alive to maintain the file watcher.
    /// Drop the watcher to stop monitoring.
    pub fn watch(&self) -> Result<RecommendedWatcher, Box<dyn std::error::Error>> {
        let config_path = self.config_path.clone();
        let current_config = self.current_config.clone();
        let event_sender = self.event_sender.clone();

        // Create a channel to receive file system events
        let (_tx, rx) = std::sync::mpsc::channel::<notify::Result<Event>>();

        // Create the file system watcher
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
            match res {
                Ok(event) => {
                    // Only reload on file modification events
                    if event.kind.is_modify() {
                        // Use tokio runtime to spawn async reload task
                        let config_path_clone = config_path.clone();
                        let current_config_clone = current_config.clone();
                        let event_sender_clone = event_sender.clone();

                        tokio::spawn(async move {
                            // Load new configuration
                            match crate::config::ConfigLoader::new(config_path_clone, "AXIOM")
                                .load()
                            {
                                Ok(new_config) => {
                                    // Update current configuration
                                    *current_config_clone.write().expect("Config lock poisoned") =
                                        new_config.clone();

                                    // Send reload event
                                    let _ = event_sender_clone
                                        .send(ConfigEvent::Reloaded(Box::new(new_config)));
                                }
                                Err(e) => {
                                    // Send error event
                                    let _ = event_sender_clone.send(ConfigEvent::Error(format!(
                                        "Failed to reload configuration: {}",
                                        e
                                    )));
                                }
                            }
                        });
                    }
                }
                Err(e) => {
                    // Send error event for watcher errors
                    let _ = event_sender.send(ConfigEvent::Error(format!("Watcher error: {}", e)));
                }
            }
        })?;

        // Start watching the configuration file
        watcher.watch(&self.config_path, RecursiveMode::NonRecursive)?;

        // Spawn a task to process events from the channel
        tokio::spawn(async move {
            while let Ok(_event) = rx.recv() {
                // Events are already handled in the watcher callback
                // This loop just keeps the receiver alive
            }
        });

        Ok(watcher)
    }
}

#[cfg(feature = "hot-reload")]
/// Configuration manager for hot reload
pub struct ConfigManager {
    config: Arc<RwLock<AppConfig>>,
}

#[cfg(feature = "hot-reload")]
impl ConfigManager {
    /// Create a new configuration manager
    pub fn new(config: AppConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
        }
    }

    /// Get current configuration
    pub fn get(&self) -> AppConfig {
        self.config.read().expect("Config lock poisoned").clone()
    }

    /// Update configuration
    pub async fn update(&self, new_config: AppConfig) {
        *self.config.write().expect("Config lock poisoned") = new_config;
    }
}
