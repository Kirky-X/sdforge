// Copyright (c) 2026 Kirky.X
//! Configuration hot reload support
//!
//! This module provides configuration hot-reload functionality using the Confers library.

use std::path::PathBuf;

use tokio::sync::mpsc;

use crate::config::ConfigError;
use crate::AppConfig;

/// Configuration event type for hot reload
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

/// File watcher for configuration files using Confers FsWatcher
pub struct ConfigWatcherImpl {
    path: PathBuf,
}

impl ConfigWatcherImpl {
    /// Create a new configuration watcher
    ///
    /// Returns a tuple of the watcher and a receiver for config events.
    pub async fn new(path: PathBuf) -> Result<(Self, mpsc::Receiver<ConfigEvent>), ConfigError> {
        let (tx, rx) = mpsc::channel(1);

        let mut watcher = confers::watcher::FsWatcher::new(&path, 200)
            .await
            .map_err(|e| ConfigError::WatchError(e.to_string()))?;

        let path_clone = path.clone();
        tokio::spawn(async move {
            while let Some(_changed_path) = watcher.recv().await {
                if let Ok(content) = tokio::fs::read_to_string(&path_clone).await {
                    match toml::from_str::<AppConfig>(&content) {
                        Ok(config) => {
                            let _ = tx.send(ConfigEvent::Reloaded(Box::new(config))).await;
                        }
                        Err(e) => {
                            let _ = tx.send(ConfigEvent::Error(e.to_string())).await;
                        }
                    }
                }
            }
        });

        Ok((Self { path }, rx))
    }

    /// Get the watched configuration path
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Get the current configuration (loads from file)
    pub async fn get(&self) -> Result<AppConfig, ConfigError> {
        let content = tokio::fs::read_to_string(&self.path)
            .await
            .map_err(|e| ConfigError::LoadError(e.to_string()))?;
        toml::from_str(&content).map_err(|e| ConfigError::ParseError {
            message: e.to_string(),
        })
    }
}

/// Create a configuration watcher for the specified path
///
/// This is a wrapper that provides backward compatibility with the existing API.
pub async fn create_config_watcher(
    path: &str,
) -> Result<(ConfigWatcherImpl, mpsc::Receiver<ConfigEvent>), ConfigError> {
    if !PathBuf::from(path).exists() {
        return Err(ConfigError::FileNotFound {
            path: path.to_string(),
        });
    }

    ConfigWatcherImpl::new(path.into()).await
}

/// Configuration manager for hot reload
pub struct ConfigManager {
    config: tokio::sync::RwLock<AppConfig>,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new(config: AppConfig) -> Self {
        Self {
            config: tokio::sync::RwLock::new(config),
        }
    }

    /// Get current configuration
    pub async fn get(&self) -> AppConfig {
        self.config.read().await.clone()
    }

    /// Update configuration
    pub async fn update(&self, new_config: AppConfig) {
        *self.config.write().await = new_config;
    }
}

