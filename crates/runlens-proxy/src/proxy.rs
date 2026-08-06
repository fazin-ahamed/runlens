use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Notify;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProxyMode {
    Record,
    Replay,
    Mixed,
}

#[derive(Debug, Clone)]
pub struct ProxyConfig {
    pub port: u16,
    pub mode: ProxyMode,
    pub bind_addr: String,
    pub tls: bool,
    pub session_id: Option<String>,
}

pub struct Proxy {
    config: ProxyConfig,
}

/// Small HTTP forward proxy.
///
/// Accepts both `CONNECT host:port` tunnels and absolute-form requests
/// (`GET http://host/path HTTP/1.1`). It establishes the upstream
/// connection, returns the tunnel to the client, and relays bytes in
/// both directions. Response content is counted so the caller can keep
/// a rough traffic total.
pub struct TrafficStats {
    pub bytes_upstream: u64,
    pub bytes_downstream: u64,
}

impl Proxy {
    pub fn new(config: ProxyConfig, _session_id: Option<String>) -> Self {
        Self { config }
    }

    pub async fn serve(&self, shutdown: Arc<Notify>) -> anyhow::Result<()> {
        let listener = TcpListener::bind(format!("{}:{}", self.config.bind_addr, self.config.port))
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "failed to bind proxy on {}:{}: {e}",
                    self.config.bind_addr,
                    self.config.port
                )
            })?;

        let local_addr = listener
            .local_addr()
            .map_err(|e| anyhow::anyhow!("failed to read listener address: {e}"))?;
        println!("runlens proxy listening on {} ({:?})", local_addr, self.config.mode);

        loop {
            let (mut client, _) = match listener.accept().await {
                Ok(accepted) => accepted,
                Err(e) => {
                    tracing::debug!("accept failed: {e}");
                    continue;
                },
            };

            let shutdown_signal = shutdown.clone();
            tokio::spawn(async move {
                let _ = handle_connection(&mut client, shutdown_signal).await;
            });
        }
    }
}

async fn handle_connection(client: &mut TcpStream, shutdown_signal: Arc<Notify>) -> anyhow::Result<TrafficStats> {
    let mut head = Vec::with_capacity(4096);
    let mut buf = [0u8; 4096];
    let header_end;

    loop {
        let n = client
            .read(&mut buf)
            .await
            .map_err(|e| anyhow::anyhow!("read head: {e}"))?;
        if n == 0 {
            anyhow::bail!("client closed before sending a request");
        }
        head.extend_from_slice(&buf[..n]);
        if let Some(idx) = find_header_end(&head) {
            header_end = idx;
            break;
        }
        if head.len() > 64 * 1024 {
            anyhow::bail!("request head too large");
        }
    }

    let first_line = head
        .split(|&b| b == b'\n')
        .next()
        .map(|l| String::from_utf8_lossy(l).trim().to_string())
        .unwrap_or_default();

    let (target_host, target_port, connect_mode) = parse_request_line(&first_line)?;

    let _ = shutdown_signal;

    let mut upstream = TcpStream::connect((target_host.as_str(), target_port))
        .await
        .map_err(|e| anyhow::anyhow!("connect to {target_host}:{target_port}: {e}"))?;

    if connect_mode {
        client
            .write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
            .await
            .map_err(|e| anyhow::anyhow!("write 200: {e}"))?;
    } else {
        // Absolute-form request: shunt the request head upstream and
        // relay any leftover bytes (request body) too.
        let rewritten = rewrite_origin_form(&head[..header_end], &target_host, target_port)
            .unwrap_or_else(|| head[..header_end].to_vec());
        upstream
            .write_all(&rewritten)
            .await
            .map_err(|e| anyhow::anyhow!("forward request: {e}"))?;
        if header_end < head.len() {
            upstream
                .write_all(&head[header_end..])
                .await
                .map_err(|e| anyhow::anyhow!("forward request body: {e}"))?;
        }
    }

    let mut stats = relay(client, &mut upstream).await?;
    let _ = &mut stats;
    Ok(stats)
}

async fn relay(client: &mut TcpStream, upstream: &mut TcpStream) -> anyhow::Result<TrafficStats> {
    let mut up_bytes: u64 = 0;
    let mut down_bytes: u64 = 0;
    let mut left = [0u8; 64 * 1024];
    let mut right = [0u8; 64 * 1024];
    loop {
        tokio::select! {
            n = client.read(&mut left) => {
                let n = n.map_err(|e| anyhow::anyhow!("read client: {e}"))?;
                if n == 0 { break; }
                upstream.write_all(&left[..n]).await.map_err(|e| anyhow::anyhow!("write upstream: {e}"))?;
                up_bytes += n as u64;
            }
            n = upstream.read(&mut right) => {
                let n = n.map_err(|e| anyhow::anyhow!("read upstream: {e}"))?;
                if n == 0 { break; }
                client.write_all(&right[..n]).await.map_err(|e| anyhow::anyhow!("write client: {e}"))?;
                down_bytes += n as u64;
            }
        }
    }
    Ok(TrafficStats {
        bytes_upstream: up_bytes,
        bytes_downstream: down_bytes,
    })
}

