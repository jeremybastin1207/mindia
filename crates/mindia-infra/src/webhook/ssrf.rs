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
            || host_lower.contains(".internal")
            || host_lower.contains(".corp"))
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
            tracing::warn!(host = %host_without_port, error = %e, "Failed to resolve hostname for SSRF validation");
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
