//! Host/Origin checks with FastMCP's semantics (`fastmcp/server/http.py`,
//! `HostOriginGuardMiddleware` in strict mode), which the env allow-lists were
//! written for: ports are ignored on hosts, entries are `fnmatch` globs, and a
//! loopback Origin is accepted whenever the Host is loopback. rmcp's own check
//! matches exactly, so it is disabled in favor of this one.

use std::net::IpAddr;
use std::sync::Arc;

use axum::extract::Request;
use axum::http::{StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

const DEFAULT_HOSTS: &[&str] = &["127.0.0.1", "localhost", "::1"];

pub struct Guard {
    hosts: Vec<String>,
    origins: Vec<String>,
}

impl Guard {
    pub fn new(extra_hosts: Vec<String>, origins: Vec<String>) -> Arc<Self> {
        let mut hosts: Vec<String> = DEFAULT_HOSTS.iter().map(|h| h.to_string()).collect();
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
        if let Some(origin) = req.headers().get(header::ORIGIN) {
            let origin = String::from_utf8_lossy(origin.as_bytes());
            if !self.origin_allowed(&origin, &host) {
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

    fn origin_allowed(&self, origin: &str, host: &str) -> bool {
        let normalized = normalize_origin(origin);
        if self.origins.iter().any(|allowed| {
            let pattern = normalize_origin(allowed);
            pattern == "*" || fnmatch(&normalized, &pattern)
        }) {
            return true;
        }
        let origin_host = split_url(origin.trim()).map(|u| u.host).unwrap_or_default();
        if is_loopback(&origin_host) && is_loopback(host) {
            return true;
        }
        normalized == normalize_origin(&format!("http://{host}"))
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
    port: Option<u16>,
    has_tail: bool,
}

/// Python's `urlsplit` for `scheme://host[:port][tail]`; `None` where its
/// `.port` would raise.
fn split_url(url: &str) -> Option<Url> {
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
        None | Some("") => None,
        Some(p) if p.bytes().all(|b| b.is_ascii_digit()) => Some(p.parse::<u16>().ok()?),
        Some(_) => return None,
    };
    Some(Url {
        scheme: scheme.to_lowercase(),
        host: host.to_lowercase(),
        port,
        has_tail: !tail.is_empty(),
    })
}

/// `scheme://host:port` with the default port filled in, else the input lowercased.
fn normalize_origin(origin: &str) -> String {
    let origin = origin.trim().trim_end_matches('/');
    let Some(url) = split_url(origin) else {
        return origin.to_lowercase();
    };
    if url.scheme.is_empty() || url.host.is_empty() || url.has_tail {
        return origin.to_lowercase();
    }
    let mut host = normalize_host(&url.host);
    if host.contains(':') {
        host = format!("[{host}]");
    }
    let port = url.port.or(match url.scheme.as_str() {
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
        assert!(guard.origin_allowed("http://localhost:9999", "127.0.0.1:8502"));
        assert!(!guard.origin_allowed("https://mcp.ali.fyi:80", "mcp.ali.fyi:80"));
        assert!(guard.origin_allowed("http://mcp.ali.fyi", "mcp.ali.fyi"));
        assert!(!guard.origin_allowed("http://localhost:9999", "mcp.ali.fyi"));
        assert!(!guard.origin_allowed("https://evil.example", "127.0.0.1:8502"));
    }
}
