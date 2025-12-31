use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use sysinfo::{Disks, System};
use tokio::task::JoinHandle;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use tracing::{error, warn};

use mindia_core::{AppError, Config};

#[derive(Clone)]
pub struct CapacityChecker {
    config: Config,
    system: Arc<std::sync::Mutex<System>>,
}

impl CapacityChecker {
    pub fn new(config: Config) -> Self {
        let mut system = System::new();
        system.refresh_all();

        Self {
            config,
            system: Arc::new(std::sync::Mutex::new(system)),
        }
    }

    /// Check if there's enough disk space at the given path
    pub fn check_disk_space(&self, path: &Path, required_bytes: u64) -> Result<()> {
        let min_free_bytes = self.config.min_disk_free_gb() * 1024 * 1024 * 1024;
        let total_required = required_bytes + min_free_bytes;

        // Get the mount point for the path
        let mount_path = self.get_mount_path(path)?;

        // Refresh disk info
        let disks = Disks::new_with_refreshed_list();

        // Find the disk for this path
        let available_bytes = disks
            .iter()
            .find(|disk| {
                let disk_path = PathBuf::from(disk.mount_point());
                mount_path.starts_with(&disk_path) || disk_path.starts_with(&mount_path)
            })
            .map(|disk| disk.available_space())
            .ok_or_else(|| {
                anyhow!(
                    "Could not determine disk space for path: {}",
                    path.display()
                )
            })?;

        if available_bytes < total_required {
            let behavior = &self.config.disk_check_behavior();
            let error = AppError::InsufficientDiskSpace {
                available: available_bytes,
                required: total_required,
            };

            match behavior.as_str() {
                "fail" => {
                    error!(
                        available_bytes = available_bytes,
                        required_bytes = total_required,
                        path = %path.display(),
                        "Insufficient disk space"
                    );
                    return Err(error.into());
                }
                "warn" => {
                    warn!(
                        available_bytes = available_bytes,
                        required_bytes = total_required,
                        path = %path.display(),
                        "Insufficient disk space (warning only)"
                    );
                }
                _ => {
                    warn!(
                        behavior = %behavior,
                        "Unknown disk_check_behavior, defaulting to warn"
                    );
                }
            }
        }

        Ok(())
    }

    /// Check if there's enough disk space (async; runs sync check in spawn_blocking to avoid blocking the runtime).
    pub async fn check_disk_space_async(&self, path: &Path, required_bytes: u64) -> Result<()> {
        let path = path.to_path_buf();
        let checker = self.clone();
        tokio::task::spawn_blocking(move || checker.check_disk_space(&path, required_bytes))
            .await
            .map_err(|e| anyhow::anyhow!("spawn_blocking for disk space check: {}", e))?
    }

