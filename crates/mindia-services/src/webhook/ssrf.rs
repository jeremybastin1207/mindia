//! SSRF (Server-Side Request Forgery) validation for webhook URLs.
//!
//! Validates URLs before webhook delivery to prevent requests to internal/private hosts.

use std::net::{IpAddr, Ipv6Addr};
use tokio::net::lookup_host;

/// Validate URL to prevent SSRF attacks before sending webhook.
///
/// Rejects private/internal IPs, localhost, and internal hostnames.
/// Resolves hostname to validate resolved IPs (prevents DNS rebinding).
pub async fn validate_url_for_ssrf(
    url: &str,
    allow_private_ips: bool,
    allowlist: Option<&[String]>,
) -> Result<(), String> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("URL must start with http:// or https://".to_string());
    }

    let parsed_url = reqwest::Url::parse(url).map_err(|e| format!("Invalid URL format: {}", e))?;

    let host = parsed_url
        .host_str()
        .ok_or_else(|| "URL must have a host".to_string())?;

    let host_without_port = host.split(':').next().unwrap_or(host);

    if let Some(allowed_domains) = allowlist {
        let host_lower = host_without_port.to_lowercase();
        let is_allowed = allowed_domains.iter().any(|allowed| {
            let allowed_lower = allowed.to_lowercase();
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

    if let Ok(ip) = host_without_port.parse::<IpAddr>() {
        if !allow_private_ips && is_private_ip(&ip) {
            return Err("Private/internal IP addresses are not allowed".to_string());
        }
    }

    let host_lower = host_without_port.to_lowercase();
    if !allow_private_ips
        && (host_lower == "localhost"
            || host_lower.ends_with(".local")
            || host_lower == "127.0.0.1"
            || host_lower == "::1"
            || host_lower.starts_with("0.")
            || host_lower == "0.0.0.0"
            || host_lower.ends_with(".internal")
            || host_lower.ends_with(".corp"))
    {
        return Err("Localhost and internal hostnames are not allowed".to_string());
    }

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
            tracing::warn!(host = %host_without_port, error = %e, "DNS resolution failed for SSRF validation");
            return Err(format!(
                "Hostname could not be resolved (SSRF check): {}",
                e
            ));
        }
    }

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

fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => {
            let octets = ipv4.octets();
            octets[0] == 10
                || (octets[0] == 172 && octets[1] >= 16 && octets[1] <= 31)
                || (octets[0] == 192 && octets[1] == 168)
                || octets[0] == 127
                || (octets[0] == 169 && octets[1] == 254)
                || (octets[0] >= 224 && octets[0] <= 239)
                || octets[0] == 0
        }
        IpAddr::V6(ipv6) => {
            // Check IPv4-mapped IPv6 addresses (::ffff:x.x.x.x) - they bypass V4-only checks
            if let Some(ipv4) = ipv6.to_ipv4_mapped() {
                return is_private_ip(&IpAddr::V4(ipv4));
            }
            ipv6.is_loopback()
                || ipv6.is_unspecified()
                || ipv6.is_multicast()
                || is_ipv6_link_local(ipv6)
                || is_ipv6_unique_local(ipv6)
        }
    }
}

fn is_ipv6_link_local(ip: &Ipv6Addr) -> bool {
    let segments = ip.segments();
    segments[0] & 0xffc0 == 0xfe80
}

fn is_ipv6_unique_local(ip: &Ipv6Addr) -> bool {
    let segments = ip.segments();
    segments[0] & 0xfe00 == 0xfc00
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ssrf_rejects_invalid_scheme() {
        assert!(validate_url_for_ssrf("ftp://example.com", false, None)
            .await
            .is_err());
        assert!(validate_url_for_ssrf("file:///etc/passwd", false, None)
            .await
            .is_err());
        assert!(validate_url_for_ssrf("javascript:alert(1)", false, None)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_ssrf_rejects_localhost() {
        assert!(validate_url_for_ssrf("http://localhost/", false, None)
            .await
            .is_err());
        assert!(validate_url_for_ssrf("https://localhost:443/", false, None)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_ssrf_rejects_127_0_0_1() {
        assert!(validate_url_for_ssrf("http://127.0.0.1/", false, None)
            .await
            .is_err());
        assert!(
            validate_url_for_ssrf("http://127.0.0.1:3000/webhook", false, None)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_ssrf_rejects_private_ipv4() {
        assert!(validate_url_for_ssrf("http://10.0.0.1/", false, None)
            .await
            .is_err());
        assert!(validate_url_for_ssrf("http://192.168.1.1/", false, None)
            .await
            .is_err());
        assert!(validate_url_for_ssrf("http://172.16.0.1/", false, None)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_ssrf_rejects_ipv4_mapped_ipv6() {
        // ::ffff:127.0.0.1 and ::ffff:10.0.0.1 are IPv4-mapped IPv6; must be rejected
        assert!(
            validate_url_for_ssrf("http://[::ffff:127.0.0.1]/", false, None)
                .await
                .is_err()
        );
        assert!(
            validate_url_for_ssrf("http://[::ffff:10.0.0.1]/", false, None)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_ssrf_rejects_internal_hostnames() {
        assert!(validate_url_for_ssrf("http://foo.internal/", false, None)
            .await
            .is_err());
        assert!(validate_url_for_ssrf("http://api.corp/", false, None)
            .await
            .is_err());
        assert!(validate_url_for_ssrf("http://machine.local/", false, None)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_ssrf_allowlist_allows_listed_host() {
        // Use public IP to avoid DNS; allowlist permits it
        let allowlist = vec!["8.8.8.8".to_string()];
        let result = validate_url_for_ssrf("http://8.8.8.8/", false, Some(&allowlist)).await;
        assert!(result.is_ok(), "expected Ok, got {:?}", result);
    }

    #[tokio::test]
    async fn test_ssrf_allowlist_rejects_unlisted_domain() {
        let allowlist = vec!["allowed.com".to_string()];
        assert!(
            validate_url_for_ssrf("http://evil.com/", false, Some(&allowlist))
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_ssrf_require_http_or_https() {
        assert!(validate_url_for_ssrf("", false, None).await.is_err());
        assert!(validate_url_for_ssrf("not-a-url", false, None)
            .await
            .is_err());
    }
}
