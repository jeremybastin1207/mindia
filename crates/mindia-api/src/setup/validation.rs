//! Configuration validation
//!
//! Validates critical configuration values at startup to catch misconfigurations early.

use anyhow::Result;
use mindia_core::Config;

/// Validate critical configuration values
///
/// This function checks that critical configuration is set correctly and will
/// fail fast if there are issues that could cause security problems or runtime errors.
///
/// # Arguments
/// * `config` - Application configuration to validate
///
/// # Returns
/// Ok(()) if validation passes, Err with details if validation fails
pub fn validate_config(config: &Config) -> Result<()> {
    // Validate production mode detection
    let is_production = config.is_production();
    let env_var = std::env::var("ENVIRONMENT")
        .or_else(|_| std::env::var("APP_ENV"))
        .ok();

    if is_production && env_var.is_none() {
        tracing::warn!(
            "Production mode detected but ENVIRONMENT/APP_ENV not set - error details may leak"
        );
    }

    // Validate CORS configuration in production
    if is_production {
        let cors_origins = config.cors_origins();
        if cors_origins.contains(&"*".to_string()) {
            return Err(anyhow::anyhow!(
                "CORS configured to allow all origins (*) in production - this is a security risk. \
                Please set specific allowed origins via CORS_ORIGINS environment variable."
            ));
        }
    }

    // Validate trusted proxy count
    let trusted_proxy_count = std::env::var("TRUSTED_PROXY_COUNT")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1);

    if trusted_proxy_count > 10 {
        tracing::warn!(
            trusted_proxy_count = trusted_proxy_count,
            "TRUSTED_PROXY_COUNT is very high - ensure this matches your actual proxy setup"
        );
    }

    // Validate database connection settings
    if config.db_max_connections() == 0 {
        return Err(anyhow::anyhow!("Database max connections cannot be 0"));
    }

    if config.db_timeout_seconds() == 0 {
        return Err(anyhow::anyhow!("Database timeout cannot be 0"));
    }

    // Validate rate limiting configuration
    if config.http_rate_limit_per_minute() == 0 {
        return Err(anyhow::anyhow!("HTTP rate limit cannot be 0"));
    }

    if let Some(tenant_limit) = config.http_tenant_rate_limit_per_minute() {
        if tenant_limit == 0 {
            return Err(anyhow::anyhow!("Tenant rate limit cannot be 0"));
        }
        if tenant_limit > config.http_rate_limit_per_minute() {
            tracing::warn!(
                tenant_limit = tenant_limit,
                global_limit = config.http_rate_limit_per_minute(),
                "Tenant rate limit is higher than global limit - tenants can exceed global limit"
            );
        }
    }

    // Validate file size limits
    if config.max_file_size_bytes() == 0 {
        return Err(anyhow::anyhow!("Max file size cannot be 0"));
    }

    #[cfg(feature = "video")]
    {
        if config.max_video_size_bytes() == 0 {
            return Err(anyhow::anyhow!("Max video size cannot be 0"));
        }
    }

    #[cfg(feature = "document")]
    {
        if config.max_document_size_bytes() == 0 {
            return Err(anyhow::anyhow!("Max document size cannot be 0"));
        }
    }

    #[cfg(feature = "audio")]
    {
        if config.max_audio_size_bytes() == 0 {
            return Err(anyhow::anyhow!("Max audio size cannot be 0"));
        }
    }

    // Validate JWT secret is set
    if config.jwt_secret().is_empty() {
        return Err(anyhow::anyhow!(
            "JWT secret cannot be empty - set JWT_SECRET environment variable"
        ));
    }

    // Warn about weak JWT secrets in production
    if is_production && config.jwt_secret().len() < 32 {
        tracing::warn!(
            "JWT secret is shorter than 32 characters - consider using a longer, more secure secret"
        );
    }

    tracing::info!("Configuration validation passed");
    Ok(())
}
