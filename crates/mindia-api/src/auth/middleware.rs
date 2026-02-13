use crate::auth::api_key::{extract_key_prefix, verify_api_key};
use crate::auth::models::TenantContext;
use crate::error::HttpAppError;
use crate::middleware::audit;
use crate::utils::ip_extraction::{extract_client_ip, ClientIp};
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use mindia_core::AppError;
use mindia_db::{ApiKeyRepository, TenantRepository};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use subtle::ConstantTimeEq;
use tokio::sync::Mutex;

const API_KEY_PREFIX: &str = "mk_live_";

#[derive(Clone)]
pub struct AuthFailureLimiter {
    inner: Arc<Mutex<HashMap<String, (u32, Instant)>>>,
    max_failures: u32,
    window: Duration,
}

impl AuthFailureLimiter {
    pub fn new(max_failures: u32, window_seconds: u64) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            max_failures,
            window: Duration::from_secs(window_seconds),
        }
    }

    pub async fn record_failure(&self, ip: &str) -> bool {
        let mut guard = self.inner.lock().await;
        let now = Instant::now();
        let (count, reset_at) = guard.entry(ip.to_string()).or_insert((0, now + self.window));
        if now >= *reset_at {
            *count = 0;
            *reset_at = now + self.window;
        }
        *count += 1;
        *count >= self.max_failures
    }

    pub async fn is_blocked(&self, ip: &str) -> bool {
        let mut guard = self.inner.lock().await;
        if let Some((count, reset_at)) = guard.get(ip) {
            if Instant::now() >= *reset_at {
                guard.remove(ip);
                return false;
            }
            return *count >= self.max_failures;
        }
        false
    }
}

#[derive(Clone)]
pub struct AuthState {
    pub master_api_key: String,
    pub api_key_repository: ApiKeyRepository,
    pub tenant_repository: TenantRepository,
    pub auth_failure_limiter: Option<Arc<AuthFailureLimiter>>,
}

fn secure_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

pub async fn auth_middleware(
    State(auth_state): State<Arc<AuthState>>,
    mut request: Request,
    next: Next,
) -> Response {
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
    let client_ip_str = client_ip.as_deref().unwrap_or("unknown");
    if let Some(ref limiter) = auth_state.auth_failure_limiter {
        if limiter.is_blocked(client_ip_str).await {
            return (StatusCode::TOO_MANY_REQUESTS, "Too many failed auth attempts").into_response();
        }
    }

    let auth_header = match request
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
    {
        Some(h) => h,
        None => {
            if let Some(ref limiter) = auth_state.auth_failure_limiter {
                if limiter.record_failure(client_ip_str).await {
                    return (StatusCode::TOO_MANY_REQUESTS, "Too many failed auth attempts").into_response();
                }
            }
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

    if !auth_header.starts_with("Bearer ") {
        if let Some(ref limiter) = auth_state.auth_failure_limiter {
            if limiter.record_failure(client_ip_str).await {
                return (StatusCode::TOO_MANY_REQUESTS, "Too many failed auth attempts").into_response();
            }
        }
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
    let user_agent = request
        .headers()
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    if secure_compare(token, &auth_state.master_api_key) {
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

        audit::log_authentication_attempt(
            Some(tenant_context.tenant_id),
            Some(tenant_context.user_id),
            None,
            client_ip.clone(),
            user_agent,
            true,
            None,
        );

        request.extensions_mut().insert(ClientIp(
            client_ip.clone().unwrap_or_else(|| "unknown".to_string()),
        ));
        request.extensions_mut().insert(tenant_context);
        return next.run(request).await;
    }

    if token.starts_with(API_KEY_PREFIX) {
        match authenticate_generated_key(
            token,
            &auth_state.api_key_repository,
            &auth_state.tenant_repository,
            client_ip.clone(),
            user_agent.clone(),
        )
        .await
        {
            Ok((tenant_context, api_key_id)) => {
                let api_key_repo = auth_state.api_key_repository.clone();
                tokio::spawn(async move {
                    let _ = api_key_repo.update_last_used(api_key_id).await;
                });

                audit::log_authentication_attempt(
                    Some(tenant_context.tenant_id),
                    Some(tenant_context.user_id),
                    Some(api_key_id),
                    client_ip.clone(),
                    user_agent,
                    true,
                    None,
                );

                request.extensions_mut().insert(ClientIp(
                    client_ip.clone().unwrap_or_else(|| "unknown".to_string()),
                ));
                request.extensions_mut().insert(tenant_context);
                return next.run(request).await;
            }
            Err(e) => {
                if let Some(ref limiter) = auth_state.auth_failure_limiter {
                    if limiter.record_failure(client_ip_str).await {
                        return (StatusCode::TOO_MANY_REQUESTS, "Too many failed auth attempts").into_response();
                    }
                }
                audit::log_authentication_attempt(
                    None,
                    None,
                    None,
                    client_ip,
                    user_agent,
                    false,
                    Some(e.to_string()),
                );
                return HttpAppError(AppError::Unauthorized(e.to_string())).into_response();
            }
        }
    }

    if let Some(ref limiter) = auth_state.auth_failure_limiter {
        if limiter.record_failure(client_ip_str).await {
            return (StatusCode::TOO_MANY_REQUESTS, "Too many failed auth attempts").into_response();
        }
    }
    audit::log_authentication_attempt(
        None,
        None,
        None,
        client_ip,
        user_agent,
        false,
        Some("Invalid API key".to_string()),
    );
    HttpAppError(AppError::Unauthorized("Invalid API key".to_string())).into_response()
}

async fn authenticate_generated_key(
    token: &str,
    api_key_repo: &ApiKeyRepository,
    tenant_repo: &TenantRepository,
    _client_ip: Option<String>,
    _user_agent: Option<String>,
) -> Result<(TenantContext, uuid::Uuid), AppError> {
    use crate::auth::models::UserRole;
    use mindia_core::models::{Tenant, TenantStatus};

    let prefix = extract_key_prefix(token);
    let candidates = api_key_repo.get_by_key_prefix(&prefix).await?;

    for api_key in candidates {
        if !api_key.is_active {
            continue;
        }
        if ApiKeyRepository::is_expired(&api_key) {
            return Err(AppError::Unauthorized("API key has expired".to_string()));
        }
        if verify_api_key(token, &api_key.key_hash)
            .map_err(|e| AppError::Internal(e.to_string()))?
        {
            let tenant = tenant_repo
                .get_tenant_by_id(api_key.tenant_id)
                .await?
                .ok_or_else(|| AppError::Internal("Tenant not found for API key".to_string()))?;

            if tenant.status != TenantStatus::Active {
                return Err(AppError::Unauthorized("Tenant is not active".to_string()));
            }

            let tenant_context = TenantContext {
                tenant_id: api_key.tenant_id,
                user_id: api_key.id, // Use api_key id as user identifier for generated keys
                role: UserRole::Admin,
                tenant: Tenant {
                    id: tenant.id,
                    name: tenant.name,
                    status: tenant.status,
                    created_at: tenant.created_at,
                    updated_at: tenant.updated_at,
                },
            };

            return Ok((tenant_context, api_key.id));
        }
    }

    Err(AppError::Unauthorized("Invalid API key".to_string()))
}
