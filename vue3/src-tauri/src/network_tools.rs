use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    net::{TcpStream, ToSocketAddrs, UdpSocket},
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tungstenite::{connect, http::Request, Message};
use url::Url;

#[derive(Clone, Deserialize)]
pub struct NetworkProbeRequest {
    pub tool: String,
    pub target: String,
    pub port: Option<u16>,
    pub timeout_ms: u64,
    pub count: u16,
    pub method: Option<String>,
    pub headers: Option<String>,
    pub body: Option<String>,
    pub payload: Option<String>,
    pub payload_hex: bool,
}

#[derive(Clone, Serialize)]
pub struct NetworkProbeResult {
    pub tool: String,
    pub target: String,
    pub success: bool,
    pub elapsed_ms: u128,
    pub summary: String,
    pub error: Option<String>,
    pub details: serde_json::Value,
}

fn result(
    req: &NetworkProbeRequest,
    started: Instant,
    success: bool,
    summary: String,
    error: Option<String>,
    details: serde_json::Value,
) -> NetworkProbeResult {
    NetworkProbeResult {
        tool: req.tool.clone(),
        target: req.target.clone(),
        success,
        elapsed_ms: started.elapsed().as_millis(),
        summary,
        error,
        details,
    }
}
fn timeout(req: &NetworkProbeRequest) -> Duration {
    Duration::from_millis(req.timeout_ms.clamp(100, 120_000))
}
fn port(req: &NetworkProbeRequest, default: u16) -> Result<u16, String> {
    Ok(req.port.unwrap_or(default).max(1))
}
fn endpoint(req: &NetworkProbeRequest, default: u16) -> Result<String, String> {
    let target = req.target.trim();
    if target.is_empty() {
        return Err("目标不能为空".into());
    }
    let target = target.trim_matches(['[', ']']);
    let target = target
        .parse::<std::net::IpAddr>()
        .map(|ip| {
            if ip.is_ipv6() {
                format!("[{ip}]")
            } else {
                ip.to_string()
            }
        })
        .unwrap_or_else(|_| target.to_string());
    Ok(format!("{}:{}", target, port(req, default)?))
}
fn cancelled(cancel: &AtomicBool) -> Result<(), String> {
    if cancel.load(Ordering::Relaxed) {
        Err("探测已取消".into())
    } else {
        Ok(())
    }
}

fn parse_headers(input: &str) -> Result<Vec<(String, String)>, String> {
    input
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let (key, value) = line
                .split_once(':')
                .ok_or_else(|| format!("请求头格式无效：{line}"))?;
            if key.trim().is_empty() {
                return Err("请求头名称不能为空".into());
            }
            Ok((key.trim().into(), value.trim().into()))
        })
        .collect()
}
fn decode_payload(req: &NetworkProbeRequest) -> Result<Vec<u8>, String> {
    let payload = req.payload.clone().unwrap_or_default();
    if req.payload_hex {
        hex::decode(payload.replace([' ', ':'], "")).map_err(|e| format!("十六进制数据无效：{e}"))
    } else {
        Ok(payload.into_bytes())
    }
}

