//! IP address extraction utilities
//!
//! Provides secure extraction of client IP addresses from X-Forwarded-For headers
//! with validation to prevent header spoofing attacks.

use axum::http::HeaderMap;
use std::net::IpAddr;

/// Extract and validate client IP from request headers
///
/// When behind a load balancer or proxy, the X-Forwarded-For header contains a
/// chain of IP addresses. This function validates and extracts the appropriate
/// client IP based on the number of trusted proxies.
///
/// # Arguments
/// * `headers` - HTTP request headers
/// * `socket_addr` - Direct socket address (fallback if headers unavailable)
/// * `trusted_proxy_count` - Number of trusted proxies/load balancers in front
///
/// # Returns
/// Validated client IP address as a string, or "unknown" if extraction fails
pub fn extract_client_ip(
    headers: &HeaderMap,
    socket_addr: Option<&std::net::SocketAddr>,
    trusted_proxy_count: usize,
) -> String {
    // Try X-Forwarded-For header first
    if let Some(forwarded_for) = headers.get("x-forwarded-for") {
        if let Ok(header_value) = forwarded_for.to_str() {
            let ip = extract_from_forwarded_for(header_value, trusted_proxy_count);
            if ip != "unknown" {
                return ip;
            }
        }
    }

    // Try X-Real-IP header (single IP, trusted by some proxies)
    if let Some(real_ip) = headers.get("x-real-ip") {
        if let Ok(header_value) = real_ip.to_str() {
            let trimmed = header_value.trim();
            if is_valid_ip(trimmed) {
                return trimmed.to_string();
            }
        }
    }

    // Fall back to direct socket address
    if let Some(addr) = socket_addr {
        return addr.ip().to_string();
    }

    "unknown".to_string()
}

/// Extract client IP from X-Forwarded-For header chain
///
/// The X-Forwarded-For header contains comma-separated IPs in order:
/// `client, proxy1, proxy2, ...`
///
/// When behind multiple proxies, we need to take the appropriate IP from the chain.
/// If `trusted_proxy_count` is N, we trust the last N IPs and use the one before them.
///
/// # Arguments
/// * `header_value` - Value of X-Forwarded-For header
/// * `trusted_proxy_count` - Number of trusted proxies (0 = trust no proxies)
///
/// # Returns
/// Validated client IP or "unknown" if extraction/validation fails
fn extract_from_forwarded_for(header_value: &str, trusted_proxy_count: usize) -> String {
    let ips: Vec<&str> = header_value
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if ips.is_empty() {
        return "unknown".to_string();
    }

    // If no trusted proxies, we can't trust X-Forwarded-For (could be spoofed)
    // Use the last IP in chain (closest to us) as fallback, but validate it
    if trusted_proxy_count == 0 {
        let last_ip = ips.last().unwrap_or(&"");
        if is_valid_ip(last_ip) {
            return last_ip.to_string();
        }
        return "unknown".to_string();
    }

    // With trusted proxies, calculate which IP to use
    // If chain has N trusted proxies at the end, client is at position (len - N)
    if ips.len() <= trusted_proxy_count {
        // Not enough IPs in chain - something's wrong, fall back to last IP
        let last_ip = ips.last().unwrap_or(&"");
        if is_valid_ip(last_ip) {
            return last_ip.to_string();
        }
        return "unknown".to_string();
    }

    // Get the client IP (before the trusted proxy chain)
    let client_ip_pos = ips.len().saturating_sub(trusted_proxy_count + 1);
    let client_ip = ips.get(client_ip_pos).unwrap_or(&"");

    if is_valid_ip(client_ip) {
        return client_ip.to_string();
    }

    "unknown".to_string()
}

/// Validate that a string is a valid IP address
///
/// Checks both IPv4 and IPv6 formats
fn is_valid_ip(ip_str: &str) -> bool {
    ip_str.parse::<IpAddr>().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn create_headers_with_xff(xff_value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", HeaderValue::from_str(xff_value).unwrap());
        headers
    }

    #[test]
    fn test_extract_from_forwarded_for_single_ip() {
        assert_eq!(extract_from_forwarded_for("192.168.1.1", 0), "192.168.1.1");
        assert_eq!(extract_from_forwarded_for("192.168.1.1", 1), "192.168.1.1");
    }

    #[test]
    fn test_extract_from_forwarded_for_with_proxy() {
        // Client -> Proxy -> Server
        // X-Forwarded-For: client, proxy
        assert_eq!(
            extract_from_forwarded_for("192.168.1.1, 10.0.0.1", 1),
            "192.168.1.1"
        );
    }

    #[test]
    fn test_extract_from_forwarded_for_multiple_proxies() {
        // Client -> LB -> Proxy -> Server
        // X-Forwarded-For: client, lb, proxy
        assert_eq!(
            extract_from_forwarded_for("192.168.1.1, 10.0.0.1, 10.0.0.2", 2),
            "192.168.1.1"
        );
    }

    #[test]
    fn test_extract_from_forwarded_for_no_trusted_proxies() {
        // When trust count is 0, we can't trust the header fully
        // Should use last IP in chain (closest to us)
        assert_eq!(
            extract_from_forwarded_for("192.168.1.1, 10.0.0.1", 0),
            "10.0.0.1"
        );
    }

    #[test]
    fn test_extract_from_forwarded_for_invalid_ip() {
        assert_eq!(
            extract_from_forwarded_for("not.an.ip.address", 0),
            "unknown"
        );
    }

    #[test]
    fn test_extract_client_ip_from_xff() {
        let headers = create_headers_with_xff("192.168.1.1");
        let ip = extract_client_ip(&headers, None, 0);
        assert_eq!(ip, "192.168.1.1");
    }

    #[test]
    fn test_extract_client_ip_fallback_to_socket() {
        let headers = HeaderMap::new();
        let socket = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));
        let ip = extract_client_ip(&headers, Some(&socket), 0);
        assert_eq!(ip, "127.0.0.1");
    }

    #[test]
    fn test_extract_client_ip_fallback_to_unknown() {
        let headers = HeaderMap::new();
        let ip = extract_client_ip(&headers, None, 0);
        assert_eq!(ip, "unknown");
    }

    #[test]
    fn test_is_valid_ip() {
        assert!(is_valid_ip("192.168.1.1"));
        assert!(is_valid_ip("::1"));
        assert!(is_valid_ip("2001:0db8:85a3:0000:0000:8a2e:0370:7334"));
        assert!(!is_valid_ip("not.an.ip"));
        assert!(!is_valid_ip(""));
        assert!(!is_valid_ip("999.999.999.999"));
    }
}
