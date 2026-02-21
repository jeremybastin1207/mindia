use anyhow::{Context, Result};
use mindia_storage::Storage;
use std::io::Write;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

/// Archive format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFormat {
    Zip,
    Tar,
}

/// Sanitize filename for archive entry to prevent path traversal.
/// Extracts only the base name (strips path components like `../`).
fn sanitize_archive_filename(filename: &str, fallback: &str) -> String {
    Path::new(filename)
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|s| !s.is_empty() && *s != "." && *s != "..")
        .unwrap_or(fallback)
        .to_string()
}

impl FromStr for ArchiveFormat {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "zip" => Ok(ArchiveFormat::Zip),
            "tar" => Ok(ArchiveFormat::Tar),
            _ => Err(anyhow::anyhow!("Unsupported archive format: {}", s)),
        }
    }
}

/// Create a ZIP archive from media items
pub async fn create_zip_archive(
    storage: Arc<dyn Storage>,
    media_items: Vec<(uuid::Uuid, String, String)>, // (media_id, storage_key, original_filename)
) -> Result<Vec<u8>> {
    use zip::write::{FileOptions, ZipWriter};
    use zip::CompressionMethod;

    let mut buffer = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buffer));
        let options = FileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(0o644);

        for (media_id, storage_key, original_filename) in media_items {
            // Download file from storage
            let file_data = storage
                .download(&storage_key)
                .await
                .with_context(|| format!("Failed to download file: {}", storage_key))?;

            // Sanitize filename to prevent path traversal
            let safe_filename =
                sanitize_archive_filename(&original_filename, &format!("unnamed_{}", media_id));

            // Add file to archive
            zip.start_file(&safe_filename, options)
                .with_context(|| format!("Failed to add file to ZIP: {}", safe_filename))?;
            zip.write_all(&file_data)
                .with_context(|| format!("Failed to write file data to ZIP: {}", safe_filename))?;
        }

        zip.finish().context("Failed to finalize ZIP archive")?;
    }

    Ok(buffer)
}

/// Create a TAR archive from media items
pub async fn create_tar_archive(
    storage: Arc<dyn Storage>,
    media_items: Vec<(uuid::Uuid, String, String)>, // (media_id, storage_key, original_filename)
) -> Result<Vec<u8>> {
    use tar::Builder;

    let mut buffer = Vec::new();
    {
        let mut tar = Builder::new(&mut buffer);

        for (media_id, storage_key, original_filename) in media_items {
            // Download file from storage
            let file_data = storage
                .download(&storage_key)
                .await
                .with_context(|| format!("Failed to download file: {}", storage_key))?;

            // Sanitize filename to prevent path traversal
            let safe_filename =
                sanitize_archive_filename(&original_filename, &format!("unnamed_{}", media_id));

            // Create header
            let mut header = tar::Header::new_gnu();
            header.set_size(file_data.len() as u64);
            header.set_mode(0o644); // Set file permissions (rw-r--r--)
            header.set_cksum();

            // Add file to archive
            tar.append_data(&mut header, &safe_filename, &*file_data)
                .with_context(|| format!("Failed to add file to TAR: {}", safe_filename))?;
        }

        tar.finish().context("Failed to finalize TAR archive")?;
    }

    Ok(buffer)
}

/// Create an archive in the specified format
pub async fn create_archive(
    format: ArchiveFormat,
    storage: Arc<dyn Storage>,
    media_items: Vec<(uuid::Uuid, String, String)>,
) -> Result<Vec<u8>> {
    match format {
        ArchiveFormat::Zip => create_zip_archive(storage, media_items).await,
        ArchiveFormat::Tar => create_tar_archive(storage, media_items).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_archive_format_from_str() {
        assert_eq!("zip".parse::<ArchiveFormat>().unwrap(), ArchiveFormat::Zip);
        assert_eq!("ZIP".parse::<ArchiveFormat>().unwrap(), ArchiveFormat::Zip);
        assert_eq!("tar".parse::<ArchiveFormat>().unwrap(), ArchiveFormat::Tar);
        assert_eq!("TAR".parse::<ArchiveFormat>().unwrap(), ArchiveFormat::Tar);

        assert!("invalid".parse::<ArchiveFormat>().is_err());
    }

    #[test]
    fn test_archive_format_partial_eq() {
        assert_eq!(ArchiveFormat::Zip, ArchiveFormat::Zip);
        assert_ne!(ArchiveFormat::Zip, ArchiveFormat::Tar);
    }

    #[test]
    fn test_sanitize_archive_filename() {
        // Path traversal attempts should be stripped to base name
        assert_eq!(
            sanitize_archive_filename("../../etc/passwd", "fallback"),
            "passwd"
        );
        assert_eq!(
            sanitize_archive_filename("../foo/bar.txt", "fallback"),
            "bar.txt"
        );
        // Normal filenames unchanged
        assert_eq!(
            sanitize_archive_filename("document.pdf", "fallback"),
            "document.pdf"
        );
        assert_eq!(
            sanitize_archive_filename("image.png", "fallback"),
            "image.png"
        );
        // Edge cases use fallback
        assert_eq!(sanitize_archive_filename("", "fallback"), "fallback");
        assert_eq!(sanitize_archive_filename("..", "fallback"), "fallback");
        assert_eq!(sanitize_archive_filename(".", "fallback"), "fallback");
    }
}
