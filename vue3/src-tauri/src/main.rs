#[path = "../../../src/config.rs"]
mod config;
#[path = "../../../src/policy.rs"]
mod policy;
#[path = "../../../src/proxy.rs"]
mod proxy;
#[path = "../../../src/ssh.rs"]
mod ssh;

use config::Config;
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, process::Command, sync::Arc};
use tauri::{Manager, State};

struct AppState {
    config: Arc<RwLock<Config>>,
    proxy: proxy::ProxyManager,
    ssh: ssh::SshManager,
    operation: Mutex<()>,
    tray_status: Mutex<Option<tauri::menu::MenuItem<tauri::Wry>>>,
    message: Mutex<Option<String>>,
}
#[derive(Serialize)]
struct Snapshot {
    config: Config,
    running: bool,
    logs: Vec<proxy::LogEntry>,
    traffic: Vec<u64>,
    message: Option<String>,
    ssh_running: bool,
    ssh_local_port: Option<u16>,
    interfaces: Vec<NetworkInterface>,
}
#[derive(Clone, Serialize)]
struct NetworkInterface { name: String, kind: String, addresses: Vec<String> }
#[derive(Clone, Serialize)]
struct PublicIpProbe { ip: Option<String>, sources: Vec<String>, confidence: String, error: Option<String> }
#[derive(Clone, Serialize)]
struct UpdateInfo { current_version: String, latest_version: Option<String>, release_url: Option<String>, available: bool, error: Option<String> }
const APP_VERSION: &str = "0.1.1";
fn version_tuple(value: &str) -> Option<(u64, u64, u64)> {
    let values = value.trim().trim_start_matches('v').split('.').map(|part| part.split('-').next().unwrap_or(part).parse::<u64>().ok()).collect::<Option<Vec<_>>>()?;
    (values.len() >= 3).then_some((values[0], values[1], values[2]))
}
#[tauri::command]
fn check_update() -> UpdateInfo {
    let output = Command::new("/usr/bin/curl").args(["--fail", "--silent", "--show-error", "--location", "--max-time", "10", "-H", "Accept: application/vnd.github+json", "https://api.github.com/repos/yogkang/DomainEgress/releases/latest"]).output();
    let response = match output { Ok(output) if output.status.success() => output, Ok(output) => return UpdateInfo { current_version: APP_VERSION.into(), latest_version: None, release_url: None, available: false, error: Some(String::from_utf8_lossy(&output.stderr).trim().to_string()) }, Err(error) => return UpdateInfo { current_version: APP_VERSION.into(), latest_version: None, release_url: None, available: false, error: Some(error.to_string()) } };
    let payload: serde_json::Value = match serde_json::from_slice(&response.stdout) { Ok(value) => value, Err(error) => return UpdateInfo { current_version: APP_VERSION.into(), latest_version: None, release_url: None, available: false, error: Some(format!("Release 返回格式无效：{error}")) } };
    let latest = payload.get("tag_name").and_then(|value| value.as_str()).unwrap_or_default().trim_start_matches('v').to_string();
    let release_url = payload.get("html_url").and_then(|value| value.as_str()).map(String::from);
    let available = version_tuple(&latest).zip(version_tuple(APP_VERSION)).is_some_and(|(latest, current)| latest > current);
    UpdateInfo { current_version: APP_VERSION.into(), latest_version: (!latest.is_empty()).then_some(latest), release_url, available, error: None }
}
#[tauri::command]
fn open_update(url: String) -> Result<(), String> {
    if !url.starts_with("https://github.com/yogkang/DomainEgress/releases/") { return Err("更新地址不受信任".into()); }
    Command::new("/usr/bin/open").arg(url).status().map_err(|e| e.to_string()).and_then(|status| if status.success() { Ok(()) } else { Err("无法打开更新页面".into()) })
}
fn fetch_public_ip(url: &str) -> Result<String, String> {
    let output = Command::new("/usr/bin/curl").args(["--fail", "--silent", "--show-error", "--location", "--max-time", "10", url]).output().map_err(|e| e.to_string())?;
    if !output.status.success() { return Err(String::from_utf8_lossy(&output.stderr).trim().to_string()); }
    let value = String::from_utf8_lossy(&output.stdout).trim().parse::<std::net::IpAddr>().map_err(|_| "来源返回的不是有效 IP 地址".to_string())?;
    let public = match value {
        std::net::IpAddr::V4(ip) => !(ip.is_private() || ip.is_loopback() || ip.is_link_local() || ip.is_unspecified() || ip.is_multicast()),
        std::net::IpAddr::V6(ip) => !(ip.is_unique_local() || ip.is_loopback() || ip.is_unicast_link_local() || ip.is_unspecified() || ip.is_multicast()),
    };
    if !public { return Err("来源返回的不是公网 IP 地址".into()); }
    Ok(value.to_string())
}
#[tauri::command]
fn probe_public_ip() -> PublicIpProbe {
    let endpoints = [
        ("ipify", "https://api.ipify.org"),
        ("icanhazip", "https://ipv4.icanhazip.com"),
        ("amazon", "https://checkip.amazonaws.com"),
        ("ident", "https://4.ident.me"),
    ];
    let mut results: Vec<(&str, String)> = Vec::new();
    for (name, url) in endpoints {
        let result = fetch_public_ip(url);
        if let Ok(ip) = &result {
            for (first_name, first_ip) in &results {
                if first_ip == ip {
                    return PublicIpProbe { ip: Some(ip.clone()), sources: vec![(*first_name).into(), name.into()], confidence: "双源一致".into(), error: None };
                }
            }
            results.push((name, ip.clone()));
        }
    }
    PublicIpProbe { ip: None, sources: results.iter().map(|(name, _)| (*name).into()).collect(), confidence: "未确认".into(), error: Some("候选地址均未能得到两个一致的公网 IPv4 来源".into()) }
}
fn local_interfaces() -> Vec<NetworkInterface> {
    let output = Command::new("/usr/sbin/networksetup").arg("-listallhardwareports").output().ok();
    let text = output.map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
    let mut result = Vec::new(); let mut kind = String::new(); let mut device = String::new();
    for line in text.lines().chain(std::iter::once("")) {
        if let Some(value) = line.strip_prefix("Hardware Port: ") { kind = value.trim().to_string(); }
        if let Some(value) = line.strip_prefix("Device: ") { device = value.trim().to_string(); }
        if line.is_empty() && !device.is_empty() {
            let body = Command::new("/sbin/ifconfig").arg(&device).output().ok().map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
            let addresses = body.lines().filter_map(|line| { let fields: Vec<_> = line.split_whitespace().collect(); if fields.first() == Some(&"inet") || fields.first() == Some(&"inet6") { fields.get(1).map(|x| x.to_string()) } else { None } }).collect::<Vec<_>>();
            if !addresses.is_empty() { result.push(NetworkInterface { name: device.clone(), kind: kind.clone(), addresses }); }
            kind.clear(); device.clear();
        }
    }
    result
}
#[tauri::command]
fn snapshot(state: State<AppState>) -> Snapshot {
    Snapshot {
        config: state.config.read().clone(),
        running: state.proxy.is_running(),
        logs: state.proxy.logs(),
        traffic: state.proxy.traffic(),
        message: state.message.lock().take(),
        ssh_running: state.ssh.is_running(),
        ssh_local_port: state.ssh.local_port(),
        interfaces: local_interfaces(),
    }
}
fn keychain_service() -> &'static str { "com.domainegress.client.ssh" }
#[tauri::command]
fn keychain_set(account: String, secret: String) -> Result<(), String> {
    if account.trim().is_empty() || secret.is_empty() { return Err("钥匙串账号和凭据不能为空".into()); }
    let output = Command::new("/usr/bin/security").args(["add-generic-password", "-a", &account, "-s", keychain_service(), "-w", &secret, "-U"]).output().map_err(|e| e.to_string())?;
    if output.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&output.stderr).trim().to_string()) }
}
#[tauri::command]
fn keychain_delete(account: String) -> Result<(), String> {
    let output = Command::new("/usr/bin/security").args(["delete-generic-password", "-a", &account, "-s", keychain_service()]).output().map_err(|e| e.to_string())?;
    if output.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&output.stderr).trim().to_string()) }
}
fn icloud_config_path() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("." )).join("Library/Mobile Documents/iCloud~com~domainegress~client/Documents/DomainEgress/config.json")
}
#[tauri::command]
fn icloud_status() -> Result<bool, String> { Ok(icloud_config_path().parent().is_some_and(|p| p.exists())) }
#[tauri::command]
fn icloud_sync(state: State<AppState>) -> Result<String, String> {
    let path = icloud_config_path(); let parent = path.parent().ok_or_else(|| "iCloud 目录无效".to_string())?;
    fs::create_dir_all(parent).map_err(|e| format!("无法创建 iCloud 同步目录：{e}"))?;
    let config = state.config.read().clone(); let data = serde_json::to_vec_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(&path, data).map_err(|e| format!("iCloud 配置写入失败：{e}"))?;
    Ok(path.display().to_string())
}
#[tauri::command]
fn icloud_read() -> Result<Option<Config>, String> {
    let path = icloud_config_path(); if !path.exists() { return Ok(None); }
    let data = fs::read(&path).map_err(|e| e.to_string())?;
    serde_json::from_slice(&data).map(Some).map_err(|e| format!("iCloud 配置格式无效：{e}"))
}
fn validate(config: &Config) -> Result<(), String> {
    if !["whitelist", "blacklist"].contains(&config.access_mode.as_str()) {
        return Err("访问模式无效".into());
    }
    if !["error", "warn", "info", "debug"].contains(&config.log_level.as_str()) {
        return Err("日志级别无效".into());
    }
    if !(1..=3650).contains(&config.log_retention_days) {
        return Err("日志保留天数应为 1–3650".into());
    }
    if !(1..=21).contains(&config.trend_retention_days) {
        return Err("趋势历史保留天数应为 1–21".into());
    }
    for (host, port) in [
        (&config.http_host, config.http_port),
        (&config.socks_host, config.socks_port),
    ] {
        if host.parse::<std::net::IpAddr>().is_err() || port == 0 {
            return Err("监听地址必须为 IP，端口应为 1–65535".into());
        }
    }
    if config.http_host == config.socks_host && config.http_port == config.socks_port {
        return Err("HTTP 与 SOCKS5 不能使用相同监听地址和端口".into());
    }
    for rule in config.whitelist.iter().chain(&config.blacklist) {
        let host = rule.strip_prefix("*.").or_else(|| rule.strip_prefix('.')).unwrap_or(rule);
        let valid_domain = host.len() <= 253
            && host.split('.').all(|part| {
                !part.is_empty()
                    && part.len() <= 63
                    && !part.starts_with('-')
                    && !part.ends_with('-')
                    && part.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
            });
        if host.parse::<std::net::IpAddr>().is_err() && !valid_domain {
            return Err(format!("无效规则：{rule}。请输入域名、IP 或 *.example.com"));
        }
    }
    Ok(())
}
#[tauri::command]
fn save_config(config: Config, state: State<AppState>) -> Result<(), String> {
    validate(&config)?;
    let _operation = state.operation.lock();
    let previous = state.config.read().clone();
    let changed = previous.http_host != config.http_host
        || previous.http_port != config.http_port
        || previous.socks_host != config.socks_host
        || previous.socks_port != config.socks_port;
    if state.proxy.is_running() && changed {
        return Err("请先停止代理，再修改监听地址或端口".into());
    }
    config.save().map_err(|e| format!("配置保存失败：{e}"))?;
    *state.config.write() = config;
    Ok(())
}
#[tauri::command]
fn set_running(running: bool, state: State<AppState>) -> Result<(), String> {
    let _operation = state.operation.lock();
    if running {
        validate(&state.config.read())?;
        if let Some(id) = state.config.read().active_ssh_profile.clone() {
            if let Some(profile) = state.config.read().ssh_profiles.iter().find(|p| p.id == id) {
                let port = state.ssh.start(profile).map_err(|e| e.to_string())?;
                state.config.write().ssh_proxy_port = Some(port);
            }
        }
        state.proxy.start()
    } else {
        let result = state.proxy.stop();
        let _ = state.ssh.stop();
        state.config.write().ssh_proxy_port = None;
        result
    }
    .map_err(|e| e.to_string())?;
    update_tray_status(&state, running);
    Ok(())
}
fn update_tray_status(state: &AppState, running: bool) {
    if let Some(item) = state.tray_status.lock().as_ref() {
        let _ = item.set_text(if running { "🛡️ 代理运行中" } else { "⏹️ 代理已停止" });
    }
}
#[derive(Serialize, Deserialize)]
struct GistRules {
    format: String,
    version: u32,
    whitelist: Vec<String>,
    blacklist: Vec<String>,
}
fn gist_api_url(provider: &str, gist_id: &str) -> Result<String, String> {
    if gist_id.trim().is_empty() { return Err("请填写 Gist ID".into()); }
    match provider {
        "github" => Ok(format!("https://api.github.com/gists/{}", gist_id.trim())),
        "gitee" => Ok(format!("https://gitee.com/api/v5/gists/{}", gist_id.trim())),
        _ => Err("不支持的 Gist 服务商".into()),
    }
}
fn gist_request(url: &str, token: &str, method: &str, body: Option<String>) -> Result<serde_json::Value, String> {
    let mut command = Command::new("/usr/bin/curl");
    command.args(["-sS", "-f", "-X", method, "-H", "Accept: application/json"]);
    if !token.trim().is_empty() { command.args(["-H", &format!("Authorization: token {}", token.trim())]); }
    if let Some(body) = body { command.args(["-H", "Content-Type: application/json", "--data-raw", &body]); }
    let output = command.arg(url).output().map_err(|e| format!("请求 Gist 失败：{e}"))?;
    if !output.status.success() { return Err(String::from_utf8_lossy(&output.stderr).trim().to_string()); }
    serde_json::from_slice(&output.stdout).map_err(|e| format!("Gist 返回格式无效：{e}"))
}
#[tauri::command]
fn gist_pull(provider: String, gist_id: String, file_name: String, token: String) -> Result<GistRules, String> {
    let response = gist_request(&gist_api_url(&provider, &gist_id)?, &token, "GET", None)?;
    let file = response.get("files").and_then(|files| files.get(&file_name)).and_then(|file| file.get("content")).and_then(|content| content.as_str()).ok_or_else(|| format!("Gist 中未找到文件：{file_name}"))?;
    serde_json::from_str(file).map_err(|e| format!("规则文件格式无效：{e}"))
}
#[tauri::command]
fn gist_push(provider: String, gist_id: String, file_name: String, token: String, config: Config) -> Result<(), String> {
    if token.trim().is_empty() { return Err("推送 Gist 需要访问令牌".into()); }
    let rules = GistRules { format: "domain-egress-rules".into(), version: 1, whitelist: config.whitelist, blacklist: config.blacklist };
    let content = serde_json::to_string_pretty(&rules).map_err(|e| e.to_string())?;
    let body = serde_json::json!({ "files": { file_name: { "content": content } } }).to_string();
    gist_request(&gist_api_url(&provider, &gist_id)?, &token, "PATCH", Some(body)).map(|_| ())
}
#[tauri::command]
fn clear_logs(state: State<AppState>) {
    state.proxy.clear_logs();
}
#[derive(Serialize)]
struct PortRow {
    port: String,
    pid: u32,
    name: String,
    started: String,
    elapsed: String,
}
fn ps_value(pid: u32, field: &str) -> Result<String, String> {
    let output = Command::new("/bin/ps")
        .args(["-p", &pid.to_string(), "-o", &format!("{field}=")])
        .output()
        .map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
fn ports() -> Result<Vec<PortRow>, String> {
    let output = Command::new("/usr/sbin/lsof")
        .args(["-nP", "-iTCP", "-sTCP:LISTEN", "-F", "pcn"])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() && !output.stderr.is_empty() {
        return Err(String::from_utf8_lossy(&output.stderr).into());
    }
    let (mut pid, mut name, mut rows) = (0, String::new(), Vec::new());
    let mut times = std::collections::HashMap::new();
    for field in String::from_utf8_lossy(&output.stdout).lines() {
        if let Some(v) = field.strip_prefix('p') {
            pid = v.parse().unwrap_or(0);
        } else if let Some(v) = field.strip_prefix('c') {
            name = v.to_string();
        } else if let Some(v) = field.strip_prefix('n') {
            if pid > 0 {
                let (started, elapsed) = times.entry(pid).or_insert_with(|| {
                    (
                        ps_value(pid, "lstart").unwrap_or_default(),
                        ps_value(pid, "etime").unwrap_or_default(),
                    )
                });
                rows.push(PortRow {
                    port: v.into(),
                    pid,
                    name: name.clone(),
                    started: started.clone(),
                    elapsed: elapsed.clone(),
                });
            }
        }
    }
    rows.sort_by(|a, b| a.port.cmp(&b.port).then(a.pid.cmp(&b.pid)));
    rows.dedup_by(|a, b| a.pid == b.pid && a.port == b.port);
    Ok(rows)
}
#[tauri::command]
async fn list_ports() -> Result<Vec<PortRow>, String> {
    tauri::async_runtime::spawn_blocking(ports)
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn terminate_process(pid: u32, started: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        if pid <= 1 || pid == std::process::id() {
            return Err("不允许终止此进程".into());
        }
        if !ports()?
            .iter()
            .any(|p| p.pid == pid && p.started == started)
            || started.is_empty()
        {
            return Err("进程已变化，请刷新列表后重试".into());
        }
        let output = Command::new("/bin/kill")
            .args(["-TERM", &pid.to_string()])
            .output()
            .map_err(|e| e.to_string())?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).into())
        }
    })
    .await
    .map_err(|e| e.to_string())?
}
fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let (config, mut message) = match Config::load() {
                Ok(c) => (c, None),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (Config::default(), None),
                Err(e) => {
                    let mut c = Config::default();
                    c.auto_start = false;
                    (c, Some(format!("配置读取失败，已暂停自动启动：{e}")))
                }
            };
            let shared = Arc::new(RwLock::new(config.clone()));
            let proxy = proxy::ProxyManager::new(shared.clone());
            let ssh = ssh::SshManager::new();
            if config.auto_start {
                let startup = validate(&config).and_then(|_| {
                    if let Some(id) = config.active_ssh_profile.as_ref() {
                        if let Some(profile) = config.ssh_profiles.iter().find(|p| &p.id == id) {
                            let port = ssh.start(profile).map_err(|e| e.to_string())?;
                            shared.write().ssh_proxy_port = Some(port);
                        }
                    }
                    proxy.start().map_err(|e| e.to_string())
                });
                if let Err(e) = startup {
                    let _ = ssh.stop();
                    message = Some(format!(
                        "代理自动启动失败：{e}。请检查是否有其他进程占用监听端口。"
                    ));
                }
            }
            app.manage(AppState {
                config: shared,
                proxy,
                ssh,
                operation: Mutex::new(()),
                tray_status: Mutex::new(None),
                message: Mutex::new(message),
            });
            use tauri::{
                menu::{Menu, MenuItem},
                tray::TrayIconBuilder,
            };
            let show = MenuItem::with_id(app, "show", "显示 DomainEgress", true, None::<&str>)?;
            let status = MenuItem::with_id(
                app,
                "status",
                if app.state::<AppState>().proxy.is_running() {
                    "🛡️ 代理运行中"
                } else {
                    "⏹️ 代理已停止"
                },
                false,
                None::<&str>,
            )?;
            let start = MenuItem::with_id(app, "start", "启动代理", true, None::<&str>)?;
            let stop = MenuItem::with_id(app, "stop", "停止代理", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            *app.state::<AppState>().tray_status.lock() = Some(status.clone());
            let menu = Menu::with_items(app, &[&show, &status, &start, &stop, &quit])?;
            TrayIconBuilder::new()
                .icon(tauri::image::Image::from_bytes(include_bytes!(
                    "../icons/icon.png"
                ))?)
                .tooltip("DomainEgress")
                .menu(&menu)
                .on_menu_event(|app, event| {
                    let state = app.state::<AppState>();
                    match event.id.as_ref() {
                        "show" => {
                            if let Some(w) = app.get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                        "start" | "stop" => {
                            if let Err(e) = set_running(event.id.as_ref() == "start", state.clone())
                            {
                                *state.message.lock() = Some(e);
                                if let Some(w) = app.get_webview_window("main") {
                                    let _ = w.show();
                                }
                            }
                        }
                        "quit" => {
                            let _ = state.proxy.stop();
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            snapshot,
            probe_public_ip,
            check_update,
            open_update,
            save_config,
            set_running,
            clear_logs,
            gist_pull,
            gist_push,
            list_ports,
            terminate_process
            ,keychain_set, keychain_delete, icloud_status, icloud_sync, icloud_read
        ])
        .build(tauri::generate_context!())
        .expect("启动 DomainEgress 失败")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                let _ = app.state::<AppState>().proxy.stop();
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn config_validation() {
        let mut config = Config::default();
        assert!(validate(&config).is_ok());
        config.whitelist.push("https://example.com/path".into());
        assert!(validate(&config).is_err());
        config.whitelist = vec!["*.example.com".into(), "::1".into()];
        assert!(validate(&config).is_ok());
        config.socks_port = config.http_port;
        assert!(validate(&config).is_err());
    }
}