fn command_probe(
    req: &NetworkProbeRequest,
    command: &str,
    args: &[String],
    started: Instant,
    cancel: &AtomicBool,
) -> NetworkProbeResult {
    let mut child = match Command::new(command)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            return result(
                req,
                started,
                false,
                "无法启动系统网络工具".into(),
                Some(e.to_string()),
                serde_json::json!({}),
            )
        }
    };
    let deadline = Instant::now() + timeout(req) * req.count.max(1) as u32 + Duration::from_secs(2);
    loop {
        if cancel.load(Ordering::Relaxed) {
            let _ = child.kill();
            return result(
                req,
                started,
                false,
                "探测已取消".into(),
                Some("cancelled".into()),
                serde_json::json!({}),
            );
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                let (stdout, stderr) = child
                    .wait_with_output()
                    .map(|output| (output.stdout, output.stderr))
                    .unwrap_or_default();
                let text = String::from_utf8_lossy(&stdout).to_string();
                let err = String::from_utf8_lossy(&stderr).trim().to_string();
                return result(
                    req,
                    started,
                    status.success(),
                    if status.success() {
                        "探测完成".into()
                    } else {
                        "探测失败".into()
                    },
                    (!status.success()).then_some(if err.is_empty() { text.clone() } else { err }),
                    serde_json::json!({"output": text}),
                );
            }
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(25)),
            _ => {
                let _ = child.kill();
                return result(
                    req,
                    started,
                    false,
                    "探测超时".into(),
                    Some("timeout".into()),
                    serde_json::json!({}),
                );
            }
        }
    }
}
fn ping(req: &NetworkProbeRequest, started: Instant, cancel: &AtomicBool) -> NetworkProbeResult {
    let args = vec![
        "-n".into(),
        "-c".into(),
        req.count.clamp(1, 20).to_string(),
        "-W".into(),
        (timeout(req).as_millis() as u64).to_string(),
        req.target.trim().into(),
    ];
    command_probe(req, "/sbin/ping", &args, started, cancel)
}
fn traceroute(
    req: &NetworkProbeRequest,
    started: Instant,
    cancel: &AtomicBool,
) -> NetworkProbeResult {
    let args = vec![
        "-n".into(),
        "-m".into(),
        "30".into(),
        "-w".into(),
        timeout(req).as_secs().max(1).to_string(),
        req.target.trim().into(),
    ];
    command_probe(req, "/usr/sbin/traceroute", &args, started, cancel)
}

