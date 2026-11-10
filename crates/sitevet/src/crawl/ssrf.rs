//! Private-address blocking. Checked after DNS resolution, so a public name that resolves to a
//! private address (DNS rebinding) is blocked too.

use std::net::{IpAddr, SocketAddr};

use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use url::{Host, Url};

/// Loopback, private, link-local (including cloud metadata at 169.254.169.254), carrier-grade
/// NAT, unique-local and unspecified addresses.
pub(crate) fn is_private(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let [a, b, ..] = v4.octets();
            v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || a == 0
                || (a == 100 && (64..128).contains(&b))
        }
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_private(IpAddr::V4(v4));
            }
            let first = v6.segments()[0];
            v6.is_loopback() || v6.is_unspecified() || (first & 0xfe00) == 0xfc00 || (first & 0xffc0) == 0xfe80
        }
    }
}

/// True when the URL's host is a private address, or a name that resolves to one.
pub(crate) async fn host_is_private(url: &Url) -> bool {
    match url.host() {
        Some(Host::Ipv4(ip)) => is_private(ip.into()),
        Some(Host::Ipv6(ip)) => is_private(ip.into()),
        Some(Host::Domain(name)) => {
            let port = url.port_or_known_default().unwrap_or(80);
            match tokio::net::lookup_host((name, port)).await {
                Ok(mut addrs) => addrs.any(|a| is_private(a.ip())),
                Err(_) => false,
            }
        }
        None => false,
    }
}

/// A literal IP host that the guard must refuse. Names go through [`GuardedResolver`].
pub(crate) fn literal_is_blocked(url: &Url) -> bool {
    match url.host() {
        Some(Host::Ipv4(ip)) => is_private(ip.into()),
        Some(Host::Ipv6(ip)) => is_private(ip.into()),
        _ => false,
    }
}

pub(crate) const BLOCKED: &str = "blocked: private network address";

/// DNS for the HTTP client. Drops private addresses when blocking is on.
pub(crate) struct GuardedResolver {
    pub(crate) block_private: bool,
}

impl Resolve for GuardedResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let block = self.block_private;
        Box::pin(async move {
            let addrs: Vec<SocketAddr> = tokio::net::lookup_host((name.as_str(), 0)).await?.collect();
            let allowed: Vec<SocketAddr> = addrs.into_iter().filter(|a| !block || !is_private(a.ip())).collect();
            if allowed.is_empty() {
                return Err(BLOCKED.into());
            }
            Ok(Box::new(allowed.into_iter()) as Addrs)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::is_private;

    #[test]
    fn classifies_addresses() {
        for ip in [
            "127.0.0.1",
            "10.1.2.3",
            "172.16.0.1",
            "192.168.1.1",
            "169.254.169.254",
            "100.64.0.1",
            "0.0.0.0",
            "::1",
            "fd00:ec2::254",
            "fe80::1",
            "::ffff:127.0.0.1",
        ] {
            assert!(is_private(ip.parse().unwrap()), "{ip}");
        }
        for ip in ["8.8.8.8", "93.184.216.34", "2606:4700::1111", "100.128.0.1"] {
            assert!(!is_private(ip.parse().unwrap()), "{ip}");
        }
    }
}
