use std::fmt::{Display, Formatter, Result as FmtResult};
use std::str::FromStr;

/// Storage backend types
///
/// This enum defines the available storage backend types.
/// It's defined in core because it's used in configuration and database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(
    feature = "sqlx",
    sqlx(type_name = "storage_backend", rename_all = "lowercase")
)]
#[serde(rename_all = "lowercase")]
pub enum StorageBackend {
    S3,
    Local,
    Nfs,
}

impl FromStr for StorageBackend {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "s3" => Ok(StorageBackend::S3),
            "local" => Ok(StorageBackend::Local),
            "nfs" => Ok(StorageBackend::Nfs),
            _ => Err(anyhow::anyhow!("Invalid storage backend: {}", s)),
        }
    }
}

impl Display for StorageBackend {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            StorageBackend::S3 => write!(f, "s3"),
            StorageBackend::Local => write!(f, "local"),
            StorageBackend::Nfs => write!(f, "nfs"),
        }
    }
}