/// Parse a request line like `CONNECT example.com:443 HTTP/1.1` or
/// `GET http://example.com/path HTTP/1.1`. Returns (host, port, is_connect).
fn parse_request_line(line: &str) -> anyhow::Result<(String, u16, bool)> {
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_ascii_uppercase();
    let target = parts.next().unwrap_or_default();

    if method.is_empty() || target.is_empty() {
        anyhow::bail!("unparseable request line: {line:?}");
    }

    let is_connect = method == "CONNECT";

    if is_connect {
        let (host, port) = split_authority(target);
        return Ok((host, port.unwrap_or(443), true));
    }

    let target = strip_url_scheme(target);
    let (host, port) = split_authority(target);
    Ok((host, port.unwrap_or(80), false))
}

fn split_authority(authority: &str) -> (String, Option<u16>) {
    let authority = authority.trim_end_matches('/');
    let authority = authority.split(['/', '?', '#']).next().unwrap_or(authority);
    if let Some((host, port)) = authority.rsplit_once(':') {
        if host.ends_with(']') || port.parse::<u16>().is_ok() {
            let port = port.parse::<u16>().ok();
            return (host.trim_start_matches('[').trim_end_matches(']').to_string(), port);
        }
    }
    (authority.to_string(), None)
}

fn strip_url_scheme(target: &str) -> &str {
    if let Some(rest) = target.strip_prefix("http://") {
        rest
    } else if let Some(rest) = target.strip_prefix("https://") {
        rest
    } else {
        target
    }
}

/// Rewrite an absolute-form request into origin-form so the upstream
/// sees `GET /path HTTP/1.1` plus Host header.
fn rewrite_origin_form(head: &[u8], host: &str, port: u16) -> Option<Vec<u8>> {
    let text = String::from_utf8_lossy(head);
    let line_end = text.find("\r\n")?;
    let first_line = &text[..line_end];
    let mut parts = first_line.split_whitespace();
    let method = parts.next()?;
    let target = parts.next()?;
    let version = parts.next().unwrap_or("HTTP/1.1");

    let path = target
        .strip_prefix("http://")
        .or_else(|| target.strip_prefix("https://"))
        .and_then(|rest| rest.find('/').map(|idx| &rest[idx..]))
        .unwrap_or("/");

    let mut out = format!("{method} {path} {version}\r\nHost: {host}:{port}\r\n").into_bytes();
    let rest = &text[line_end + 2..];
    out.extend_from_slice(rest.as_bytes());
    Some(out)
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n").map(|i| i + 4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_connect_line() {
        let (host, port, is_connect) = parse_request_line("CONNECT example.com:443 HTTP/1.1").unwrap();
        assert_eq!(host, "example.com");
        assert_eq!(port, 443);
        assert!(is_connect);
    }

    #[test]
    fn parses_absolute_form() {
        let (host, port, is_connect) = parse_request_line("GET http://example.com/path HTTP/1.1").unwrap();
        assert_eq!(host, "example.com");
        assert_eq!(port, 80);
        assert!(!is_connect);
    }

    #[test]
    fn parses_default_port() {
        let (host, port, _) = parse_request_line("GET http://example.com/ HTTP/1.1").unwrap();
        assert_eq!(host, "example.com");
        assert_eq!(port, 80);
    }

    #[test]
    fn finds_header_end() {
        assert_eq!(find_header_end(b"GET / HTTP/1.1\r\nHost: x\r\n\r\nbody"), Some(27));
        assert_eq!(find_header_end(b"no end here"), None);
    }

    #[test]
    fn rewrites_absolute_to_origin() {
        let head = b"GET http://example.com/a/b HTTP/1.1\r\nHost: old\r\n\r\n";
        let rewritten = rewrite_origin_form(head, "example.com", 8080).unwrap();
        let text = String::from_utf8(rewritten).unwrap();
        assert!(text.starts_with("GET /a/b HTTP/1.1\r\nHost: example.com:8080\r\n"));
    }

    #[tokio::test]
    async fn connects_and_relays_bytes() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        // Upstream echo server.
        let upstream = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let upstream_port = upstream.local_addr().unwrap().port();
        let upstream_task = tokio::spawn(async move {
            let (mut socket, _) = upstream.accept().await.unwrap();
            let mut buf = [0u8; 1024];
            loop {
                let n = socket.read(&mut buf).await.unwrap();
                if n == 0 {
                    break;
                }
                socket.write_all(&buf[..n]).await.unwrap();
            }
        });

        // Our proxy in front of it.
        let proxy_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy_port = proxy_listener.local_addr().unwrap().port();
        let proxy = Proxy::new(
            ProxyConfig {
                port: proxy_port,
                mode: ProxyMode::Mixed,
                bind_addr: "127.0.0.1".into(),
                tls: false,
                session_id: None,
            },
            None,
        );
        // serve() binds its own listener on the same port, which we want to reuse.
        drop(proxy_listener);

        let shutdown = Arc::new(Notify::new());
        let dir = tokio::spawn(async move { proxy.serve(shutdown.clone()).await.unwrap() });

        // Give the proxy a moment to bind.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let mut client = TcpStream::connect(("127.0.0.1", proxy_port)).await.unwrap();
        client
            .write_all(format!("CONNECT 127.0.0.1:{upstream_port} HTTP/1.1\r\n\r\n").as_bytes())
            .await
            .unwrap();
        let mut resp = [0u8; 64];
        let n = client.read(&mut resp).await.unwrap();
        assert!(String::from_utf8_lossy(&resp[..n]).starts_with("HTTP/1.1 200"));

        client.write_all(b"ping").await.unwrap();
        let mut echo = [0u8; 4];
        let n = client.read(&mut echo).await.unwrap();
        assert_eq!(&echo[..n], b"ping");

        dir.abort();
        drop(upstream_task);
    }
}
