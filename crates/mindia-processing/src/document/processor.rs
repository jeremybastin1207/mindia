//! Document processor - metadata extraction and validation

use crate::metadata::DocumentMetadata;
use crate::traits::MediaProcessor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;

pub struct DocumentProcessor;

#[async_trait]
impl MediaProcessor for DocumentProcessor {
    type Metadata = DocumentMetadata;

    async fn extract_metadata(&self, data: &[u8]) -> Result<Self::Metadata, anyhow::Error> {
        // Determine format from magic bytes
        let format = if data.len() >= 4 && &data[0..4] == b"%PDF" {
            "pdf"
        } else if data.len() >= 2 && data[0..2] == [0x50, 0x4B] {
            // ZIP-based formats (DOCX, XLSX, PPTX)
            if data.len() >= 30 {
                // Check for Office Open XML signatures
                if data.starts_with(b"PK\x03\x04") {
                    "office" // Generic office format
                } else {
                    "zip"
                }
            } else {
                "unknown"
            }
        } else if data.len() >= 2 && data[0..2] == [0xD0, 0xCF] {
            "office_legacy" // Older MS Office formats
        } else {
            "unknown"
        };

        // Extract basic metadata
        let (page_count, title, author) = if format == "pdf" {
            self.extract_pdf_metadata(data).await?
        } else {
            (None, None, None)
        };

        Ok(DocumentMetadata {
            page_count,
            format: format.to_string(),
            title,
            author,
            size_bytes: Some(data.len() as u64),
        })
    }

    fn validate(&self, data: &[u8]) -> Result<(), anyhow::Error> {
        if data.is_empty() {
            return Err(anyhow!("Document data is empty"));
        }

        // Basic format validation
        if data.len() >= 4 && &data[0..4] == b"%PDF" {
            // PDF validation - check for PDF version header
            Ok(())
        } else if data.len() >= 2 && data[0..2] == [0x50, 0x4B] {
            // ZIP-based format validation
            Ok(())
        } else if data.len() >= 2 && data[0..2] == [0xD0, 0xCF] {
            // Legacy Office format
            Ok(())
        } else {
            // Unknown format - allow it but log warning
            tracing::warn!("Unknown document format detected");
            Ok(())
        }
    }

    fn get_dimensions(&self, _data: &[u8]) -> Option<(u32, u32)> {
        // Documents don't have pixel dimensions (pages have sizes but not easily accessible)
        None
    }
}

