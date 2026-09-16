use crate::{config::Config, policy};
use anyhow::Result;
use parking_lot::{Mutex, RwLock};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{SystemTime, UNIX_EPOCH},
};
#[derive(Clone, serde::Serialize)]
pub struct LogEntry {
    pub timestamp: u64,
    pub level: String,
    pub source: String,
    pub method: String,
    pub target: String,
    pub params: String,
    pub outcome: String,
}
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::Semaphore,
    time::{timeout, Duration},
};
#[derive(Clone)]
pub struct ProxyManager {
    worker: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
    cfg: Arc<RwLock<Config>>,
    running: Arc<AtomicBool>,
    denied: Arc<RwLock<Vec<LogEntry>>>,
    traffic: Arc<RwLock<Vec<u64>>>,
}
impl ProxyManager {
    pub fn new(cfg: Arc<RwLock<Config>>) -> Self {
        Self {
            worker: Arc::new(Mutex::new(None)),
            cfg,
            running: Arc::new(AtomicBool::new(false)),
            denied: Arc::new(RwLock::new(Vec::new())),
            traffic: Arc::new(RwLock::new(Vec::new())),
        }
    }
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }
    pub fn start(&self) -> Result<()> {
        let mut worker = self.worker.lock();
        if self.is_running() {
            return Ok(());
        }
        if let Some(old) = worker.take() {
            let _ = old.join();
        }
        let c = self.cfg.read().clone();
        // Transfer the actual bound sockets to Tokio; never release and rebind them.
        let http = std::net::TcpListener::bind((c.http_host.as_str(), c.http_port))?;
        let socks = std::net::TcpListener::bind((c.socks_host.as_str(), c.socks_port))?;
        http.set_nonblocking(true)?;
        socks.set_nonblocking(true)?;
        let rt = tokio::runtime::Runtime::new()?;
        let (http, socks) = {
            let _guard = rt.enter();
            (TcpListener::from_std(http)?, TcpListener::from_std(socks)?)
        };
        let run = self.running.clone();
        let cfg = self.cfg.clone();
        let logs = self.denied.clone();
        let traffic = self.traffic.clone();
        run.store(true, Ordering::SeqCst);
        match thread::Builder::new()
            .name("domain-egress-proxy".into())
            .spawn(move || {
                rt.block_on(async move {
                    let sem = Arc::new(Semaphore::new(100));
                    while run.load(Ordering::SeqCst) {
                        tokio::select! {
                            Ok((stream, _)) = http.accept() => {
                                if let Ok(permit) = sem.clone().try_acquire_owned() {
                                    let (c, l, t) = (cfg.clone(), logs.clone(), traffic.clone());
                                    tokio::spawn(async move {
                                        let _permit = permit;
                                        let _ = handle_http(stream, c, l, t).await;
                                    });
                                }
                            }
                            Ok((stream, _)) = socks.accept() => {
                                if let Ok(permit) = sem.clone().try_acquire_owned() {
                                    let (c, l, t) = (cfg.clone(), logs.clone(), traffic.clone());
                                    tokio::spawn(async move {
                                        let _permit = permit;
                                        let _ = handle_socks(stream, c, l, t).await;
                                    });
                                }
                            }
                            _ = tokio::time::sleep(std::time::Duration::from_millis(50)) => {}
                        }
                    }
                });
            }) {
            Ok(handle) => {
                *worker = Some(handle);
                Ok(())
            }
            Err(error) => {
                self.running.store(false, Ordering::SeqCst);
                Err(error.into())
            }
        }
    }
    pub fn stop(&self) -> Result<()> {
        let mut worker = self.worker.lock();
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = worker.take() {
            handle
                .join()
                .map_err(|_| anyhow::anyhow!("代理线程异常退出"))?;
        }
        Ok(())
    }
    pub fn logs(&self) -> Vec<LogEntry> {
        self.denied.read().clone()
    }
    pub fn clear_logs(&self) {
        self.denied.write().clear();
    }
    pub fn traffic(&self) -> Vec<u64> {
        self.traffic.read().clone()
    }
}
async fn ok(c: &Arc<RwLock<Config>>, h: &str) -> bool {
    let x = c.read();
    let r = if x.access_mode == "blacklist" {
        &x.blacklist
    } else {
        &x.whitelist
    };
    policy::allowed(&x.access_mode, r, h)
}
fn level_allowed(configured: &str, level: &str) -> bool {
    let rank = |x| match x {
        "error" => 0,
        "warn" => 1,
        "info" => 2,
        "debug" => 3,
        _ => 2,
    };
    rank(level) <= rank(configured)
}
fn record(
    logs: &Arc<RwLock<Vec<LogEntry>>>,
    traffic: &Arc<RwLock<Vec<u64>>>,
    cfg: &Arc<RwLock<Config>>,
    entry: LogEntry,
) {
    if entry.outcome == "放行" {
        let mut t = traffic.write();
        t.push(entry.timestamp);
        let cutoff = now().saturating_sub(cfg.read().trend_retention_days.clamp(1, 21) as u64 * 86_400);
        t.retain(|x| *x >= cutoff);
    }
    let cfg_snapshot = cfg.read().clone();
    if !level_allowed(&cfg_snapshot.log_level, &entry.level) {
        return;
    }
    let mut logs = logs.write();
    logs.push(entry);
    let cutoff = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| {
            d.as_secs()
                .saturating_sub(cfg_snapshot.log_retention_days.max(1) as u64 * 86_400)
        })
        .unwrap_or_default();
    logs.retain(|x| x.timestamp >= cutoff);
    if logs.len() > 2000 {
        let excess = logs.len() - 2000;
        logs.drain(..excess);
    }
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}
async fn pipe(mut a: TcpStream, mut b: TcpStream) -> Result<()> {
    tokio::io::copy_bidirectional(&mut a, &mut b).await?;
    Ok(())
}
async fn connect_target(cfg: &Arc<RwLock<Config>>, host: &str, port: u16) -> Result<TcpStream> {
    timeout(Duration::from_secs(15), async {
    let ssh_port = cfg.read().ssh_proxy_port;
    if let Some(proxy_port) = ssh_port {
        let mut stream = TcpStream::connect(("127.0.0.1", proxy_port)).await?;
        stream.write_all(&[5, 1, 0]).await?;
        let mut greeting = [0u8; 2]; stream.read_exact(&mut greeting).await?;
        anyhow::ensure!(greeting == [5, 0], "SSH SOCKS 出口认证失败");
        let bytes = host.as_bytes(); anyhow::ensure!(bytes.len() <= 255, "目标域名过长");
        let mut request = vec![5, 1, 0, 3, bytes.len() as u8]; request.extend_from_slice(bytes); request.extend_from_slice(&port.to_be_bytes());
        stream.write_all(&request).await?;
        let mut response = [0u8; 4]; stream.read_exact(&mut response).await?;
        anyhow::ensure!(response[1] == 0, "SSH 出口连接目标失败，错误码 {}", response[1]);
        let skip = match response[3] { 1 => 4, 3 => { let mut len = [0u8; 1]; stream.read_exact(&mut len).await?; len[0] as usize }, 4 => 16, _ => 0 };
        let mut tail = vec![0u8; skip + 2]; stream.read_exact(&mut tail).await?;
        Ok(stream)
    } else {
        Ok(TcpStream::connect((host, port)).await?)
    }
    }).await.map_err(|_| anyhow::anyhow!("连接上游目标超时（15 秒）"))?
}
fn target_address(authority: &str, default_port: u16) -> Result<(&str, u16)> {
    if let Some(ipv6) = authority.strip_prefix('[') {
        let (host, suffix) = ipv6
            .split_once(']')
            .ok_or_else(|| anyhow::anyhow!("无效 IPv6 地址"))?;
        host.parse::<std::net::Ipv6Addr>()?;
        let port = if suffix.is_empty() {
            default_port
        } else {
            suffix
                .strip_prefix(':')
                .ok_or_else(|| anyhow::anyhow!("无效端口"))?
                .parse()?
        };
        anyhow::ensure!(port > 0, "无效端口");
        return Ok((host, port));
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => (host, port.parse()?),
        None => (authority, default_port),
    };
    anyhow::ensure!(
        !host.is_empty() && !host.contains(':') && port > 0,
        "无效目标地址"
    );
    Ok((host, port))
}
async fn handle_http(
    mut c: TcpStream,
    cfg: Arc<RwLock<Config>>,
    denied: Arc<RwLock<Vec<LogEntry>>>,
    traffic: Arc<RwLock<Vec<u64>>>,
) -> Result<()> {
    let mut b = vec![0; 8192];
    let n = c.read(&mut b).await?;
    let q = String::from_utf8_lossy(&b[..n]);
    let f = q.lines().next().unwrap_or("");
    let h = if f.starts_with("CONNECT ") {
        f.split_whitespace().nth(1).unwrap_or("")
    } else {
        q.lines()
            .find_map(|l| {
                l.split_once(':')
                    .filter(|(name, _)| name.eq_ignore_ascii_case("host"))
                    .map(|(_, value)| value)
            })
            .map(str::trim)
            .unwrap_or("")
    };
    let (host, port) = target_address(h, if f.starts_with("CONNECT ") { 443 } else { 80 })?;
    let source = c
        .peer_addr()
        .map(|x| x.to_string())
        .unwrap_or_else(|_| "unknown".into());
    let method = f.split_whitespace().next().unwrap_or("UNKNOWN").to_string();
    let request_target = f.split_whitespace().nth(1).unwrap_or(h);
    let params = request_target
        .split_once('?')
        .map(|x| x.1)
        .unwrap_or("")
        .to_string();
    if !ok(&cfg, host).await {
        record(
            &denied,
            &traffic,
            &cfg,
            LogEntry {
                timestamp: now(),
                level: "warn".into(),
                source: source.clone(),
                method: method.clone(),
                target: host.into(),
                params: params.clone(),
                outcome: "拦截".into(),
            },
        );
        if f.starts_with("CONNECT ") {
            c.write_all(b"HTTP/1.1 200 Connection Established\r\nConnection: close\r\n\r\n").await?;
        } else {
            c.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await?;
        }
        return Ok(());
    }
    record(
        &denied,
        &traffic,
        &cfg,
        LogEntry {
            timestamp: now(),
            level: "info".into(),
            source,
            method,
            target: host.into(),
            params,
            outcome: "放行".into(),
        },
    );
    let mut d = match connect_target(&cfg, host, port).await {
        Ok(stream) => stream,
        Err(_) => {
            c.write_all(b"HTTP/1.1 502 Bad Gateway\r\nConnection: close\r\n\r\n").await?;
            return Ok(());
        }
    };
    if f.starts_with("CONNECT ") {
        c.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
            .await?
    } else {
        d.write_all(&b[..n]).await?
    }
    pipe(c, d).await
}
async fn handle_socks(
    mut c: TcpStream,
    cfg: Arc<RwLock<Config>>,
    denied: Arc<RwLock<Vec<LogEntry>>>,
    traffic: Arc<RwLock<Vec<u64>>>,
) -> Result<()> {
    let mut h = [0; 2];
    c.read_exact(&mut h).await?;
    let mut m = vec![0; h[1] as usize];
    c.read_exact(&mut m).await?;
    c.write_all(&[5, 0]).await?;
    let mut q = [0; 4];
    c.read_exact(&mut q).await?;
    if q[1] != 1 {
        return Ok(());
    }
    let host = match q[3] {
        1 => {
            let mut x = [0; 4];
            c.read_exact(&mut x).await?;
            std::net::Ipv4Addr::from(x).to_string()
        }
        3 => {
            let mut l = [0; 1];
            c.read_exact(&mut l).await?;
            let mut x = vec![0; l[0] as usize];
            c.read_exact(&mut x).await?;
            String::from_utf8_lossy(&x).into_owned()
        }
        4 => {
            let mut x = [0; 16];
            c.read_exact(&mut x).await?;
            std::net::Ipv6Addr::from(x).to_string()
        }
        _ => return Ok(()),
    };
    let mut p = [0; 2];
    c.read_exact(&mut p).await?;
    let port = u16::from_be_bytes(p);
    let source = c
        .peer_addr()
        .map(|x| x.to_string())
        .unwrap_or_else(|_| "unknown".into());
    if !ok(&cfg, &host).await {
        record(
            &denied,
            &traffic,
            &cfg,
            LogEntry {
                timestamp: now(),
                level: "warn".into(),
                source: source.clone(),
                method: "CONNECT".into(),
                target: host.clone(),
                params: format!("port={port}"),
                outcome: "拦截".into(),
            },
        );
        c.write_all(&[5, 0, 0, 1, 0, 0, 0, 0, 0, 0]).await?;
        return Ok(());
    }
    record(
        &denied,
        &traffic,
        &cfg,
        LogEntry {
            timestamp: now(),
            level: "info".into(),
            source,
            method: "CONNECT".into(),
            target: host.clone(),
            params: format!("port={port}"),
            outcome: "放行".into(),
        },
    );
    let d = match connect_target(&cfg, host.as_str(), port).await {
        Ok(stream) => stream,
        Err(_) => {
            c.write_all(&[5, 5, 0, 1, 0, 0, 0, 0, 0, 0]).await?;
            return Ok(());
        }
    };
    c.write_all(&[5, 0, 0, 1, 0, 0, 0, 0, 0, 0]).await?;
    pipe(c, d).await
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    #[test]
    fn local_http_connect_and_socks_policy() {
        use std::io::{Read, Write};
        let config = Config {
            http_port: free_port(),
            socks_port: free_port(),
            whitelist: vec!["127.0.0.1".into()],
            ..Config::default()
        };
        let manager = ProxyManager::new(Arc::new(RwLock::new(config.clone())));
        manager.start().unwrap();
        let connect = |port| {
            let stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(3)))
                .unwrap();
            stream
        };
        // Requests rejected by policy never need an upstream connection.
        let mut denied = connect(config.http_port);
        denied
            .write_all(
                b"CONNECT forbidden.invalid:443 HTTP/1.1\r\nHost: forbidden.invalid:443\r\n\r\n",
            )
            .unwrap();
        let mut response = [0u8; 256];
        let n = denied.read(&mut response).unwrap();
        assert!(String::from_utf8_lossy(&response[..n]).contains("200 Connection Established"));
        // A real local echo server exercises CONNECT and SOCKS TCP forwarding.
        let upstream = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let upstream_port = upstream.local_addr().unwrap().port();
        let echo = thread::spawn(move || {
            for _ in 0..2 {
                let (mut stream, _) = upstream.accept().unwrap();
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(3)))
                    .unwrap();
                let mut bytes = [0; 4];
                stream.read_exact(&mut bytes).unwrap();
                stream.write_all(&bytes).unwrap();
            }
        });
        let mut http = connect(config.http_port);
        http.write_all(format!("CONNECT 127.0.0.1:{upstream_port} HTTP/1.1\r\nHost: 127.0.0.1:{upstream_port}\r\n\r\n").as_bytes()).unwrap();
        let n = http.read(&mut response).unwrap();
        assert!(String::from_utf8_lossy(&response[..n]).contains("200 Connection Established"));
        http.write_all(b"ping").unwrap();
        let mut pong = [0; 4];
        http.read_exact(&mut pong).unwrap();
        assert_eq!(&pong, b"ping");
        let mut socks = connect(config.socks_port);
        socks.write_all(&[5, 1, 0]).unwrap();
        let mut hello = [0; 2];
        socks.read_exact(&mut hello).unwrap();
        assert_eq!(hello, [5, 0]);
        let mut request = vec![5, 1, 0, 1, 127, 0, 0, 1];
        request.extend(upstream_port.to_be_bytes());
        socks.write_all(&request).unwrap();
        let mut reply = [0; 10];
        socks.read_exact(&mut reply).unwrap();
        assert_eq!(reply[1], 0);
        socks.write_all(b"pong").unwrap();
        socks.read_exact(&mut pong).unwrap();
        assert_eq!(&pong, b"pong");
        echo.join().unwrap();
        assert_eq!(manager.logs().len(), 3);
        assert_eq!(manager.traffic().len(), 2);
        manager.stop().unwrap();
    }
    #[test]
    fn destination_defaults_and_ipv6() {
        assert_eq!(
            target_address("example.com", 80).unwrap(),
            ("example.com", 80)
        );
        assert_eq!(
            target_address("example.com:443", 80).unwrap(),
            ("example.com", 443)
        );
        assert_eq!(target_address("[::1]:8080", 80).unwrap(), ("::1", 8080));
        assert!(target_address("host:0", 80).is_err());
    }
    fn free_port() -> u16 {
        std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }
    #[test]
    fn restart_releases_both_listeners() {
        let config = Config {
            http_port: free_port(),
            socks_port: free_port(),
            ..Config::default()
        };
        let manager = ProxyManager::new(Arc::new(RwLock::new(config.clone())));
        for _ in 0..3 {
            manager.start().unwrap();
            assert!(manager.is_running());
            assert!(std::net::TcpListener::bind(("127.0.0.1", config.http_port)).is_err());
            assert!(std::net::TcpListener::bind(("127.0.0.1", config.socks_port)).is_err());
            manager.stop().unwrap();
            assert!(!manager.is_running());
            assert!(std::net::TcpListener::bind(("127.0.0.1", config.http_port)).is_ok());
            assert!(std::net::TcpListener::bind(("127.0.0.1", config.socks_port)).is_ok());
        }
    }
    #[test]
    fn conflict_does_not_start_or_leak_listener() {
        let occupied = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let config = Config {
            http_port: free_port(),
            socks_port: occupied.local_addr().unwrap().port(),
            ..Config::default()
        };
        let manager = ProxyManager::new(Arc::new(RwLock::new(config.clone())));
        assert!(manager.start().is_err());
        assert!(!manager.is_running());
        assert!(std::net::TcpListener::bind(("127.0.0.1", config.http_port)).is_ok());
    }
}
