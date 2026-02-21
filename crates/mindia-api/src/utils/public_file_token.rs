//! Signed token for public file access (no auth).
//!
//! Payload: expiry_ts (u64 BE) || tenant_id (16 bytes) || media_id (16 bytes) = 40 bytes.
//! Token = base64url(payload || HMAC-SHA256(secret, payload)).

use crate::error::HttpAppError;
use base64::Engine;
use hmac::{Hmac, Mac};
use mindia_core::AppError;
use sha2::Sha256;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

const PAYLOAD_LEN: usize = 8 + 16 + 16; // expiry + tenant_id + media_id
const MAC_LEN: usize = 32; // SHA256
const TOKEN_LEN: usize = PAYLOAD_LEN + MAC_LEN;

/// Build a signed token for public file access.
pub fn create(tenant_id: Uuid, media_id: Uuid, expires_in: Duration, secret: &[u8]) -> String {
    let expiry_ts = SystemTime::now()
        .checked_add(expires_in)
        .unwrap_or(SystemTime::UNIX_EPOCH)
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let mut payload = [0u8; PAYLOAD_LEN];
    payload[0..8].copy_from_slice(&expiry_ts.to_be_bytes());
    payload[8..24].copy_from_slice(tenant_id.as_bytes());
    payload[24..40].copy_from_slice(media_id.as_bytes());

    let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("HMAC accepts any key size");
    mac.update(&payload);
    let tag = mac.finalize().into_bytes();

    let mut token_bytes = [0u8; TOKEN_LEN];
    token_bytes[0..PAYLOAD_LEN].copy_from_slice(&payload);
    token_bytes[PAYLOAD_LEN..].copy_from_slice(&tag);

    base64_url_encode(&token_bytes)
}

/// Verify token and return (tenant_id, media_id) or error.
pub fn verify(token: &str, secret: &[u8]) -> Result<(Uuid, Uuid), HttpAppError> {
    let decoded = base64_url_decode(token).map_err(|_| {
        HttpAppError::from(AppError::InvalidInput(
            "Invalid public file token".to_string(),
        ))
    })?;
    if decoded.len() != TOKEN_LEN {
        return Err(HttpAppError::from(AppError::InvalidInput(
            "Invalid public file token".to_string(),
        )));
    }
    let (payload, tag) = decoded.split_at(PAYLOAD_LEN);
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("HMAC accepts any key size");
    mac.update(payload);
    mac.verify_slice(tag).map_err(|_| {
        HttpAppError::from(AppError::InvalidInput(
            "Invalid public file token".to_string(),
        ))
    })?;

    let expiry_ts = u64::from_be_bytes(payload[0..8].try_into().unwrap());
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    if now > expiry_ts {
        return Err(HttpAppError::from(AppError::InvalidInput(
            "Public file token has expired".to_string(),
        )));
    }

    let tenant_id = Uuid::from_bytes(payload[8..24].try_into().unwrap());
    let media_id = Uuid::from_bytes(payload[24..40].try_into().unwrap());
    Ok((tenant_id, media_id))
}

fn base64_url_encode(data: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}

fn base64_url_decode(s: &str) -> Result<Vec<u8>, base64::DecodeError> {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(s)
}
