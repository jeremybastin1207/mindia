//! Application-wide constants.

use uuid::Uuid;

/// Default tenant ID used when authenticating with the master API key.
/// Deterministic UUID (v5-style) distinct from Uuid::nil() to avoid confusion with
/// uninitialized or sentinel values. Stable across deployments.
/// Format: d2e8f4a1-7b3c-5d6e-8f9a-0b1c2d3e4f5a
pub const DEFAULT_TENANT_ID: Uuid = Uuid::from_u128(0xd2e8f4a1_7b3c_5d6e_8f9a_0b1c2d3e4f5a);

/// Default user ID for master API key context.
/// Format: e3f9a5b2-8c4d-6e7f-9a0b-1c2d3e4f5a6b
pub const DEFAULT_USER_ID: Uuid = Uuid::from_u128(0xe3f9a5b2_8c4d_6e7f_9a0b_1c2d3e4f5a6b);
