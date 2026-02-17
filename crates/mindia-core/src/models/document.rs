use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::storage::StorageLocation;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub filename: String,
    pub original_filename: String,
    pub storage: StorageLocation,
    pub content_type: String,
    pub file_size: i64,
    pub page_count: Option<i32>,
    pub uploaded_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub store_behavior: String,
    pub store_permanently: bool,
    pub expires_at: Option<DateTime<Utc>>,
}

impl Document {
    pub fn storage_key(&self) -> &str {
        &self.storage.key
    }
    pub fn storage_url(&self) -> &str {
        &self.storage.url
    }
    pub fn storage_bucket(&self) -> Option<&str> {
        self.storage.bucket.as_deref()
    }
    /// Backend type (S3, Local, NFS) for this file's storage location.
    pub fn storage_backend(&self) -> crate::StorageBackend {
        self.storage.backend
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DocumentResponse {
    pub id: Uuid,
    pub filename: String,
    pub url: String,
    pub content_type: String,
    pub file_size: i64,
    pub page_count: Option<i32>,
    pub uploaded_at: DateTime<Utc>,
    pub store_behavior: String,
    pub store_permanently: bool,
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_name: Option<String>,
}

impl From<Document> for DocumentResponse {
    fn from(doc: Document) -> Self {
        DocumentResponse {
            id: doc.id,
            filename: doc.original_filename,
            url: doc.storage.url,
            content_type: doc.content_type,
            file_size: doc.file_size,
            page_count: doc.page_count,
            uploaded_at: doc.uploaded_at,
            store_behavior: doc.store_behavior,
            store_permanently: doc.store_permanently,
            expires_at: doc.expires_at,
            folder_id: None,
            folder_name: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage_types::StorageBackend;

    fn test_storage_location(url: &str, key: &str) -> StorageLocation {
        StorageLocation {
            id: Uuid::new_v4(),
            backend: StorageBackend::S3,
            bucket: Some("my-bucket".to_string()),
            key: key.to_string(),
            url: url.to_string(),
        }
    }

    #[test]
    fn test_document_response_from_document() {
        let doc_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();
        let uploaded_at = Utc::now();
        let updated_at = Utc::now();
        let expires_at = Some(Utc::now() + chrono::Duration::days(30));

        let document = Document {
            id: doc_id,
            tenant_id,
            filename: "document_123.pdf".to_string(),
            original_filename: "document.pdf".to_string(),
            storage: test_storage_location(
                "https://s3.amazonaws.com/my-bucket/uploads/document_123.pdf",
                "uploads/document_123.pdf",
            ),
            content_type: "application/pdf".to_string(),
            file_size: 2048000,
            page_count: Some(42),
            uploaded_at,
            updated_at,
            store_behavior: "temporary".to_string(),
            store_permanently: false,
            expires_at,
        };

        let response = DocumentResponse::from(document.clone());

        assert_eq!(response.id, doc_id);
        assert_eq!(response.filename, "document.pdf");
        assert_eq!(
            response.url,
            "https://s3.amazonaws.com/my-bucket/uploads/document_123.pdf"
        );
        assert_eq!(response.content_type, "application/pdf");
        assert_eq!(response.file_size, 2048000);
        assert_eq!(response.page_count, Some(42));
        assert_eq!(response.uploaded_at, uploaded_at);
        assert_eq!(response.store_behavior, "temporary");
        assert!(!response.store_permanently);
        assert_eq!(response.expires_at, expires_at);
    }

    #[test]
    fn test_document_response_from_document_permanent() {
        let doc_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();
        let uploaded_at = Utc::now();
        let updated_at = Utc::now();

        let document = Document {
            id: doc_id,
            tenant_id,
            filename: "permanent_456.pdf".to_string(),
            original_filename: "permanent.pdf".to_string(),
            storage: test_storage_location(
                "https://s3.amazonaws.com/my-bucket/uploads/permanent_456.pdf",
                "uploads/permanent_456.pdf",
            ),
            content_type: "application/pdf".to_string(),
            file_size: 5120000,
            page_count: Some(100),
            uploaded_at,
            updated_at,
            store_behavior: "permanent".to_string(),
            store_permanently: true,
            expires_at: None,
        };

        let response = DocumentResponse::from(document.clone());

        assert_eq!(response.id, doc_id);
        assert_eq!(response.filename, "permanent.pdf");
        assert!(response.store_permanently);
        assert_eq!(response.expires_at, None);
        assert_eq!(response.page_count, Some(100));
    }

    #[test]
    fn test_document_response_from_document_no_page_count() {
        let doc_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();
        let uploaded_at = Utc::now();
        let updated_at = Utc::now();

        let document = Document {
            id: doc_id,
            tenant_id,
            filename: "nopage_789.pdf".to_string(),
            original_filename: "nopage.pdf".to_string(),
            storage: test_storage_location(
                "https://s3.amazonaws.com/my-bucket/uploads/nopage_789.pdf",
                "uploads/nopage_789.pdf",
            ),
            content_type: "application/pdf".to_string(),
            file_size: 1024000,
            page_count: None,
            uploaded_at,
            updated_at,
            store_behavior: "temporary".to_string(),
            store_permanently: false,
            expires_at: None,
        };

        let response = DocumentResponse::from(document.clone());

        assert_eq!(response.page_count, None);
    }

    #[test]
    fn test_document_response_from_document_single_page() {
        let doc_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();
        let uploaded_at = Utc::now();
        let updated_at = Utc::now();

        let document = Document {
            id: doc_id,
            tenant_id,
            filename: "single_page.pdf".to_string(),
            original_filename: "single.pdf".to_string(),
            storage: test_storage_location(
                "https://s3.amazonaws.com/my-bucket/uploads/single_page.pdf",
                "uploads/single_page.pdf",
            ),
            content_type: "application/pdf".to_string(),
            file_size: 256000,
            page_count: Some(1),
            uploaded_at,
            updated_at,
            store_behavior: "temporary".to_string(),
            store_permanently: false,
            expires_at: Some(Utc::now() + chrono::Duration::days(7)),
        };

        let response = DocumentResponse::from(document);

        assert_eq!(response.page_count, Some(1));
    }
}
