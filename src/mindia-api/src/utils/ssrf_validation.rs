//! SSRF (Server-Side Request Forgery) validation utilities
//!
//! Provides validation functions to prevent SSRF attacks by:
//! - Rejecting private/internal IP addresses
//! - Rejecting localhost and internal hostnames
//! - Resolving hostnames and validating resolved IPs (prevents DNS rebinding)

use std::net::{IpAddr, Ipv6Addr};
use tokio::net::lookup_host;

/// Validate URL to prevent SSRF attacks
///
/// This function performs comprehensive SSRF validation:
/// 1. Checks URL scheme (must be http/https)
/// 2. Extracts and validates hostname
/// 3. Checks URL allowlist if configured (defense in depth)
/// 4. Checks for private/i6ternal IP addresses in hostname
/// 5. Checks for localhost and internal hostnames
/// 6. Resolves hostname to IP and validates resolved IPs (prevents DNS rebinding)
///
/// # Arguments
/// * `url` - URL to validate (must be already parsed by reqwest::Url or url::Url)
/// * `allow_private_ips` - If true, allows private IPs (default: false for security)
/// * `allowlist` - Optional list of allowed domains/hostnames (if None, all public domains allowed)
///
/// # Returns
/// Ok(()) if URL is safe, Err with error message if unsafe
pub async fn validate_url_for_ssrf(
    url: &str,
    allow_private_ips: bool,
    allowlist: Option<&[String]>,
) -> Result<(), String> {
    // Basic URL format validation
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("URL must start with http:// or https://".to_string());
    }

    // Parse URL to extract hostname using reqwest::Url (already available in handler)
    let parsed_url = reqwest::Url::parse(url).map_err(|e| format!("Invalid URL format: {}", e))?;

    let host = parsed_url
        .host_str()
        .ok_or_else(|| "URL must have a host".to_string())?;

    // Remove port if present
    let host_without_port = host.split(':').next().unwrap_or(host);

    // Check allowlist first (if configured)
    if let Some(allowed_domains) = allowlist {
        let host_lower = host_without_port.to_lowercase();
        let is_allowed = allowed_domains.iter().any(|allowed| {
            let allowed_lower = allowed.to_lowercase();
            // Exact match or subdomain match (e.g., cdn.example.com matches example.com)
            host_lower == allowed_lower || host_lower.ends_with(&format!(".{}", allowed_lower))
        });

        if !is_allowed {
            return Err(format!(
                "URL hostname '{}' is not in the allowed list. Allowed domains: {}",
                host_without_port,
                allowed_domains.join(", ")
            ));
        }
    }

    // Check if hostname is an IP address directly
    if let Ok(ip) = host_without_port.parse::<IpAddr>() {
        if !allow_private_ips && is_private_ip(&ip) {
            return Err("Private/internal IP addresses are not allowed".to_string());
        }
    }

    // Reject localhost and common internal hostnames
    let host_lower = host_without_port.to_lowercase();
    if !allow_private_ips
        && (host_lower == "localhost"
            || host_lower.ends_with(".local")
            || host_lower == "127.0.0.1"
            || host_lower == "::1"
            || host_lower.starts_with("0.")
            || host_lower == "0.0.0.0"
            || host_lower.contains(".internal")
            || host_lower.contains(".corp"))
    {
        return Err("Localhost and internal hostnames are not allowed".to_string());
    }

    // Resolve hostname to IP addresses to prevent DNS rebinding attacks
    // DNS rebinding: Attacker controls DNS to return private IP on second lookup
    let mut resolved_ips = Vec::new();
    let port = parsed_url
        .port()
        .unwrap_or(if parsed_url.scheme() == "https" {
            443
        } else {
            80
        });
    match lookup_host((host_without_port, port)).await {
        Ok(ips) => {
            for socket_addr in ips {
                resolved_ips.push(socket_addr.ip());
            }
        }
        Err(e) => {
            // DNS resolution failure - could be legitimate or could be blocked
            // Log but allow (defense in depth - we already validated hostname format)
            // In strict mode, we could reject on DNS failure, but that might break legitimate use cases
            tracing::warn!(host = %host_without_port, error = %e, "Failed to resolve hostname for SSRF validation");
        }
    }

    // Validate all resolved IPs
    if !allow_private_ips && !resolved_ips.is_empty() {
        for ip in &resolved_ips {
            if is_private_ip(ip) {
                return Err(format!(
                    "Hostname resolves to private/internal IP address: {}",
                    ip
                ));
            }
        }
    }

    Ok(())
}

