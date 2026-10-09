use axum::{
    body::Body,
    extract::{ConnectInfo, Request},
    response::Response,
    Router,
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    sync::Arc,
};
use tower::ServiceExt;
use tower_http::services::ServeFile;

#[derive(Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub port: u16,
    pub host: String,
    pub root: String,
}
#[derive(Clone, Serialize)]
pub struct AccessLog {
    pub timestamp: String,
    pub client: String,
    pub port: u16,
    pub method: String,
    pub path: String,
    pub status: u16,
}
#[derive(Serialize)]
pub struct Snapshot {
    pub services: Vec<ServiceConfig>,
    pub running: Vec<u16>,
    pub logs: Vec<AccessLog>,
}
#[derive(Default)]
struct Inner {
    services: Vec<ServiceConfig>,
    running: HashMap<u16, tokio::task::JoinHandle<()>>,
    logs: Vec<AccessLog>,
}
#[derive(Clone)]
pub struct Manager {
    inner: Arc<Mutex<Inner>>,
    path: PathBuf,
}
impl Manager {
    pub fn new() -> Self {
        let path = dirs::data_local_dir()
            .unwrap_or_default()
            .join("DomainEgress/static-http.json");
        Self::with_storage(path)
    }
    pub fn with_storage(path: PathBuf) -> Self {
        let services = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Self {
            inner: Arc::new(Mutex::new(Inner {
                services,
                ..Default::default()
            })),
            path,
        }
    }
    pub fn snapshot(&self) -> Snapshot {
        let inner = self.inner.lock();
        Snapshot {
            services: inner.services.clone(),
            running: inner
                .running
                .iter()
                .filter(|(_, task)| !task.is_finished())
                .map(|(port, _)| *port)
                .collect(),
            logs: inner.logs.clone(),
        }
    }
    fn persist(&self, services: &[ServiceConfig]) -> Result<(), String> {
        std::fs::create_dir_all(self.path.parent().unwrap()).map_err(|e| e.to_string())?;
        let temporary = self.path.with_extension("json.tmp");
        std::fs::write(
            &temporary,
            serde_json::to_vec_pretty(services).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        std::fs::rename(temporary, &self.path).map_err(|e| e.to_string())
    }
    pub fn save(&self, mut config: ServiceConfig) -> Result<(), String> {
        if config.port == 0 {
            return Err("端口必须为 1–65535".into());
        }
        config
            .host
            .trim()
            .parse::<IpAddr>()
            .map_err(|_| "监听地址必须为 IP 地址")?;
        config.host = config.host.trim().into();
        let root =
            std::fs::canonicalize(config.root.trim()).map_err(|e| format!("目录不可访问：{e}"))?;
        if !root.is_dir() {
            return Err("请选择目录作为根目录".into());
        }
        config.root = root.to_string_lossy().into();
        let mut inner = self.inner.lock();
        if inner
            .running
            .get(&config.port)
            .is_some_and(|task| !task.is_finished())
            && inner
                .services
                .iter()
                .any(|s| s.port == config.port && s.host != config.host)
        {
            return Err("修改监听地址前请先停止服务".into());
        }
        let mut services = inner.services.clone();
        services.retain(|s| s.port != config.port);
        services.push(config);
        services.sort_by_key(|s| s.port);
        self.persist(&services)?;
        inner.services = services;
        Ok(())
    }
    pub async fn remove(&self, port: u16) -> Result<(), String> {
        let task = {
            let mut inner = self.inner.lock();
            let services: Vec<_> = inner
                .services
                .iter()
                .filter(|s| s.port != port)
                .cloned()
                .collect();
            self.persist(&services)?;
            inner.services = services;
            inner.running.remove(&port)
        };
        if let Some(task) = task {
            task.abort();
            let _ = task.await;
        }
        Ok(())
    }
    pub async fn stop(&self, port: u16) {
        let task = self.inner.lock().running.remove(&port);
        if let Some(task) = task {
            task.abort();
            let _ = task.await;
        }
    }
    pub fn clear(&self) {
        self.inner.lock().logs.clear();
    }
    pub fn start(&self, port: u16) -> Result<(), String> {
        let mut inner = self.inner.lock();
        if inner.running.get(&port).is_some_and(|t| !t.is_finished()) {
            return Ok(());
        }
        let config = inner
            .services
            .iter()
            .find(|s| s.port == port)
            .ok_or("请先保存服务配置")?;
        let address = SocketAddr::new(config.host.parse().map_err(|_| "监听地址无效")?, port);
        let listener = std::net::TcpListener::bind(address)
            .map_err(|e| format!("端口 {port} 启动失败：{e}"))?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let listener = tokio::net::TcpListener::from_std(listener).map_err(|e| e.to_string())?;
        let manager = self.clone();
        let app = Router::new().fallback(
            move |ConnectInfo(client): ConnectInfo<SocketAddr>, request: Request| {
                let manager = manager.clone();
                async move { manager.serve(port, client, request).await }
            },
        );
        inner.running.insert(
            port,
            tokio::spawn(async move {
                let _ = axum::serve(
                    listener,
                    app.into_make_service_with_connect_info::<SocketAddr>(),
                )
                .await;
            }),
        );
        Ok(())
    }
    async fn serve(&self, port: u16, client: SocketAddr, request: Request) -> Response {
        let method = request.method().to_string();
        let uri = request.uri().to_string();
        let root = self
            .inner
            .lock()
            .services
            .iter()
            .find(|s| s.port == port)
            .map(|s| PathBuf::from(&s.root));
        let path = decode_path(request.uri().path());
        let response = match (root, path) {
            (Some(root), Some(path)) if method == "GET" || method == "HEAD" => {
                let candidate = root.join(path.trim_start_matches('/'));
                match std::fs::canonicalize(&candidate) {
                    Ok(directory)
                        if directory.starts_with(&root)
                            && directory.is_dir()
                            && !request.uri().path().ends_with('/') =>
                    {
                        let mut location =
                            format!("/{}/", request.uri().path().trim_start_matches('/'));
                        if let Some(query) = request.uri().query() {
                            location.push('?');
                            location.push_str(query);
                        }
                        Response::builder()
                            .status(308)
                            .header("Location", location)
                            .body(Body::empty())
                            .unwrap()
                    }
                    _ => {
                        let candidate = if candidate.is_dir() {
                            candidate.join("index.html")
                        } else {
                            candidate
                        };
                        match std::fs::canonicalize(candidate) {
                            Ok(file) if file.starts_with(&root) && file.is_file() => {
                                match ServeFile::new(file).oneshot(request).await {
                                    Ok(response) => response.map(Body::new),
                                    Err(_) => error(500, "读取文件失败"),
                                }
                            }
                            Ok(_) => error(403, "禁止访问根目录之外的文件"),
                            Err(_) => error(404, "文件不存在"),
                        }
                    }
                }
            }
            (_, None) => error(400, "请求路径无效"),
            _ => Response::builder()
                .status(405)
                .header("Allow", "GET, HEAD")
                .body(Body::empty())
                .unwrap(),
        };
        let log = AccessLog {
            timestamp: chrono::Local::now().to_rfc3339(),
            client: client.to_string(),
            port,
            method,
            path: uri,
            status: response.status().as_u16(),
        };
        println!(
            "[static-http] {} {} :{} {} {} {}",
            log.timestamp,
            log.client,
            port,
            log.method,
            log.path.escape_default(),
            log.status
        );
        let mut inner = self.inner.lock();
        inner.logs.push(log);
        if inner.logs.len() > 1000 {
            inner.logs.remove(0);
        }
        response
    }
}
fn error(status: u16, message: &str) -> Response {
    Response::builder()
        .status(status)
        .header("Content-Type", "text/plain; charset=utf-8")
        .body(Body::from(message.to_string()))
        .unwrap()
}
fn decode_path(path: &str) -> Option<String> {
    let bytes = path.as_bytes();
    let mut result = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return None;
            }
            result.push(
                ((bytes[i + 1] as char).to_digit(16)? * 16 + (bytes[i + 2] as char).to_digit(16)?)
                    as u8,
            );
            i += 3;
        } else {
            result.push(bytes[i]);
            i += 1;
        }
    }
    let result = String::from_utf8(result).ok()?;
    if result.contains('\0') || result.contains('\\') || result.split('/').any(|s| s == "..") {
        return None;
    }
    Some(result)
}
