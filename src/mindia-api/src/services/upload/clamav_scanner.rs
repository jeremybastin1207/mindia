//! VirusScanner implementation using ClamAV.

use async_trait::async_trait;
use std::sync::Arc;

use mindia_processing::VirusScanner;
use mindia_services::{ClamAVService, ScanResult};

/// ClamAV-backed VirusScanner for the upload pipeline.
#[allow(dead_code)]
pub struct ClamAVVirusScanner {
    service: Arc<ClamAVService>,
}

impl ClamAVVirusScanner {
    #[allow(dead_code)]
    pub fn new(service: Arc<ClamAVService>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl VirusScanner for ClamAVVirusScanner {
    async fn scan(&self, data: &[u8]) -> anyhow::Result<()> {
        match self.service.scan_bytes(data).await {
            ScanResult::Clean => Ok(()),
            ScanResult::Infected(name) => {
                anyhow::bail!("File rejected: virus detected ({})", name)
            }
            ScanResult::Error(e) => anyhow::bail!("Virus scan failed: {}", e),
        }
    }
}