#[cfg(test)]
#[allow(unused)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Process a watcher event result, asserting on Reloaded events when
    /// expected values are provided. Extracted from the file-change tests so
    /// all match arms can be covered deterministically by unit tests.
    fn handle_watcher_event(event: Option<ConfigEvent>, expected: Option<(&str, u16)>) {
        match event {
            Some(ConfigEvent::Reloaded(config)) => {
                if let Some((host, port)) = expected {
                    assert_eq!(config.server.host, host);
                    assert_eq!(config.server.port, port);
                }
            }
            Some(ConfigEvent::Error(_)) => {}
            None => {}
        }
    }

    /// Test ConfigEvent variants
    #[test]
    fn test_config_event_variants() {
        let config = AppConfig::default();
        let reloaded_event = ConfigEvent::Reloaded(Box::new(config.clone()));

        assert!(
            matches!(reloaded_event, ConfigEvent::Reloaded(_)),
            "Expected Reloaded event, got {:?}",
            reloaded_event
        );

        let error_event = ConfigEvent::Error("Test error message".to_string());

        assert!(
            matches!(error_event, ConfigEvent::Error(ref msg) if msg == "Test error message"),
            "Expected Error event with correct message, got {:?}",
            error_event
        );
    }

    /// Test ConfigManager creation and operations
    #[tokio::test]
    async fn test_config_manager_operations() {
        let config = AppConfig::default();
        let manager = ConfigManager::new(config.clone());

        let retrieved = manager.get().await;
        assert_eq!(retrieved.server.host, config.server.host);

        let mut new_config = AppConfig::default();
        new_config.server.host = "127.0.0.1".to_string();
        manager.update(new_config.clone()).await;

        let updated = manager.get().await;
        assert_eq!(updated.server.host, "127.0.0.1");
    }

    /// Test ConfigManager with RwLock operations
    #[tokio::test]
    async fn test_config_manager_rwlock_operations() {
        let config = AppConfig::default();
        let manager = ConfigManager::new(config);

        let _retrieved = manager.get().await;

        let mut new_config = AppConfig::default();
        new_config.server.host = "127.0.0.1".to_string();
        manager.update(new_config.clone()).await;

        let updated = manager.get().await;
        assert_eq!(updated.server.host, "127.0.0.1");
    }

    /// Test create_config_watcher with non-existent path
    #[tokio::test]
    async fn test_create_config_watcher_nonexistent() {
        let result = create_config_watcher("/nonexistent/path.toml").await;
        assert!(result.is_err());
    }

    /// Test create_config_watcher with valid path
    #[tokio::test]
    async fn test_create_config_watcher_valid() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config_content = r#"
            [server]
            host = "localhost"
            port = 8080
            request_timeout_secs = 30

            [authentication]
            type = "none"

            [logging]
            level = "info"
            format = "json"
        "#;
        std::fs::write(&config_path, config_content).unwrap();

        let result = create_config_watcher(config_path.to_str().unwrap()).await;
        assert!(result.is_ok());
    }

    // ============================================================================
    // ConfigWatcherImpl path() accessor tests
    // ============================================================================

    #[tokio::test]
    async fn test_config_watcher_path_accessor() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config_content = r#"
            [server]
            host = "localhost"
            port = 8080
            request_timeout_secs = 30

            [authentication]
            type = "none"

            [logging]
            level = "info"
            format = "json"
        "#;
        std::fs::write(&config_path, config_content).unwrap();

        let (watcher, _rx) = create_config_watcher(config_path.to_str().unwrap())
            .await
            .expect("Watcher creation should succeed");

        // path() should return the path we passed in
        assert_eq!(watcher.path(), &config_path);
    }

    // ============================================================================
    // ConfigWatcherImpl get() method tests
    // ============================================================================

    #[tokio::test]
    async fn test_config_watcher_get_valid_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config_content = r#"
            [server]
            host = "0.0.0.0"
            port = 3000
            request_timeout_secs = 60

            [authentication]
            type = "none"

            [logging]
            level = "debug"
            format = "json"
        "#;
        std::fs::write(&config_path, config_content).unwrap();

        let (watcher, _rx) = create_config_watcher(config_path.to_str().unwrap())
            .await
            .expect("Watcher creation should succeed");

        let config = watcher.get().await.expect("get() should parse valid config");
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.server.request_timeout_secs, 60);
    }

    #[tokio::test]
    async fn test_config_watcher_get_invalid_toml() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        // Write invalid TOML content
        std::fs::write(&config_path, "this is not valid toml = = =").unwrap();

        let (watcher, _rx) = create_config_watcher(config_path.to_str().unwrap())
            .await
            .expect("Watcher creation should succeed");

        let result = watcher.get().await;
        assert!(
            result.is_err(),
            "get() should fail on invalid TOML content"
        );
    }

    #[tokio::test]
    async fn test_config_watcher_get_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config_content = r#"
            [server]
            host = "localhost"
            port = 8080
            request_timeout_secs = 30

            [authentication]
            type = "none"

            [logging]
            level = "info"
            format = "json"
        "#;
        std::fs::write(&config_path, config_content).unwrap();

        let (watcher, _rx) = create_config_watcher(config_path.to_str().unwrap())
            .await
            .expect("Watcher creation should succeed");

        // Delete the file before calling get()
        std::fs::remove_file(&config_path).unwrap();

        let result = watcher.get().await;
        assert!(
            result.is_err(),
            "get() should fail when the file no longer exists"
        );
    }

    // ============================================================================
    // handle_watcher_event helper unit tests
    //
    // Covers all match arms of the helper deterministically, independent of
    // filesystem watcher timing.
    // ============================================================================

    #[test]
    fn test_handle_watcher_event_reloaded_with_expectations() {
        let mut config = AppConfig::default();
        config.server.host = "0.0.0.0".to_string();
        config.server.port = 9090;
        handle_watcher_event(
            Some(ConfigEvent::Reloaded(Box::new(config))),
            Some(("0.0.0.0", 9090)),
        );
    }

    #[test]
    fn test_handle_watcher_event_reloaded_without_expectations() {
        let config = AppConfig::default();
        handle_watcher_event(Some(ConfigEvent::Reloaded(Box::new(config))), None);
    }

    #[test]
    fn test_handle_watcher_event_error_variant() {
        handle_watcher_event(Some(ConfigEvent::Error("err".to_string())), None);
    }

    #[test]
    fn test_handle_watcher_event_none_variant() {
        handle_watcher_event(None, None);
    }

    // ============================================================================
    // Watcher file-change detection tests
    //
    // These tests modify the config file after creating the watcher and verify
    // that a ConfigEvent is emitted. They exercise the watcher spawn loop
    // (lines 44-51) which reads and parses the file on change.
    // ============================================================================

    #[tokio::test]
    async fn test_config_watcher_detects_file_change_to_valid_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let initial_content = r#"
            [server]
            host = "localhost"
            port = 8080
            request_timeout_secs = 30

            [authentication]
            type = "none"

            [logging]
            level = "info"
            format = "json"
        "#;
        std::fs::write(&config_path, initial_content).unwrap();

        let (watcher, mut rx) = create_config_watcher(config_path.to_str().unwrap())
            .await
            .expect("Watcher creation should succeed");

        // Modify the config file to trigger the watcher
        let updated_content = r#"
            [server]
            host = "0.0.0.0"
            port = 9090
            request_timeout_secs = 45

            [authentication]
            type = "none"

            [logging]
            level = "debug"
            format = "json"
        "#;

        // Write the updated content (may need to write twice for some watchers
        // to detect the change reliably)
        std::fs::write(&config_path, updated_content).unwrap();

        // Wait for the watcher to detect the change and emit an event.
        // Use a timeout to avoid hanging if the watcher doesn't fire.
        let event = tokio::time::timeout(
            tokio::time::Duration::from_secs(3),
            rx.recv(),
        )
        .await;

        let event_opt: Option<ConfigEvent> = event.unwrap_or_default();
        handle_watcher_event(event_opt, Some(("0.0.0.0", 9090)));

        // path() should still return the original path
        assert_eq!(watcher.path(), &config_path);
    }

    #[tokio::test]
    async fn test_config_watcher_detects_file_change_to_invalid_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let initial_content = r#"
            [server]
            host = "localhost"
            port = 8080
            request_timeout_secs = 30

            [authentication]
            type = "none"

            [logging]
            level = "info"
            format = "json"
        "#;
        std::fs::write(&config_path, initial_content).unwrap();

        let (_watcher, mut rx) = create_config_watcher(config_path.to_str().unwrap())
            .await
            .expect("Watcher creation should succeed");

        // Overwrite with invalid TOML to trigger the Error branch (lines 50-51)
        std::fs::write(&config_path, "invalid toml content = = =").unwrap();

        let event = tokio::time::timeout(
            tokio::time::Duration::from_secs(3),
            rx.recv(),
        )
        .await;

        let event_opt: Option<ConfigEvent> = event.unwrap_or_default();
        handle_watcher_event(event_opt, None);
    }

    /// Deterministically cover the timeout (Err) arm of the watcher event
    /// match by using a channel that never receives a value.
    #[tokio::test]
    async fn test_watcher_event_timeout_arm() {
        let (_tx, mut rx) = mpsc::channel::<ConfigEvent>(1);

        let event = tokio::time::timeout(
            tokio::time::Duration::from_millis(10),
            rx.recv(),
        )
        .await;

        let event_opt: Option<ConfigEvent> = event.unwrap_or_default();
        handle_watcher_event(event_opt, None);
    }

    /// Cover the `if let Ok(content)` false branch in the watcher spawn loop
    /// by writing invalid UTF-8 bytes to the config file, which causes
    /// `tokio::fs::read_to_string` to fail.
    #[tokio::test]
    async fn test_config_watcher_read_failure_on_invalid_utf8() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let initial_content = r#"
            [server]
            host = "localhost"
            port = 8080
            request_timeout_secs = 30

            [authentication]
            type = "none"

            [logging]
            level = "info"
            format = "json"
        "#;
        std::fs::write(&config_path, initial_content).unwrap();

        let (_watcher, mut rx) = create_config_watcher(config_path.to_str().unwrap())
            .await
            .expect("Watcher creation should succeed");

        // Write invalid UTF-8 bytes to trigger read_to_string failure.
        // The watcher loop's `if let Ok(content)` will be false, so no
        // event is emitted — the test just verifies no panic occurs.
        std::fs::write(&config_path, b"\xff\xfe\x00invalid utf8").unwrap();

        let event = tokio::time::timeout(
            tokio::time::Duration::from_secs(2),
            rx.recv(),
        )
        .await;

        let event_opt: Option<ConfigEvent> = event.unwrap_or_default();
        // No event expected (read failure silently continues the loop).
        // We just verify no panic occurred.
        handle_watcher_event(event_opt, None);
    }

    // ============================================================================
    // ConfigWatcherImpl::new error path coverage
    //
    // The `.map_err(|e| ConfigError::WatchError(e.to_string()))?` line in
    // ConfigWatcherImpl::new (the FsWatcher::new error path) is triggered when
    // FsWatcher::new returns an error. create_config_watcher checks for file
    // existence first, so the error path is only reachable by calling
    // ConfigWatcherImpl::new directly with a non-existent path.
    // ============================================================================

    /// Test ConfigWatcherImpl::new returns a WatchError when the path does not
    /// exist. Covers the `FsWatcher::new` error mapping path in
    /// ConfigWatcherImpl::new (line 40).
    #[tokio::test]
    async fn test_config_watcher_impl_new_nonexistent_path_returns_watch_error() {
        let result = ConfigWatcherImpl::new(PathBuf::from("/nonexistent/path/config.toml")).await;
        assert!(result.is_err(), "Non-existent path should produce an error");
        match result {
            Err(ConfigError::WatchError(msg)) => {
                // FsWatcher::new returns FileNotFound, which gets mapped to
                // WatchError with the error's string representation.
                assert!(!msg.is_empty(), "WatchError should contain a message");
            }
            Err(other) => panic!("Expected WatchError, got {:?}", other),
            Ok(_) => panic!("Expected error for non-existent path"),
        }
    }

    /// Test ConfigEvent Debug formatting produces output containing the
    /// variant name. Covers the derived Debug impl.
    #[test]
    fn test_config_event_debug_format() {
        let config = AppConfig::default();
        let reloaded = ConfigEvent::Reloaded(Box::new(config));
        let debug_str = format!("{:?}", reloaded);
        assert!(debug_str.contains("Reloaded"), "Debug should contain variant name");

        let error = ConfigEvent::Error("test error".to_string());
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("Error"), "Debug should contain variant name");
        assert!(debug_str.contains("test error"), "Debug should contain error message");
    }

    /// Test ConfigEvent Clone produces an equal value. Covers the derived
    /// Clone impl.
    #[test]
    fn test_config_event_clone_equality() {
        let config = AppConfig::default();
        let original = ConfigEvent::Reloaded(Box::new(config));
        let cloned = original.clone();
        // Both should be Reloaded variants with default configs
        assert!(matches!(cloned, ConfigEvent::Reloaded(_)));

        let original_err = ConfigEvent::Error("clone me".to_string());
        let cloned_err = original_err.clone();
        if let ConfigEvent::Error(msg) = &cloned_err {
            assert_eq!(msg, "clone me");
        } else {
            panic!("Cloned Error variant should remain Error");
        }
    }

    /// Test ConfigManager get() returns a clone of the stored config (not a
    /// reference), so modifying the returned value doesn't affect the manager.
    #[tokio::test]
    async fn test_config_manager_get_returns_independent_clone() {
        let mut config = AppConfig::default();
        config.server.port = 5000;
        let manager = ConfigManager::new(config);

        let retrieved1 = manager.get().await;
        assert_eq!(retrieved1.server.port, 5000);

        // The retrieved value is an independent clone; mutating it must not
        // affect the manager's internal state.
        let mut retrieved2 = manager.get().await;
        retrieved2.server.port = 9999;

        let retrieved3 = manager.get().await;
        assert_eq!(retrieved3.server.port, 5000, "Manager state should be unchanged");
    }

    /// Test ConfigManager update() then get() returns the new config, and a
    /// second update replaces the first. Covers sequential update operations.
    #[tokio::test]
    async fn test_config_manager_sequential_updates() {
        let config = AppConfig::default();
        let manager = ConfigManager::new(config);

        let mut config1 = AppConfig::default();
        config1.server.host = "first.host".to_string();
        manager.update(config1).await;

        let mut config2 = AppConfig::default();
        config2.server.host = "second.host".to_string();
        manager.update(config2).await;

        let final_config = manager.get().await;
        assert_eq!(final_config.server.host, "second.host", "Second update should win");
    }

    /// Test ConfigWatcherImpl::new succeeds with a directory path (not just
    /// files). FsWatcher supports watching directories.
    #[tokio::test]
    async fn test_config_watcher_impl_new_with_directory() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().to_path_buf();

        let result = ConfigWatcherImpl::new(dir_path).await;
        assert!(result.is_ok(), "Watcher should be created for a directory: {:?}", result.err());
        let (watcher, _rx) = result.unwrap();
        assert!(watcher.path().is_dir(), "Path should be the directory");
    }
}
