//! Host/Origin checks with FastMCP's semantics (`fastmcp/server/http.py`,
//! `HostOriginGuardMiddleware` in strict mode), which the env allow-lists were
//! written for: ports are ignored on hosts, entries are `fnmatch` globs, and a
//! loopback Origin is accepted whenever the Host is loopback. rmcp's own check
//! matches exactly, so it is disabled in favor of this one.

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use axum::extract::{ConnectInfo, Request};
use axum::http::{StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

pub const LOOPBACK_HOSTS: &[&str] = &["127.0.0.1", "localhost", "::1"];

pub struct Guard {
    hosts: Vec<String>,
    origins: Vec<String>,
}

impl Guard {
    pub fn new(extra_hosts: Vec<String>, origins: Vec<String>) -> Arc<Self> {
        let mut hosts: Vec<String> = LOOPBACK_HOSTS.iter().map(|h| h.to_string()).collect();
        hosts.extend(extra_hosts);
        Arc::new(Self { hosts, origins })
    }

    pub async fn check(self: Arc<Self>, req: Request, next: Next) -> Response {
        let host = req
            .headers()
            .get(header::HOST)
            .and_then(|h| h.to_str().ok())
            .map(str::to_string)
            .or_else(|| req.uri().authority().map(|a| a.to_string()))
            .unwrap_or_default();
        if !self.host_allowed(&host) {
            return (StatusCode::MISDIRECTED_REQUEST, "Misdirected Request").into_response();
        }
        if let Some(origin) = req.headers().get(header::ORIGIN)
            && !origin.is_empty()
        {
            let origin = String::from_utf8_lossy(origin.as_bytes());
            let peer = req
                .extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .map(|c| c.0.ip());
            let forwarded = req
                .headers()
                .get_all("x-forwarded-proto")
                .iter()
                .next_back()
                .map(|v| String::from_utf8_lossy(v.as_bytes()).into_owned());
            let scheme = request_scheme(peer, forwarded.as_deref());
            if !self.origin_allowed(&origin, scheme, &host) {
                return (StatusCode::FORBIDDEN, "Forbidden Origin").into_response();
            }
        }
        next.run(req).await
    }

    fn host_allowed(&self, host: &str) -> bool {
        let host = normalize_host(host);
        self.hosts.iter().any(|allowed| {
            let pattern = normalize_host(allowed);
            pattern == "*" || fnmatch(&host, &pattern)
        })
    }

    fn origin_allowed(&self, origin: &str, scheme: &str, host: &str) -> bool {
        let normalized = normalize_origin(origin);
        if self.origins.iter().any(|allowed| {
            let pattern = normalize_origin(allowed);
            pattern == "*" || fnmatch(&normalized, &pattern)
        }) {
            return true;
        }
        if is_loopback(&split_url(origin.trim()).host) && is_loopback(host) {
            return true;
        }
        normalized == normalize_origin(&format!("{scheme}://{host}"))
    }
}

/// uvicorn's ASGI scheme: `X-Forwarded-Proto` (its last occurrence, exactly one
/// of these values after trimming) counts only from a trusted proxy. uvicorn
/// trusts 127.0.0.1 by default; any loopback peer is trusted here.
fn request_scheme(peer: Option<IpAddr>, forwarded_proto: Option<&str>) -> &str {
    match forwarded_proto.map(str::trim) {
        Some(proto @ ("http" | "https" | "ws" | "wss"))
            if peer.is_some_and(|ip| ip.is_loopback()) =>
        {
            proto
        }
        _ => "http",
    }
}

/// Lowercased, brackets and a lone `:port` removed.
fn normalize_host(host: &str) -> String {
    let host = host.trim().to_lowercase();
    if let Some(inner) = host.strip_prefix('[') {
        return match inner.find(']') {
            Some(end) => inner[..end].to_string(),
            None => host,
        };
    }
    if host.matches(':').count() == 1 {
        return host.split(':').next().unwrap_or_default().to_string();
    }
    host
}

fn is_loopback(host: &str) -> bool {
    let host = normalize_host(host);
    host == "localhost" || host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
}

struct Url {
    scheme: String,
    host: String,
    /// `None` where Python's `.port` would raise; `.hostname` still works.
    port: Option<Option<u16>>,
    has_tail: bool,
}

/// Python's `urlsplit` for `scheme://host[:port][tail]`.
fn split_url(url: &str) -> Url {
    let (scheme, rest) = url.split_once("://").unwrap_or(("", url));
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (netloc, tail) = rest.split_at(end);
    let netloc = netloc.rsplit('@').next().unwrap_or_default();
    let (host, port) = match netloc.strip_prefix('[') {
        Some(inner) => {
            let (host, after) = inner.split_once(']').unwrap_or((inner, ""));
            (host, after.strip_prefix(':'))
        }
        None => match netloc.split_once(':') {
            Some((h, p)) => (h, Some(p)),
            None => (netloc, None),
        },
    };
    let port = match port {
        None | Some("") => Some(None),
        Some(p) if p.bytes().all(|b| b.is_ascii_digit()) => p.parse::<u16>().ok().map(Some),
        Some(_) => None,
    };
    Url {
        scheme: scheme.to_lowercase(),
        host: host.to_lowercase(),
        port,
        has_tail: !tail.is_empty(),
    }
}

/// `scheme://host:port` with the default port filled in, else the input lowercased.
fn normalize_origin(origin: &str) -> String {
    let origin = origin.trim().trim_end_matches('/');
    let url = split_url(origin);
    let Some(port) = url.port else {
        return origin.to_lowercase();
    };
    if url.scheme.is_empty() || url.host.is_empty() || url.has_tail {
        return origin.to_lowercase();
    }
    let mut host = normalize_host(&url.host);
    if host.contains(':') {
        host = format!("[{host}]");
    }
    let port = port.or(match url.scheme.as_str() {
        "http" => Some(80),
        "https" => Some(443),
        _ => None,
    });
    match port {
        Some(port) => format!("{}://{host}:{port}", url.scheme),
        None => format!("{}://{host}", url.scheme),
    }
}

/// Python's `fnmatchcase`: `*`, `?`, `[seq]` and `[!seq]`.
fn fnmatch(text: &str, pattern: &str) -> bool {
    let text: Vec<char> = text.chars().collect();
    let pattern: Vec<char> = pattern.chars().collect();
    matches(&text, &pattern)
}

fn matches(text: &[char], pattern: &[char]) -> bool {
    let Some((&p, rest)) = pattern.split_first() else {
        return text.is_empty();
    };
    match p {
        '*' => (0..=text.len()).any(|i| matches(&text[i..], rest)),
        '?' => !text.is_empty() && matches(&text[1..], rest),
        '[' => match class(rest) {
            Some((set, negated, after)) => text
                .split_first()
                .is_some_and(|(&c, tail)| in_class(c, set) != negated && matches(tail, after)),
            None => text.first() == Some(&'[') && matches(&text[1..], rest),
        },
        _ => text.first() == Some(&p) && matches(&text[1..], rest),
    }
}

/// The class body after `[`, whether it is negated, and the pattern after `]`.
fn class(pattern: &[char]) -> Option<(&[char], bool, &[char])> {
    let (negated, body) = match pattern.first() {
        Some('!') => (true, &pattern[1..]),
        _ => (false, pattern),
    };
    // A `]` first in the class is literal.
    let close = body.iter().skip(1).position(|&c| c == ']')? + 1;
    Some((&body[..close], negated, &body[close + 1..]))
}

fn in_class(c: char, set: &[char]) -> bool {
    let mut i = 0;
    while i < set.len() {
        if i + 2 < set.len() && set[i + 1] == '-' {
            if set[i] <= c && c <= set[i + 2] {
                return true;
            }
            i += 3;
        } else {
            if set[i] == c {
                return true;
            }
            i += 1;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnmatch_like_python() {
        assert!(fnmatch("box.tail.ts.net", "*.ts.net"));
        assert!(!fnmatch("ts.net", "*.ts.net"));
        assert!(fnmatch("a1", "a?"));
        assert!(!fnmatch("a", "a?"));
        assert!(fnmatch("b", "[abc]"));
        assert!(!fnmatch("b", "[!abc]"));
        assert!(fnmatch("q", "[a-z]"));
        assert!(fnmatch("]", "[]]"));
        assert!(fnmatch("[x", "[x"));
        assert!(!fnmatch("Box", "box"));
    }

    #[test]
    fn hosts_normalize_like_fastmcp() {
        assert_eq!(normalize_host("LocalHost:9000"), "localhost");
        assert_eq!(normalize_host("[::1]:8502"), "::1");
        assert_eq!(normalize_host("::1"), "::1");
        assert_eq!(normalize_host("mcp.ali.fyi:443"), "mcp.ali.fyi");
    }

    #[test]
    fn origins_normalize_like_fastmcp() {
        assert_eq!(
            normalize_origin("HTTPS://Mcp.Ali.Fyi/"),
            "https://mcp.ali.fyi:443"
        );
        assert_eq!(normalize_origin("http://[::1]:9999"), "http://[::1]:9999");
        assert_eq!(normalize_origin("http://x:99999"), "http://x:99999");
        assert_eq!(normalize_origin("http://x/path"), "http://x/path");
        assert_eq!(normalize_origin("null"), "null");
        assert_eq!(normalize_origin("http://x:+80"), "http://x:+80");
    }

    #[test]
    fn origin_falls_back_to_loopback_and_same_origin() {
        let guard = Guard::new(vec!["mcp.ali.fyi".into()], vec![]);
        assert!(guard.origin_allowed("http://localhost:9999", "http", "127.0.0.1:8502"));
        assert!(!guard.origin_allowed("https://mcp.ali.fyi:80", "http", "mcp.ali.fyi:80"));
        assert!(guard.origin_allowed("http://mcp.ali.fyi", "http", "mcp.ali.fyi"));
        assert!(!guard.origin_allowed("http://localhost:9999", "http", "mcp.ali.fyi"));
        assert!(!guard.origin_allowed("https://evil.example", "http", "127.0.0.1:8502"));
        // An invalid port spoils the origin's port, not its hostname.
        assert!(guard.origin_allowed("http://localhost:1:2", "http", "127.0.0.1:8502"));
        assert!(guard.origin_allowed("http://localhost:99999", "http", "127.0.0.1:8502"));
    }

    #[test]
    fn same_origin_uses_the_request_scheme() {
        let guard = Guard::new(vec!["mcp.ali.fyi".into()], vec![]);
        assert!(guard.origin_allowed("https://mcp.ali.fyi", "https", "mcp.ali.fyi"));
        assert!(!guard.origin_allowed("https://mcp.ali.fyi", "http", "mcp.ali.fyi"));
        assert!(!guard.origin_allowed("http://mcp.ali.fyi", "https", "mcp.ali.fyi"));
    }

    #[test]
    fn forwarded_proto_only_from_a_loopback_peer() {
        let local: Option<IpAddr> = Some("127.0.0.1".parse().unwrap());
        let v6: Option<IpAddr> = Some("::1".parse().unwrap());
        let remote: Option<IpAddr> = Some("192.168.1.10".parse().unwrap());
        assert_eq!(request_scheme(local, Some("https")), "https");
        assert_eq!(request_scheme(local, Some(" wss ")), "wss");
        assert_eq!(request_scheme(v6, Some("https")), "https");
        assert_eq!(request_scheme(remote, Some("https")), "http");
        assert_eq!(request_scheme(None, Some("https")), "http");
        assert_eq!(request_scheme(local, None), "http");
        // uvicorn matches the whole value, case-sensitively.
        assert_eq!(request_scheme(local, Some("HTTPS")), "http");
        assert_eq!(request_scheme(local, Some("https, http")), "http");
    }

    #[test]
    fn unbracketed_netloc_splits_at_the_first_colon() {
        // Splitting at the last colon would read host `a:1`, port 2.
        assert_eq!(normalize_origin("http://a:1:2"), "http://a:1:2");
        assert_eq!(split_url("http://a:1:2").host, "a");
    }
}