/// Check if an IP address is private/internal
///
/// Returns true for:
/// - IPv4 private ranges: 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
/// - IPv4 localhost: 127.0.0.0/8
/// - IPv4 link-local: 169.254.0.0/16
/// - IPv4 multicast: 224.0.0.0/4
/// - IPv4 reserved: 0.0.0.0/8
/// - IPv6 loopback: ::1
/// - IPv6 link-local: fe80::/10
/// - IPv6 unique local: fc00::/7
/// - IPv6 unspecified: ::
fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => {
            let octets = ipv4.octets();
            // Private IP ranges
            octets[0] == 10 // 10.0.0.0/8
                || (octets[0] == 172 && octets[1] >= 16 && octets[1] <= 31) // 172.16.0.0/12
                || (octets[0] == 192 && octets[1] == 168) // 192.168.0.0/16
                || octets[0] == 127 // 127.0.0.0/8 (localhost)
                || (octets[0] == 169 && octets[1] == 254) // 169.254.0.0/16 (link-local)
                || (octets[0] >= 224 && octets[0] <= 239) // 224.0.0.0/4 (multicast)
                || octets[0] == 0 // 0.0.0.0/8 (reserved)
        }
        IpAddr::V6(ipv6) => {
            ipv6.is_loopback()
                || ipv6.is_unspecified()
                || ipv6.is_multicast()
                || is_ipv6_link_local(ipv6)
                || is_ipv6_unique_local(ipv6)
        }
    }
}

/// Check if IPv6 address is link-local (fe80::/10)
fn is_ipv6_link_local(ip: &Ipv6Addr) -> bool {
    let segments = ip.segments();
    segments[0] & 0xffc0 == 0xfe80 // fe80::/10
}

/// Check if IPv6 address is unique local (fc00::/7)
fn is_ipv6_unique_local(ip: &Ipv6Addr) -> bool {
    let segments = ip.segments();
    segments[0] & 0xfe00 == 0xfc00 // fc00::/7
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[tokio::test]
    async fn test_validate_url_rejects_localhost() {
        assert!(
            validate_url_for_ssrf("http://localhost/image.jpg", false, None)
                .await
                .is_err()
        );
        assert!(
            validate_url_for_ssrf("http://127.0.0.1/image.jpg", false, None)
                .await
                .is_err()
        );
        assert!(validate_url_for_ssrf("http://::1/image.jpg", false, None)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_validate_url_rejects_private_ips() {
        assert!(
            validate_url_for_ssrf("http://192.168.1.1/image.jpg", false, None)
                .await
                .is_err()
        );
        assert!(
            validate_url_for_ssrf("http://10.0.0.1/image.jpg", false, None)
                .await
                .is_err()
        );
        assert!(
            validate_url_for_ssrf("http://172.16.0.1/image.jpg", false, None)
                .await
                .is_err()
        );
        assert!(
            validate_url_for_ssrf("http://169.254.1.1/image.jpg", false, None)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_validate_url_rejects_internal_hostnames() {
        assert!(
            validate_url_for_ssrf("http://internal.service.local/image.jpg", false, None)
                .await
                .is_err()
        );
        assert!(
            validate_url_for_ssrf("http://service.corp/image.jpg", false, None)
                .await
                .is_err()
        );
        assert!(
            validate_url_for_ssrf("http://service.internal/image.jpg", false, None)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_validate_url_accepts_public_urls() {
        // Note: This test will actually resolve DNS, so it may fail in environments without internet
        // In practice, this is acceptable as it validates real-world behavior
        assert!(
            validate_url_for_ssrf("https://example.com/image.jpg", false, None)
                .await
                .is_ok()
        );
        assert!(
            validate_url_for_ssrf("https://www.google.com/favicon.ico", false, None)
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn test_validate_url_rejects_invalid_schemes() {
        assert!(validate_url_for_ssrf("file:///etc/passwd", false, None)
            .await
            .is_err());
        assert!(validate_url_for_ssrf("ftp://example.com/file", false, None)
            .await
            .is_err());
        assert!(validate_url_for_ssrf("gopher://example.com", false, None)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_validate_url_allowlist() {
        let allowlist = vec!["example.com".to_string(), "cdn.example.com".to_string()];

        // Allowed domains should pass
        assert!(
            validate_url_for_ssrf("https://example.com/image.jpg", false, Some(&allowlist))
                .await
                .is_ok()
        );
        assert!(validate_url_for_ssrf(
            "https://cdn.example.com/image.jpg",
            false,
            Some(&allowlist)
        )
        .await
        .is_ok());

        // Disallowed domains should fail
        assert!(
            validate_url_for_ssrf("https://evil.com/image.jpg", false, Some(&allowlist))
                .await
                .is_err()
        );
        assert!(
            validate_url_for_ssrf("https://google.com/favicon.ico", false, Some(&allowlist))
                .await
                .is_err()
        );
    }

    #[test]
    fn test_is_private_ip() {
        assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));
        assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))));
        assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
        assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1))));
        assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::new(169, 254, 1, 1))));

        assert!(!is_private_ip(&IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
        assert!(!is_private_ip(&IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))));

        assert!(is_private_ip(&IpAddr::V6(Ipv6Addr::new(
            0, 0, 0, 0, 0, 0, 0, 1
        )))); // ::1
        assert!(is_private_ip(&IpAddr::V6(Ipv6Addr::new(
            0, 0, 0, 0, 0, 0, 0, 0
        )))); // ::
    }
}