impl DocumentProcessor {
    async fn extract_pdf_metadata(
        &self,
        data: &[u8],
    ) -> Result<(Option<u32>, Option<String>, Option<String>), anyhow::Error> {
        // Basic PDF metadata extraction without external dependencies
        // For production, you'd want to use lopdf or pdf-extract

        // Try to extract page count from /Count in /Pages object
        let data_str = String::from_utf8_lossy(data);
        let page_count = data_str.split("/Count").nth(1).and_then(|s| {
            let num_str = s
                .chars()
                .skip_while(|c| !c.is_ascii_digit())
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>();
            num_str.parse::<u32>().ok()
        });

        // Extract title from /Title in document info
        let title = data_str.split("/Title").nth(1).and_then(|s| {
            let mut chars = s.chars().skip_while(|c| *c != '(' && *c != '<');
            if chars.next() == Some('(') {
                // Extract string between parentheses
                let title_str: String = chars.take_while(|c| *c != ')').collect();
                Some(title_str).filter(|t| !t.is_empty())
            } else {
                None
            }
        });

        // Extract author from /Author in document info
        let author = data_str.split("/Author").nth(1).and_then(|s| {
            let mut chars = s.chars().skip_while(|c| *c != '(' && *c != '<');
            if chars.next() == Some('(') {
                let author_str: String = chars.take_while(|c| *c != ')').collect();
                Some(author_str).filter(|a| !a.is_empty())
            } else {
                None
            }
        });

        Ok((page_count, title, author))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_processor() -> DocumentProcessor {
        DocumentProcessor
    }

    #[test]
    fn test_validate_pdf() {
        let processor = create_test_processor();
        // PDF file signature: "%PDF"
        let pdf_data = b"%PDF-1.4\n";
        assert!(processor.validate(pdf_data).is_ok());
    }

    #[test]
    fn test_validate_zip_based() {
        let processor = create_test_processor();
        // ZIP file signature: "PK" (0x50, 0x4B)
        let zip_data = b"PK\x03\x04";
        assert!(processor.validate(zip_data).is_ok());
    }

    #[test]
    fn test_validate_legacy_office() {
        let processor = create_test_processor();
        // Legacy Office format: 0xD0, 0xCF
        let office_data = &[0xD0, 0xCF, 0x11, 0xE0];
        assert!(processor.validate(office_data).is_ok());
    }

    #[test]
    fn test_validate_empty() {
        let processor = create_test_processor();
        let empty_data = b"";
        assert!(processor.validate(empty_data).is_err());
    }

    #[test]
    fn test_validate_unknown_format() {
        let processor = create_test_processor();
        // Unknown format - should still pass with warning
        let unknown_data = b"UNKNOWN_FORMAT";
        // Currently accepts unknown formats with a warning
        assert!(processor.validate(unknown_data).is_ok());
    }

    #[test]
    fn test_get_dimensions() {
        let processor = create_test_processor();
        let pdf_data = b"%PDF-1.4\n";
        // Documents don't have pixel dimensions
        assert_eq!(processor.get_dimensions(pdf_data), None);
    }

    #[tokio::test]
    async fn test_extract_metadata_pdf() {
        let processor = create_test_processor();
        // Minimal PDF with PDF signature
        let pdf_data = b"%PDF-1.4\n";
        let metadata = processor.extract_metadata(pdf_data).await.unwrap();

        assert_eq!(metadata.format, "pdf");
        assert_eq!(metadata.size_bytes, Some(pdf_data.len() as u64));
    }

    #[tokio::test]
    async fn test_extract_metadata_zip_based() {
        let processor = create_test_processor();
        // Create a proper ZIP header with enough bytes (30+) to be recognized as office format
        let mut zip_data = vec![0x50, 0x4B, 0x03, 0x04]; // PK\x03\x04
        zip_data.extend_from_slice(&[0; 26]); // Pad to 30 bytes minimum
        let metadata = processor.extract_metadata(&zip_data).await.unwrap();

        assert_eq!(metadata.format, "office");
        assert_eq!(metadata.size_bytes, Some(zip_data.len() as u64));
    }

    #[tokio::test]
    async fn test_extract_metadata_legacy_office() {
        let processor = create_test_processor();
        let office_data = &[0xD0, 0xCF, 0x11, 0xE0];
        let metadata = processor.extract_metadata(office_data).await.unwrap();

        assert_eq!(metadata.format, "office_legacy");
        assert_eq!(metadata.size_bytes, Some(office_data.len() as u64));
    }

    #[tokio::test]
    async fn test_extract_metadata_unknown() {
        let processor = create_test_processor();
        let unknown_data = b"UNKNOWN";
        let metadata = processor.extract_metadata(unknown_data).await.unwrap();

        assert_eq!(metadata.format, "unknown");
        assert_eq!(metadata.size_bytes, Some(unknown_data.len() as u64));
    }

    #[tokio::test]
    async fn test_extract_pdf_metadata_with_count() {
        let processor = create_test_processor();
        // PDF with /Count in /Pages
        let pdf_data = b"%PDF-1.4\n/Pages /Count 5\n";
        let (page_count, _, _) = processor.extract_pdf_metadata(pdf_data).await.unwrap();
        assert_eq!(page_count, Some(5));
    }

    #[tokio::test]
    async fn test_extract_pdf_metadata_with_title() {
        let processor = create_test_processor();
        // PDF with /Title
        let pdf_data = b"%PDF-1.4\n/Title (Test Document)\n";
        let (_, title, _) = processor.extract_pdf_metadata(pdf_data).await.unwrap();
        assert_eq!(title, Some("Test Document".to_string()));
    }

    #[tokio::test]
    async fn test_extract_pdf_metadata_with_author() {
        let processor = create_test_processor();
        // PDF with /Author
        let pdf_data = b"%PDF-1.4\n/Author (John Doe)\n";
        let (_, _, author) = processor.extract_pdf_metadata(pdf_data).await.unwrap();
        assert_eq!(author, Some("John Doe".to_string()));
    }

    #[tokio::test]
    async fn test_extract_pdf_metadata_no_metadata() {
        let processor = create_test_processor();
        // PDF without metadata
        let pdf_data = b"%PDF-1.4\n";
        let (page_count, title, author) = processor.extract_pdf_metadata(pdf_data).await.unwrap();
        assert_eq!(page_count, None);
        assert_eq!(title, None);
        assert_eq!(author, None);
    }
}
