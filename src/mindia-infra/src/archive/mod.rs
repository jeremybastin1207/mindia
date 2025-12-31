//! Archive creation service
//!
//! This module provides functionality to create archives (ZIP, TAR) from media items.

pub use service::{create_archive, ArchiveFormat};

mod service;