fn certificate(
    req: &NetworkProbeRequest,
    started: Instant,
    cancel: &AtomicBool,
) -> NetworkProbeResult {
    let endpoint = match endpoint(req, 443) {
        Ok(value) => value,
        Err(error) => {
            return result(
                req,
                started,
                false,
                "参数无效".into(),
                Some(error),
                serde_json::json!({}),
            )
        }
    };
    if let Err(error) = cancelled(cancel) {
        return result(
            req,
            started,
            false,
            error.clone(),
            Some(error),
            serde_json::json!({}),
        );
    }
    let output = match Command::new("/usr/bin/openssl")
        .args([
            "s_client",
            "-connect",
            &endpoint,
            "-servername",
            req.target.trim(),
            "-showcerts",
            "-brief",
        ])
        .stdin(std::process::Stdio::null())
        .output()
    {
        Ok(output) => output,
        Err(error) => {
            return result(
                req,
                started,
                false,
                "无法启动证书扫描".into(),
                Some(error.to_string()),
                serde_json::json!({}),
            )
        }
    };
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let begin = "-----BEGIN CERTIFICATE-----";
    let end = "-----END CERTIFICATE-----";
    let certificate_pem = combined.find(begin).and_then(|start| {
        combined[start..].find(end).map(|finish| {
            format!(
                "{}\n{}\n",
                &combined[start..start + finish + end.len()],
                end
            )
        })
    });
    let Some(pem) = certificate_pem else {
        return result(
            req,
            started,
            false,
            "TLS 握手失败或未返回证书".into(),
            Some(combined.trim().to_string()),
            serde_json::json!({"openssl": combined}),
        );
    };
    let details = match Command::new("/usr/bin/openssl")
        .args([
            "x509",
            "-noout",
            "-subject",
            "-issuer",
            "-dates",
            "-ext",
            "subjectAltName",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
    {
        Ok(mut child) => {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(pem.as_bytes());
            }
            match child.wait_with_output() {
                Ok(output) => String::from_utf8_lossy(&output.stdout).to_string(),
                Err(error) => format!("证书字段解析失败：{error}"),
            }
        }
        Err(error) => format!("证书字段解析失败：{error}"),
    };
    let success = output.status.success();
    result(
        req,
        started,
        success,
        if success {
            "证书扫描完成".into()
        } else {
            "证书扫描失败".into()
        },
        (!success).then_some(combined.clone()),
        serde_json::json!({"certificate": details, "openssl": combined}),
    )
}

fn tcp(req: &NetworkProbeRequest, started: Instant, cancel: &AtomicBool) -> NetworkProbeResult {
    let endpoint = match endpoint(req, 80) {
        Ok(v) => v,
        Err(e) => {
            return result(
                req,
                started,
                false,
                "参数无效".into(),
                Some(e),
                serde_json::json!({}),
            )
        }
    };
    let mut stream = match endpoint
        .to_socket_addrs()
        .ok()
        .and_then(|mut a| a.next())
        .and_then(|a| TcpStream::connect_timeout(&a, timeout(req)).ok())
    {
        Some(s) => s,
        None => {
            return result(
                req,
                started,
                false,
                "TCP 连接失败".into(),
                Some("connection_failed".into()),
                serde_json::json!({"endpoint": endpoint}),
            )
        }
    };
    if let Err(e) = cancelled(cancel) {
        return result(
            req,
            started,
            false,
            e.clone(),
            Some(e),
            serde_json::json!({}),
        );
    }
    let _ = stream.set_read_timeout(Some(timeout(req)));
    let payload = match decode_payload(req) {
        Ok(v) => v,
        Err(e) => {
            return result(
                req,
                started,
                false,
                "数据无效".into(),
                Some(e),
                serde_json::json!({}),
            )
        }
    };
    if !payload.is_empty() {
        if let Err(e) = stream.write_all(&payload) {
            return result(
                req,
                started,
                false,
                "TCP 发送失败".into(),
                Some(e.to_string()),
                serde_json::json!({}),
            );
        }
    }
    let mut buf = vec![0; 8192];
    let read = stream.read(&mut buf).unwrap_or(0);
    result(
        req,
        started,
        true,
        "TCP 连接成功".into(),
        None,
        serde_json::json!({"endpoint": endpoint, "bytes_read": read, "response": String::from_utf8_lossy(&buf[..read])}),
    )
}
fn udp(req: &NetworkProbeRequest, started: Instant, _cancel: &AtomicBool) -> NetworkProbeResult {
    let endpoint = match endpoint(req, 53) {
        Ok(v) => v,
        Err(e) => {
            return result(
                req,
                started,
                false,
                "参数无效".into(),
                Some(e),
                serde_json::json!({}),
            )
        }
    };
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(v) => v,
        Err(e) => {
            return result(
                req,
                started,
                false,
                "UDP 套接字创建失败".into(),
                Some(e.to_string()),
                serde_json::json!({}),
            )
        }
    };
    let payload = match decode_payload(req) {
        Ok(v) => v,
        Err(e) => {
            return result(
                req,
                started,
                false,
                "数据无效".into(),
                Some(e),
                serde_json::json!({}),
            )
        }
    };
    if let Err(e) = socket.send_to(&payload, &endpoint) {
        return result(
            req,
            started,
            false,
            "UDP 发送失败".into(),
            Some(e.to_string()),
            serde_json::json!({"endpoint": endpoint}),
        );
    }
    let _ = socket.set_read_timeout(Some(timeout(req)));
    let mut buf = vec![0; 8192];
    match socket.recv_from(&mut buf) {
        Ok((size, peer)) => result(
            req,
            started,
            true,
            "UDP 收到响应".into(),
            None,
            serde_json::json!({"peer": peer.to_string(), "bytes_read": size, "response": String::from_utf8_lossy(&buf[..size])}),
        ),
        Err(e)
            if e.kind() == std::io::ErrorKind::WouldBlock
                || e.kind() == std::io::ErrorKind::TimedOut =>
        {
            result(
                req,
                started,
                false,
                "UDP 已发送但未收到响应".into(),
                Some("no_response".into()),
                serde_json::json!({"endpoint": endpoint}),
            )
        }
        Err(e) => result(
            req,
            started,
            false,
            "UDP 接收失败".into(),
            Some(e.to_string()),
            serde_json::json!({}),
        ),
    }
}
fn http(req: &NetworkProbeRequest, started: Instant, cancel: &AtomicBool) -> NetworkProbeResult {
    if let Err(e) = cancelled(cancel) {
        return result(
            req,
            started,
            false,
            e.clone(),
            Some(e),
            serde_json::json!({}),
        );
    }
    let client = match Client::builder()
        .timeout(timeout(req))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
    {
        Ok(v) => v,
        Err(e) => {
            return result(
                req,
                started,
                false,
                "HTTP 客户端创建失败".into(),
                Some(e.to_string()),
                serde_json::json!({}),
            )
        }
    };
    let method = req
        .method
        .as_deref()
        .unwrap_or("GET")
        .parse()
        .unwrap_or(reqwest::Method::GET);
    let mut request = client.request(method, req.target.trim());
    if let Some(headers) = &req.headers {
        match parse_headers(headers) {
            Ok(items) => {
                for (k, v) in items {
                    request = request.header(k, v);
                }
            }
            Err(e) => {
                return result(
                    req,
                    started,
                    false,
                    "请求头无效".into(),
                    Some(e),
                    serde_json::json!({}),
                )
            }
        }
    }
    if let Some(body) = &req.body {
        request = request.body(body.clone());
    }
    match request.send() {
        Ok(response) => {
            let status_code = response.status();
            let status = status_code.as_u16();
            let headers = response
                .headers()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or_default().to_string()))
                .collect::<std::collections::BTreeMap<_, _>>();
            let body = response.text().unwrap_or_default();
            result(
                req,
                started,
                status_code.is_success(),
                format!("HTTP {}", status),
                None,
                serde_json::json!({"status": status, "headers": headers, "body": body}),
            )
        }
        Err(e) => result(
            req,
            started,
            false,
            "HTTP 请求失败".into(),
            Some(e.to_string()),
            serde_json::json!({}),
        ),
    }
}
fn websocket(
    req: &NetworkProbeRequest,
    started: Instant,
    cancel: &AtomicBool,
) -> NetworkProbeResult {
    if let Err(e) = cancelled(cancel) {
        return result(
            req,
            started,
            false,
            e.clone(),
            Some(e),
            serde_json::json!({}),
        );
    }
    let url = match Url::parse(req.target.trim()) {
        Ok(v) => v,
        Err(e) => {
            return result(
                req,
                started,
                false,
                "WebSocket URL 无效".into(),
                Some(e.to_string()),
                serde_json::json!({}),
            )
        }
    };
    let mut request = Request::builder().uri(url.as_str());
    if let Some(headers) = &req.headers {
        match parse_headers(headers) {
            Ok(items) => {
                for (key, value) in items {
                    request = request.header(key, value);
                }
            }
            Err(error) => {
                return result(
                    req,
                    started,
                    false,
                    "请求头无效".into(),
                    Some(error),
                    serde_json::json!({}),
                )
            }
        }
    }
    let request = match request.body(()) {
        Ok(request) => request,
        Err(error) => {
            return result(
                req,
                started,
                false,
                "WebSocket 请求无效".into(),
                Some(error.to_string()),
                serde_json::json!({}),
            )
        }
    };
    let (mut socket, response) = match connect(request) {
        Ok(v) => v,
        Err(e) => {
            return result(
                req,
                started,
                false,
                "WebSocket 连接失败".into(),
                Some(e.to_string()),
                serde_json::json!({}),
            )
        }
    };
    let payload = req.payload.clone().unwrap_or_default();
    if !payload.is_empty() {
        if let Err(e) = socket.send(Message::Text(payload.into())) {
            return result(
                req,
                started,
                false,
                "WebSocket 发送失败".into(),
                Some(e.to_string()),
                serde_json::json!({}),
            );
        }
    }
    let received = socket.read().ok().map(|m| format!("{m:?}"));
    let _ = socket.close(None);
    result(
        req,
        started,
        true,
        "WebSocket 连接成功".into(),
        None,
        serde_json::json!({"status": response.status().as_u16(), "response": received}),
    )
}
pub fn run(req: NetworkProbeRequest, cancel: Arc<AtomicBool>) -> NetworkProbeResult {
    let started = Instant::now();
    match req.tool.as_str() {
        "ping" => ping(&req, started, &cancel),
        "traceroute" => traceroute(&req, started, &cancel),
        "telnet" | "tcp" => tcp(&req, started, &cancel),
        "udp" => udp(&req, started, &cancel),
        "http" => http(&req, started, &cancel),
        "certificate" => certificate(&req, started, &cancel),
        "websocket" => websocket(&req, started, &cancel),
        _ => result(
            &req,
            started,
            false,
            "未知探测工具".into(),
            Some("unknown_tool".into()),
            serde_json::json!({}),
        ),
    }
}