    /// Check if there's enough memory available
    pub fn check_memory(&self, required_bytes: u64) -> Result<()> {
        let mut system = self.system.lock().map_err(|e| {
            tracing::error!(error = %e, "Failed to acquire system lock for memory check");
            anyhow::anyhow!("Failed to check memory: mutex poisoned")
        })?;
        system.refresh_memory();

        let total_memory = system.total_memory();
        let used_memory = system.used_memory();
        let available_memory = total_memory.saturating_sub(used_memory);
        let memory_usage_percent = (used_memory as f64 / total_memory as f64) * 100.0;

        // Check if we have enough free memory
        if available_memory < required_bytes {
            let behavior = &self.config.memory_check_behavior();
            let error = AppError::InsufficientMemory {
                available: available_memory,
                required: required_bytes,
            };

            match behavior.as_str() {
                "fail" => {
                    error!(
                        available_bytes = available_memory,
                        required_bytes = required_bytes,
                        memory_usage_percent = memory_usage_percent,
                        "Insufficient memory"
                    );
                    return Err(error.into());
                }
                "warn" => {
                    warn!(
                        available_bytes = available_memory,
                        required_bytes = required_bytes,
                        memory_usage_percent = memory_usage_percent,
                        "Insufficient memory (warning only)"
                    );
                }
                _ => {
                    warn!(
                        behavior = %behavior,
                        "Unknown memory_check_behavior, defaulting to warn"
                    );
                }
            }
        }

        // Check memory usage percentage threshold
        let max_memory = self.config.max_memory_usage_percent();
        if memory_usage_percent > max_memory {
            let behavior = &self.config.memory_check_behavior();
            let error = AppError::HighMemoryUsage {
                usage_percent: memory_usage_percent,
                threshold: max_memory,
            };

            match behavior.as_str() {
                "fail" => {
                    error!(
                        usage_percent = memory_usage_percent,
                        threshold = max_memory,
                        "Memory usage exceeds threshold"
                    );
                    return Err(error.into());
                }
                "warn" => {
                    warn!(
                        usage_percent = memory_usage_percent,
                        threshold = max_memory,
                        "Memory usage exceeds threshold (warning only)"
                    );
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Check if there's enough memory (async; runs sync check in spawn_blocking to avoid blocking the runtime).
    pub async fn check_memory_async(&self, required_bytes: u64) -> Result<()> {
        let checker = self.clone();
        tokio::task::spawn_blocking(move || checker.check_memory(required_bytes))
            .await
            .map_err(|e| anyhow::anyhow!("spawn_blocking for memory check: {}", e))?
    }

    /// Check CPU usage
    pub fn check_cpu_usage(&self) -> Result<()> {
        let mut system = self.system.lock().map_err(|e| {
            tracing::error!(error = %e, "Failed to acquire system lock for CPU check");
            anyhow::anyhow!("Failed to check CPU usage: mutex poisoned")
        })?;
        system.refresh_cpu();

        // Get average CPU usage across all cores
        let cpus = system.cpus();
        let cpu_usage = if cpus.is_empty() {
            0.0
        } else {
            cpus.iter().map(|cpu| cpu.cpu_usage()).sum::<f32>() / cpus.len() as f32
        };

        let max_cpu = self.config.max_cpu_usage_percent();
        if cpu_usage > max_cpu as f32 {
            let behavior = &self.config.cpu_check_behavior();
            let error = AppError::HighCpuUsage {
                usage_percent: cpu_usage as f64,
                threshold: max_cpu,
            };

            match behavior.as_str() {
                "fail" => {
                    error!(
                        usage_percent = cpu_usage,
                        threshold = max_cpu,
                        "CPU usage exceeds threshold"
                    );
                    return Err(error.into());
                }
                "warn" => {
                    warn!(
                        usage_percent = cpu_usage,
                        threshold = max_cpu,
                        "CPU usage exceeds threshold (warning only)"
                    );
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Check CPU usage (async; runs sync check in spawn_blocking to avoid blocking the runtime).
    pub async fn check_cpu_usage_async(&self) -> Result<()> {
        let checker = self.clone();
        tokio::task::spawn_blocking(move || checker.check_cpu_usage())
            .await
            .map_err(|e| anyhow::anyhow!("spawn_blocking for CPU check: {}", e))?
    }

    /// Estimate space needed for video transcoding
    pub fn estimate_video_transcode_space(&self, input_size: u64) -> u64 {
        (input_size as f64 * self.config.video_transcode_space_multiplier()) as u64
    }

    /// Estimate space needed for temp file (with safety margin)
    pub fn estimate_temp_file_space(&self, file_size: u64) -> u64 {
        // Add 10% safety margin
        (file_size as f64 * 1.1) as u64
    }

    /// Monitor resources during a long-running operation
    pub async fn monitor_during_operation<F, T>(
        &self,
        operation: F,
        check_interval: Duration,
    ) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>> + Send,
    {
        if !self.config.capacity_monitor_enabled() {
            return operation.await;
        }

        let cancel_token = CancellationToken::new();
        let monitor_handle = self.create_monitor_task(cancel_token.clone(), check_interval);

        // Run the operation
        let result = operation.await;

        // Cancel monitoring
        cancel_token.cancel();
        monitor_handle.await.ok();

        result
    }

    /// Create a background monitoring task that checks resources periodically
    pub fn create_monitor_task(
        &self,
        cancel_token: CancellationToken,
        check_interval: Duration,
    ) -> JoinHandle<()> {
        let checker = self.clone();
        let mut interval = interval(check_interval);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = cancel_token.cancelled() => {
                        break;
                    }
                    _ = interval.tick() => {
                        // Check disk space (using temp dir as reference) in spawn_blocking
                        if let Ok(temp_dir) = std::env::temp_dir().canonicalize() {
                            if let Err(e) = checker.check_disk_space_async(&temp_dir, 0).await {
                                if let Some(AppError::InsufficientDiskSpace { .. }) =
                                    e.downcast_ref::<AppError>()
                                {
                                    if checker.config.disk_check_behavior() == "fail" {
                                        error!(
                                            "Disk space critical during operation, canceling"
                                        );
                                        cancel_token.cancel();
                                        break;
                                    }
                                }
                            }
                        }

                        // Check memory in spawn_blocking
                        if let Err(e) = checker.check_memory_async(0).await {
                            if let Some(
                                AppError::InsufficientMemory { .. }
                                | AppError::HighMemoryUsage { .. },
                            ) = e.downcast_ref::<AppError>()
                            {
                                if checker.config.memory_check_behavior() == "fail" {
                                    error!("Memory usage critical during operation, canceling");
                                    cancel_token.cancel();
                                    break;
                                }
                            }
                        }

                        // Check CPU in spawn_blocking
                        if let Err(e) = checker.check_cpu_usage_async().await {
                            if let Some(AppError::HighCpuUsage { .. }) =
                                e.downcast_ref::<AppError>()
                            {
                                if checker.config.cpu_check_behavior() == "fail" {
                                    error!("CPU usage critical during operation, canceling");
                                    cancel_token.cancel();
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        })
    }

    /// Get the mount path for a given path
    fn get_mount_path(&self, path: &Path) -> Result<PathBuf> {
        // Try to canonicalize the path
        let canonical = path.canonicalize().context("Failed to canonicalize path")?;

        // For now, return the canonical path
        // In a more sophisticated implementation, we could traverse up to find the mount point
        Ok(canonical)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mindia_core::{Config, MediaProcessorConfig};

    #[test]
    fn test_estimate_video_transcode_space() {
        // Create minimal test config
        std::env::set_var("DATABASE_URL", "postgresql://test:test@localhost/test");
        std::env::set_var("JWT_SECRET", "test_secret_key_32_characters_long_minimum");
        std::env::set_var("STORAGE_BACKEND", "local");
        std::env::set_var("LOCAL_STORAGE_PATH", "/tmp/mindia-test");
        std::env::set_var("LOCAL_STORAGE_BASE_URL", "http://localhost:3000");

        let config = Config(Box::new(
            MediaProcessorConfig::from_env().expect("Failed to create test config"),
        ));
        let checker = CapacityChecker::new(config.clone());

        let input_size = 1_000_000; // 1 MB
        let multiplier = config.video_transcode_space_multiplier();
        let estimated = checker.estimate_video_transcode_space(input_size);

        let expected = (input_size as f64 * multiplier) as u64;
        assert_eq!(estimated, expected);
    }

    #[test]
    fn test_estimate_temp_file_space() {
        std::env::set_var("DATABASE_URL", "postgresql://test:test@localhost/test");
        std::env::set_var("JWT_SECRET", "test_secret_key_32_characters_long_minimum");
        std::env::set_var("STORAGE_BACKEND", "local");
        std::env::set_var("LOCAL_STORAGE_PATH", "/tmp/mindia-test");
        std::env::set_var("LOCAL_STORAGE_BASE_URL", "http://localhost:3000");

        let config = Config(Box::new(
            MediaProcessorConfig::from_env().expect("Failed to create test config"),
        ));
        let checker = CapacityChecker::new(config);

        let file_size = 1_000_000; // 1 MB
        let estimated = checker.estimate_temp_file_space(file_size);

        let expected = (file_size as f64 * 1.1) as u64;
        assert_eq!(estimated, expected);
    }
}
