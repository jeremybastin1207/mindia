use crate::auth::models::TenantContext;
use crate::error::HttpAppError;
use crate::middleware::audit;
use crate::utils::ip_extraction::extract_client_ip;
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};
use mindia_core::AppError;
use std::sync::Arc;
use subtle::ConstantTimeEq;

#[derive(Clone)]
pub struct AuthState {
    pub master_api_key: String,
}

/// Constant-time comparison of two strings to prevent timing attacks on API key validation.
fn secure_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

/// Middleware to authenticate requests using master API key
pub async fn auth_middleware(
    State(auth_state): State<Arc<AuthState>>,
    mut request: Request,
    next: Next,
) -> Response {
    // Extract Authorization header
    let auth_header = match request
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
    {
        Some(h) => h,
        None => {
            audit::log_authentication_attempt(
                None,
                None,
                None,
                None,
                None,
                false,
                Some("Missing authorization header".to_string()),
            );
            return HttpAppError(AppError::Unauthorized(
                "Missing authorization header".to_string(),
            ))
            .into_response();
        }
    };

    // Check for Bearer token
    if !auth_header.starts_with("Bearer ") {
        audit::log_authentication_attempt(
            None,
            None,
            None,
            None,
            None,
            false,
            Some("Invalid authorization header format".to_string()),
        );
        return HttpAppError(AppError::Unauthorized(
            "Invalid authorization header format".to_string(),
        ))
        .into_response();
    }

    let token = &auth_header[7..]; // Remove "Bearer " prefix

    // Extract client IP and user agent for audit logging
    let trusted_proxy_count = std::env::var("TRUSTED_PROXY_COUNT")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1);
    let socket_addr = request.extensions().get::<std::net::SocketAddr>().copied();
    let client_ip = Some(extract_client_ip(
        request.headers(),
        socket_addr.as_ref(),
        trusted_proxy_count,
    ));
    let user_agent = request
        .headers()
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    // Check if token matches master API key (constant-time comparison to prevent timing attacks)
    if !secure_compare(token, &auth_state.master_api_key) {
        audit::log_authentication_attempt(
            None,
            None,
            None,
            client_ip,
            user_agent,
            false,
            Some("Invalid API key".to_string()),
        );
        return HttpAppError(AppError::Unauthorized("Invalid API key".to_string())).into_response();
    }

    // Create default tenant context (deterministic UUID, not nil, for security and clarity)
    use crate::auth::models::UserRole;
    use mindia_core::constants::{DEFAULT_TENANT_ID, DEFAULT_USER_ID};
    use mindia_core::models::{Tenant, TenantStatus};

    let default_tenant_id = DEFAULT_TENANT_ID;
    let default_user_id = DEFAULT_USER_ID;
    let now = chrono::Utc::now();

    let tenant_context = TenantContext {
        tenant_id: default_tenant_id,
        user_id: default_user_id,
        role: UserRole::Admin,
        tenant: Tenant {
            id: default_tenant_id,
            name: "default".to_string(),
            status: TenantStatus::Active,
            created_at: now,
            updated_at: now,
        },
    };

    // Log successful authentication
    audit::log_authentication_attempt(
        Some(tenant_context.tenant_id),
        Some(tenant_context.user_id),
        None,
        client_ip,
        user_agent,
        true,
        None,
    );

    // Insert tenant context into request extensions
    request.extensions_mut().insert(tenant_context);

    // Continue to next middleware/handler
    next.run(request).await
}
