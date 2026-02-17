//! Shared key generation for storage backends.
//!
//! Key format: for the default tenant, `media/{filename}`; otherwise `media/{tenant_id}/{filename}`.

use uuid::Uuid;

/// Generate a storage key for the given tenant and filename.
///
/// For the default tenant this produces `media/{filename}`; for other tenants
/// `media/{tenant_id}/{filename}`. All backends must use this format for consistency.
pub fn generate_storage_key(tenant_id: Uuid, filename: &str) -> String {
    if tenant_id == mindia_core::constants::DEFAULT_TENANT_ID {
        format!("media/{}", filename)
    } else {
        format!("media/{}/{}", tenant_id, filename)
    }
}
