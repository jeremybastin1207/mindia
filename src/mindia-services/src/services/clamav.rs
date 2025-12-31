use clamav_client::{clean, Tcp};
use std::str;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct ClamAVService {
    host: String,
    port: u16,
    fail_closed: bool,
}

#[derive(Debug)]
pub enum ScanResult {
    Clean,
    Infected(String),
    Error(String),
}

impl ClamAVService {
    pub fn new(host: String, port: u16, fail_closed: bool) -> Self {
        Self {
            host,
            port,
            fail_closed,
        }
    }

    /// Scan in-memory data using sync API inside spawn_blocking to avoid !Send tokio futures.
    pub async fn scan_bytes(&self, data: &[u8]) -> ScanResult {
        let start = Instant::now();
        tracing::debug!(host = %self.host, port = %self.port, "Starting ClamAV scan");
        let data = data.to_vec();
        let host = self.host.clone();
        let port = self.port;
        let fail_closed = self.fail_closed;

        let result = tokio::time::timeout(
            Duration::from_secs(30),
            tokio::task::spawn_blocking(move || {
                let address = format!("{}:{}", host, port);
                let connection = Tcp {
                    host_address: address.as_str(),
                };
                match clamav_client::scan_buffer(data.as_slice(), connection, None) {
                    Ok(response_bytes) => match clean(&response_bytes) {
                        Ok(is_clean) => {
                            if is_clean {
                                tracing::info!(
                                    duration_ms = start.elapsed().as_millis(),
                                    "File scan completed: clean"
                                );
                                ScanResult::Clean
                            } else {
                                let response_str = match str::from_utf8(&response_bytes) {
                                    Ok(s) => s.trim(),
                                    Err(_) => "unknown",
                                };
                                let virus_name = if response_str.contains("FOUND") {
                                    response_str
                                        .split(':')
                                        .nth(1)
                                        .unwrap_or("unknown")
                                        .split_whitespace()
                                        .next()
                                        .unwrap_or("unknown")
                                        .to_string()
                                } else {
                                    "unknown".to_string()
                                };
                                tracing::warn!(
                                    duration_ms = start.elapsed().as_millis(),
                                    virus = %virus_name,
                                    "File scan detected virus"
                                );
                                ScanResult::Infected(virus_name)
                            }
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to parse ClamAV response: {}", e);
                            tracing::error!(error = %error_msg, "Failed to parse ClamAV response");
                            if fail_closed {
                                ScanResult::Error(error_msg)
                            } else {
                                tracing::warn!(
                                    "ClamAV response parsing failed, continuing (fail-open)"
                                );
                                ScanResult::Clean
                            }
                        }
                    },
                    Err(e) => {
                        let error_msg = format!("ClamAV scan error: {}", e);
                        tracing::error!(error = %error_msg, "ClamAV scan failed");
                        if fail_closed {
                            ScanResult::Error(error_msg)
                        } else {
                            tracing::warn!("ClamAV scan failed, continuing (fail-open)");
                            ScanResult::Clean
                        }
                    }
                }
            }),
        )
        .await;

        match result {
            Ok(Ok(sr)) => sr,
            Ok(Err(e)) => {
                let error_msg = format!("ClamAV scan task join error: {}", e);
                tracing::error!(error = %error_msg, "ClamAV scan panicked");
                ScanResult::Error(error_msg)
            }
            Err(_) => {
                let error_msg = "ClamAV scan timeout (exceeded 30 seconds)";
                tracing::error!(error = %error_msg, "ClamAV scan timeout");
                if fail_closed {
                    ScanResult::Error(error_msg.to_string())
                } else {
                    tracing::warn!("ClamAV scan timeout, continuing (fail-open)");
                    ScanResult::Clean
                }
            }
        }
    }
}
