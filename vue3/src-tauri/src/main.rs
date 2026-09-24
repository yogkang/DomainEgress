mod cloud;
#[path = "../../../src/config.rs"]
mod config;
mod hosts;
mod network_tools;
#[path = "../../../src/policy.rs"]
mod policy;
#[path = "../../../src/proxy.rs"]
mod proxy;
#[path = "../../../src/ssh.rs"]
mod ssh;
#[path = "../../../src/ssh_forward.rs"]
mod ssh_forward;

use config::Config;
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, process::Command, sync::Arc};
use tauri::{Manager, State};

struct AppState {
    config: Arc<RwLock<Config>>,
    proxy: proxy::ProxyManager,
    ssh: Arc<ssh::SshManager>,
    ssh_forward: Arc<ssh_forward::SshForwardManager>,
    operation: Arc<Mutex<()>>,
    tray_status: Mutex<Option<tauri::menu::MenuItem<tauri::Wry>>>,
    tray_icon: Mutex<Option<tauri::tray::TrayIcon<tauri::Wry>>>,
    tray_menu: Mutex<Option<tauri::menu::Menu<tauri::Wry>>>,
    tray_start: Mutex<Option<tauri::menu::MenuItem<tauri::Wry>>>,
    tray_stop: Mutex<Option<tauri::menu::MenuItem<tauri::Wry>>>,
    tray_quit: Mutex<Option<tauri::menu::MenuItem<tauri::Wry>>>,
    message: Mutex<Option<String>>,
    geoip: Arc<GeoIpDatabases>,
    network_cancel: Arc<std::sync::atomic::AtomicBool>,
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
    hostname: String,
    ssh_forward_ports: std::collections::HashMap<String, u16>,
}
#[derive(Clone, Serialize)]
struct NetworkInterface {
    name: String,
    kind: String,
    addresses: Vec<String>,
}
#[derive(Clone, Serialize)]
struct GeoLocation {
    country: String,
    country_code: String,
    region: String,
    city: String,
    isp: String,
    source: String,
    confidence: String,
}
#[derive(Clone, Serialize)]
struct EgressAddress {
    ip: String,
    source: String,
    location: Option<GeoLocation>,
}
#[derive(Clone, Serialize)]
struct EgressProbe {
    addresses: Vec<EgressAddress>,
    confidence: String,
    error: Option<String>,
}
#[derive(Clone, Serialize)]
struct PublicIpProbe {
    system: EgressProbe,
    ipv6: Option<EgressProbe>,
}
struct GeoIpDatabases {
    ipv4: Option<ip2region::Searcher>,
    ipv6: Option<ip2region::Searcher>,
}
#[derive(Clone)]
struct OnlineGeoLocation {
    country: String,
    country_code: String,
    region: String,
    city: String,
    isp: String,
    source: &'static str,
}
#[derive(Clone, Serialize)]
struct UpdateInfo {
    current_version: String,
    latest_version: Option<String>,
    release_url: Option<String>,
    available: bool,
    error: Option<String>,
}
const APP_VERSION: &str = "0.4.1";
fn version_tuple(value: &str) -> Option<(u64, u64, u64)> {
    let values = value
        .trim()
        .trim_start_matches('v')
        .split('.')
        .map(|part| part.split('-').next().unwrap_or(part).parse::<u64>().ok())
        .collect::<Option<Vec<_>>>()?;
    (values.len() >= 3).then_some((values[0], values[1], values[2]))
}
#[tauri::command]
async fn check_update() -> Result<UpdateInfo, String> {
    tauri::async_runtime::spawn_blocking(check_update_blocking)
        .await
        .map_err(|error| format!("检查更新任务异常结束：{error}"))
}

fn check_update_blocking() -> UpdateInfo {
    let output = Command::new("/usr/bin/curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--max-time",
            "10",
            "-H",
            "Accept: application/vnd.github+json",
            "https://api.github.com/repos/yogkang/DomainEgress/releases/latest",
        ])
        .output();
    let response = match output {
        Ok(output) if output.status.success() => output,
        Ok(output) => {
            return UpdateInfo {
                current_version: APP_VERSION.into(),
                latest_version: None,
                release_url: None,
                available: false,
                error: Some(String::from_utf8_lossy(&output.stderr).trim().to_string()),
            }
        }
        Err(error) => {
            return UpdateInfo {
                current_version: APP_VERSION.into(),
                latest_version: None,
                release_url: None,
                available: false,
                error: Some(error.to_string()),
            }
        }
    };
    let payload: serde_json::Value = match serde_json::from_slice(&response.stdout) {
        Ok(value) => value,
        Err(error) => {
            return UpdateInfo {
                current_version: APP_VERSION.into(),
                latest_version: None,
                release_url: None,
                available: false,
                error: Some(format!("Release 返回格式无效：{error}")),
            }
        }
    };
    let latest = payload
        .get("tag_name")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .trim_start_matches('v')
        .to_string();
    let release_url = payload
        .get("html_url")
        .and_then(|value| value.as_str())
        .map(String::from);
    let available = version_tuple(&latest)
        .zip(version_tuple(APP_VERSION))
        .is_some_and(|(latest, current)| latest > current);
    UpdateInfo {
        current_version: APP_VERSION.into(),
        latest_version: (!latest.is_empty()).then_some(latest),
        release_url,
        available,
        error: None,
    }
}
#[tauri::command]
async fn open_update(url: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || open_update_blocking(url))
        .await
        .map_err(|error| format!("打开更新页面任务异常结束：{error}"))?
}
fn open_update_blocking(url: String) -> Result<(), String> {
    if !url.starts_with("https://github.com/yogkang/DomainEgress/releases/") {
        return Err("更新地址不受信任".into());
    }
    Command::new("/usr/bin/open")
        .arg(url)
        .status()
        .map_err(|e| e.to_string())
        .and_then(|status| {
            if status.success() {
                Ok(())
            } else {
                Err("无法打开更新页面".into())
            }
        })
}
fn curl_output(url: &str, ssh_port: Option<u16>) -> Result<std::process::Output, String> {
    let mut command = Command::new("/usr/bin/curl");
    command.args([
        "--fail",
        "--silent",
        "--show-error",
        "--location",
        "--connect-timeout",
        "3",
        "--max-time",
        "5",
    ]);
    let proxy;
    if let Some(port) = ssh_port {
        proxy = format!("socks5h://127.0.0.1:{port}");
        command.args(["--proxy", &proxy]);
    }
    command.arg(url).output().map_err(|e| e.to_string())
}
fn fetch_public_ip(url: &str, ssh_port: Option<u16>) -> Result<String, String> {
    let output = curl_output(url, ssh_port)?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    validate_public_ip(String::from_utf8_lossy(&output.stdout).trim())
}
const IPV4_HTTP_SOURCES: [&str; 4] = [
    "https://api.ipify.org",
    "https://checkip.amazonaws.com",
    "https://icanhazip.com",
    "https://ifconfig.me/ip",
];
const IPV6_HTTP_SOURCES: [&str; 1] = ["https://api6.ipify.org"];

fn fetch_public_ip_from_http_sources(
    sources: &[&str],
    family: &str,
) -> Result<(String, String), String> {
    let mut errors = Vec::new();
    for url in sources {
        match fetch_public_ip(url, None).and_then(|ip| public_ip_family(&ip, family)) {
            Ok(ip) => return Ok((ip, (*url).to_string())),
            Err(error) => errors.push(format!("{url}：{error}")),
        }
    }
    Err(format!("所有 HTTP 探测源均失败：{}", errors.join("；")))
}
fn fetch_public_ip_dns(record_type: &str) -> Result<String, String> {
    let output = Command::new("/usr/bin/dig")
        .args([
            "+tcp",
            "@208.67.222.222",
            "myip.opendns.com",
            record_type,
            "+short",
        ])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    let response = String::from_utf8_lossy(&output.stdout);
    let value = response
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .ok_or_else(|| "DNS 未返回 IP 地址".to_string())?;
    validate_public_ip(value)
}
fn value_string(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(|item| item.as_str())
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(String::from)
}
fn parse_ipwho_location(ip: &str, value: &serde_json::Value) -> Result<OnlineGeoLocation, String> {
    if value.get("success").and_then(|item| item.as_bool()) == Some(false) {
        return Err(value_string(value, "message").unwrap_or_else(|| "ipwho 返回失败".into()));
    }
    if value_string(value, "ip").as_deref() != Some(ip) {
        return Err("ipwho 返回的 IP 不匹配".into());
    }
    let country = value_string(value, "country").ok_or_else(|| "ipwho 未返回国家".to_string())?;
    let country_code =
        value_string(value, "country_code").ok_or_else(|| "ipwho 未返回国家代码".to_string())?;
    let connection = value.get("connection").unwrap_or(&serde_json::Value::Null);
    Ok(OnlineGeoLocation {
        country,
        country_code,
        region: value_string(value, "region").unwrap_or_default(),
        city: value_string(value, "city").unwrap_or_default(),
        isp: value_string(connection, "isp")
            .or_else(|| value_string(connection, "org"))
            .unwrap_or_default(),
        source: "ipwho",
    })
}
fn parse_ipwhois_location(
    ip: &str,
    value: &serde_json::Value,
) -> Result<OnlineGeoLocation, String> {
    if value.get("success").and_then(|item| item.as_bool()) == Some(false) {
        return Err(value_string(value, "message").unwrap_or_else(|| "ipwhois 返回失败".into()));
    }
    if value_string(value, "ip").as_deref() != Some(ip) {
        return Err("ipwhois 返回的 IP 不匹配".into());
    }
    let country = value_string(value, "country").ok_or_else(|| "ipwhois 未返回国家".to_string())?;
    let country_code =
        value_string(value, "country_code").ok_or_else(|| "ipwhois 未返回国家代码".to_string())?;
    Ok(OnlineGeoLocation {
        country,
        country_code,
        region: value_string(value, "region").unwrap_or_default(),
        city: value_string(value, "city").unwrap_or_default(),
        isp: value_string(value, "org").unwrap_or_default(),
        source: "ipwhois.app",
    })
}
fn fetch_online_location(ip: &str, ssh_port: Option<u16>) -> Result<GeoLocation, String> {
    let first = curl_output(&format!("https://ipwho.is/{ip}"), ssh_port).and_then(|output| {
        if output.status.success() {
            serde_json::from_slice(&output.stdout)
                .map_err(|error| error.to_string())
                .and_then(|value| parse_ipwho_location(ip, &value))
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    });
    let second =
        curl_output(&format!("https://ipwhois.app/json/{ip}"), ssh_port).and_then(|output| {
            if output.status.success() {
                serde_json::from_slice(&output.stdout)
                    .map_err(|error| error.to_string())
                    .and_then(|value| parse_ipwhois_location(ip, &value))
            } else {
                Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
            }
        });
    match (first, second) {
        (Ok(first), Ok(second))
            if first
                .country_code
                .eq_ignore_ascii_case(&second.country_code) =>
        {
            let city_matches = !first.city.is_empty()
                && first.city.eq_ignore_ascii_case(&second.city)
                && first.region.eq_ignore_ascii_case(&second.region);
            Ok(GeoLocation {
                country: first.country,
                country_code: first.country_code,
                region: if city_matches {
                    first.region
                } else {
                    String::new()
                },
                city: if city_matches {
                    first.city
                } else {
                    String::new()
                },
                isp: first.isp,
                source: format!("{}、{}", first.source, second.source),
                confidence: if city_matches {
                    "在线双源一致".into()
                } else {
                    "在线国家一致，城市有差异".into()
                },
            })
        }
        (Ok(first), Ok(second)) => Ok(GeoLocation {
            country: first.country,
            country_code: first.country_code,
            region: String::new(),
            city: String::new(),
            isp: first.isp,
            source: format!("{}、{}", first.source, second.source),
            confidence: "在线来源不一致".into(),
        }),
        (Ok(value), Err(_)) | (Err(_), Ok(value)) => Ok(GeoLocation {
            country: value.country,
            country_code: value.country_code,
            region: value.region,
            city: value.city,
            isp: value.isp,
            source: value.source.into(),
            confidence: "在线单源".into(),
        }),
        (Err(first), Err(second)) => Err(format!("在线归属地查询失败：{first}；{second}")),
    }
}
fn parse_ip2region_location(value: &str) -> Option<GeoLocation> {
    let fields: Vec<_> = value.split('|').collect();
    let country = fields.first()?.trim();
    if country.is_empty() || country == "0" || country.eq_ignore_ascii_case("reserved") {
        return None;
    }
    Some(GeoLocation {
        country: country.into(),
        country_code: fields
            .get(4)
            .map(|item| item.trim())
            .filter(|item| !item.is_empty() && *item != "0")
            .unwrap_or_default()
            .into(),
        region: fields
            .get(1)
            .map(|item| item.trim())
            .filter(|item| *item != "0")
            .unwrap_or_default()
            .into(),
        city: fields
            .get(2)
            .map(|item| item.trim())
            .filter(|item| *item != "0")
            .unwrap_or_default()
            .into(),
        isp: fields
            .get(3)
            .map(|item| item.trim())
            .filter(|item| *item != "0")
            .unwrap_or_default()
            .into(),
        source: "ip2region 离线库".into(),
        confidence: "离线兜底".into(),
    })
}
impl GeoIpDatabases {
    fn load(resource_dir: &std::path::Path) -> Self {
        let bundled = resource_dir.join("resources").join("geoip");
        let location = |name: &str| {
            let from_bundle = bundled.join(name);
            if from_bundle.exists() {
                from_bundle
            } else {
                let direct = resource_dir.join("geoip").join(name);
                if direct.exists() {
                    direct
                } else {
                    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("resources")
                        .join("geoip")
                        .join(name)
                }
            }
        };
        Self {
            ipv4: ip2region::Searcher::new(
                location("ip2region_v4.xdb").display().to_string(),
                ip2region::CachePolicy::VectorIndex,
            )
            .ok(),
            ipv6: ip2region::Searcher::new(
                location("ip2region_v6.xdb").display().to_string(),
                ip2region::CachePolicy::VectorIndex,
            )
            .ok(),
        }
    }
    fn lookup(&self, ip: &str) -> Option<GeoLocation> {
        let searcher = match ip.parse::<std::net::IpAddr>().ok()? {
            std::net::IpAddr::V4(_) => self.ipv4.as_ref(),
            std::net::IpAddr::V6(_) => self.ipv6.as_ref(),
        }?;
        searcher
            .search(ip)
            .ok()
            .as_deref()
            .and_then(parse_ip2region_location)
    }
}
fn validate_public_ip(value: &str) -> Result<String, String> {
    let value = value
        .parse::<std::net::IpAddr>()
        .map_err(|_| "来源返回的不是有效 IP 地址".to_string())?;
    let public = match value {
        std::net::IpAddr::V4(ip) => {
            !(ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_unspecified()
                || ip.is_multicast())
        }
        std::net::IpAddr::V6(ip) => {
            !(ip.is_unique_local()
                || ip.is_loopback()
                || ip.is_unicast_link_local()
                || ip.is_unspecified()
                || ip.is_multicast())
        }
    };
    if !public {
        return Err("来源返回的不是公网 IP 地址".into());
    }
    Ok(value.to_string())
}
fn resolve_address(ip: String, source: &str, geoip: &GeoIpDatabases) -> EgressAddress {
    let location = fetch_online_location(&ip, None)
        .or_else(|_| {
            geoip
                .lookup(&ip)
                .ok_or_else(|| String::from("离线归属地库未命中"))
        })
        .ok();
    EgressAddress {
        ip,
        source: source.into(),
        location,
    }
}
fn public_ip_family(ip: &str, family: &str) -> Result<String, String> {
    let ip = validate_public_ip(ip)?;
    match (family, ip.contains(':')) {
        ("IPv4", false) | ("IPv6", true) => Ok(ip),
        _ => Err(format!("来源未返回 {family} 公网地址")),
    }
}
fn probe_ip_family_route(
    geoip: &GeoIpDatabases,
    family: &str,
    http_urls: &[&str],
    dns_record_type: &str,
) -> EgressProbe {
    let http = fetch_public_ip_from_http_sources(http_urls, family);
    let dns = fetch_public_ip_dns(dns_record_type).and_then(|ip| public_ip_family(&ip, family));
    let dns_source = "DNS/TCP · 208.67.222.222";
    match (http, dns) {
        (Ok((http, http_url)), Ok(dns)) if http == dns => EgressProbe {
            addresses: vec![resolve_address(
                http,
                &format!("HTTP · {http_url}；{dns_source}"),
                geoip,
            )],
            confidence: "HTTP 与 DNS 一致".into(),
            error: None,
        },
        (Ok((http, http_url)), Ok(dns)) => EgressProbe {
            addresses: vec![
                resolve_address(http, &format!("HTTP · {http_url}"), geoip),
                resolve_address(dns, dns_source, geoip),
            ],
            confidence: "HTTP 与 DNS 不一致".into(),
            error: None,
        },
        (Ok((http, http_url)), Err(dns_error)) => EgressProbe {
            addresses: vec![resolve_address(http, &format!("HTTP · {http_url}"), geoip)],
            confidence: "仅 HTTP 成功".into(),
            error: Some(format!("DNS/TCP 探测失败：{dns_error}")),
        },
        (Err(http_error), Ok(dns)) => EgressProbe {
            addresses: vec![resolve_address(dns, dns_source, geoip)],
            confidence: "仅 DNS 成功".into(),
            error: Some(format!("HTTP 探测失败：{http_error}")),
        },
        (Err(http_error), Err(dns_error)) => EgressProbe {
            addresses: Vec::new(),
            confidence: "探测失败".into(),
            error: Some(format!("HTTP：{http_error}；DNS/TCP：{dns_error}")),
        },
    }
}
#[tauri::command]
async fn probe_public_ip(state: State<'_, AppState>) -> Result<PublicIpProbe, String> {
    let geoip = Arc::clone(&state.geoip);
    tauri::async_runtime::spawn_blocking(move || {
        let system = probe_ip_family_route(&geoip, "IPv4", &IPV4_HTTP_SOURCES, "A");
        let ipv6_probe = probe_ip_family_route(&geoip, "IPv6", &IPV6_HTTP_SOURCES, "AAAA");
        PublicIpProbe {
            system,
            ipv6: (!ipv6_probe.addresses.is_empty()).then_some(ipv6_probe),
        }
    })
    .await
    .map_err(|error| format!("公网 IP 探测任务异常结束：{error}"))
}
fn trusted_public_ipv4() -> Result<String, String> {
    let http = fetch_public_ip_from_http_sources(&IPV4_HTTP_SOURCES, "IPv4");
    let dns = fetch_public_ip_dns("A").and_then(|ip| public_ip_family(&ip, "IPv4"));
    select_trusted_public_ipv4(http.map(|(ip, _)| ip), dns)
}

fn select_trusted_public_ipv4(
    http: Result<String, String>,
    dns: Result<String, String>,
) -> Result<String, String> {
    match (http, dns) {
        (Ok(http), Ok(dns)) if http == dns => Ok(format!("{http}/32")),
        (Ok(http), Ok(dns)) => Err(format!(
            "公网 IPv4 多源探测结果不一致，已停止写入阿里云安全组：HTTP={http}，DNS={dns}"
        )),
        (Ok(http), Err(_)) => Ok(format!("{http}/32")),
        (Err(_), Ok(dns)) => Ok(format!("{dns}/32")),
        (Err(http_error), Err(dns_error)) => Err(format!(
            "所有公网 IPv4 探测源均失败，已停止写入阿里云安全组：HTTP：{http_error}；DNS：{dns_error}"
        )),
    }
}
fn local_interfaces() -> Vec<NetworkInterface> {
    let output = Command::new("/usr/sbin/networksetup")
        .arg("-listallhardwareports")
        .output()
        .ok();
    let text = output
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    let mut result = Vec::new();
    let mut kind = String::new();
    let mut device = String::new();
    for line in text.lines().chain(std::iter::once("")) {
        if let Some(value) = line.strip_prefix("Hardware Port: ") {
            kind = value.trim().to_string();
        }
        if let Some(value) = line.strip_prefix("Device: ") {
            device = value.trim().to_string();
        }
        if line.is_empty() && !device.is_empty() {
            let body = Command::new("/sbin/ifconfig")
                .arg(&device)
                .output()
                .ok()
                .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
                .unwrap_or_default();
            let addresses = body
                .lines()
                .filter_map(|line| {
                    let fields: Vec<_> = line.split_whitespace().collect();
                    if fields.first() == Some(&"inet") || fields.first() == Some(&"inet6") {
                        fields
                            .get(1)
                            .map(|x| x.split('%').next().unwrap_or(x).to_string())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            if !addresses.is_empty() {
                result.push(NetworkInterface {
                    name: device.clone(),
                    kind: kind.clone(),
                    addresses,
                });
            }
            kind.clear();
            device.clear();
        }
    }
    result
}
fn local_hostname() -> String {
    Command::new("/bin/hostname")
        .arg("-s")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "本机".into())
}
#[tauri::command]
async fn snapshot(state: State<'_, AppState>) -> Result<Snapshot, String> {
    let config = state.config.read().clone();
    let running = state.proxy.is_running();
    let logs = state.proxy.logs();
    let traffic = state.proxy.traffic();
    let message = state.message.lock().take();
    let ssh_running = state.ssh.is_running();
    let ssh_local_port = state.ssh.local_port();
    let ssh_forward_ports = state.ssh_forward.running_ports();
    let (interfaces, hostname) =
        tauri::async_runtime::spawn_blocking(|| (local_interfaces(), local_hostname()))
            .await
            .map_err(|error| format!("读取本机网络信息任务异常结束：{error}"))?;
    Ok(Snapshot {
        config,
        running,
        logs,
        traffic,
        message,
        ssh_running,
        ssh_local_port,
        interfaces,
        hostname,
        ssh_forward_ports,
    })
}
fn keychain_service() -> &'static str {
    "com.domainegress.client.ssh"
}
#[tauri::command]
async fn keychain_set(account: String, secret: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || keychain_set_blocking(account, secret))
        .await
        .map_err(|error| format!("保存钥匙串任务异常结束：{error}"))?
}

#[tauri::command]
async fn run_network_probe(
    request: network_tools::NetworkProbeRequest,
    state: State<'_, AppState>,
) -> Result<network_tools::NetworkProbeResult, String> {
    let cancel = Arc::clone(&state.network_cancel);
    cancel.store(false, std::sync::atomic::Ordering::Relaxed);
    tauri::async_runtime::spawn_blocking(move || network_tools::run(request, cancel))
        .await
        .map_err(|error| format!("网络探测任务异常结束：{error}"))
}

#[tauri::command]
fn cancel_network_probe(state: State<'_, AppState>) -> Result<(), String> {
    state
        .network_cancel
        .store(true, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
async fn list_host_mappings() -> Result<Vec<hosts::HostMapping>, String> {
    tauri::async_runtime::spawn_blocking(hosts::list)
        .await
        .map_err(|error| format!("读取本地 DNS 任务异常结束：{error}"))?
}

#[tauri::command]
async fn list_host_groups() -> Result<Vec<hosts::HostGroup>, String> {
    tauri::async_runtime::spawn_blocking(hosts::list_groups)
        .await
        .map_err(|error| format!("读取本地 DNS 分组异常结束：{error}"))?
}

#[tauri::command]
async fn read_system_hosts() -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(hosts::read_system)
        .await
        .map_err(|error| format!("读取系统 Hosts 任务异常结束：{error}"))?
}

#[tauri::command]
async fn save_host_group(
    name: String,
    content: String,
    enabled: bool,
) -> Result<Vec<hosts::HostGroup>, String> {
    tauri::async_runtime::spawn_blocking(move || hosts::save_group(name, content, enabled))
        .await
        .map_err(|error| format!("保存本地 DNS 分组异常结束：{error}"))?
}

#[tauri::command]
async fn delete_host_group(name: String) -> Result<Vec<hosts::HostGroup>, String> {
    tauri::async_runtime::spawn_blocking(move || hosts::delete_group(name))
        .await
        .map_err(|error| format!("删除本地 DNS 分组异常结束：{error}"))?
}

#[tauri::command]
async fn add_host_mapping(
    ip: String,
    domains: Vec<String>,
    group: String,
) -> Result<Vec<hosts::HostMapping>, String> {
    tauri::async_runtime::spawn_blocking(move || hosts::add(ip, domains, group))
        .await
        .map_err(|error| format!("添加本地 DNS 任务异常结束：{error}"))?
}

#[tauri::command]
async fn remove_host_mapping(
    ip: String,
    domains: Vec<String>,
) -> Result<Vec<hosts::HostMapping>, String> {
    tauri::async_runtime::spawn_blocking(move || hosts::remove(ip, domains))
        .await
        .map_err(|error| format!("删除本地 DNS 任务异常结束：{error}"))?
}

#[tauri::command]
async fn update_host_mapping(
    old_ip: String,
    old_domains: Vec<String>,
    ip: String,
    domains: Vec<String>,
    group: String,
) -> Result<Vec<hosts::HostMapping>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        hosts::update(old_ip, old_domains, ip, domains, group)
    })
    .await
    .map_err(|error| format!("修改本地 DNS 任务异常结束：{error}"))?
}
fn keychain_set_blocking(account: String, secret: String) -> Result<(), String> {
    if account.trim().is_empty() || secret.is_empty() {
        return Err("钥匙串账号和凭据不能为空".into());
    }
    let output = Command::new("/usr/bin/security")
        .args([
            "add-generic-password",
            "-a",
            &account,
            "-s",
            keychain_service(),
            "-w",
            &secret,
            "-U",
        ])
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}
#[tauri::command]
async fn keychain_delete(account: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || keychain_delete_blocking(account))
        .await
        .map_err(|error| format!("删除钥匙串任务异常结束：{error}"))?
}
fn keychain_delete_blocking(account: String) -> Result<(), String> {
    let output = Command::new("/usr/bin/security")
        .args([
            "delete-generic-password",
            "-a",
            &account,
            "-s",
            keychain_service(),
        ])
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}
#[tauri::command]
async fn list_cloud_accounts() -> Result<Vec<cloud::CloudAccountSummary>, String> {
    tauri::async_runtime::spawn_blocking(cloud::list_accounts)
        .await
        .map_err(|error| format!("读取云账号任务异常结束：{error}"))?
}
#[tauri::command]
async fn save_cloud_account(
    input: cloud::SaveCloudAccountInput,
) -> Result<cloud::CloudAccountSummary, String> {
    tauri::async_runtime::spawn_blocking(move || cloud::save_account(input))
        .await
        .map_err(|error| format!("保存云账号任务异常结束：{error}"))?
}
#[tauri::command]
async fn verify_cloud_account(id: String) -> Result<cloud::CloudAccountSummary, String> {
    tauri::async_runtime::spawn_blocking(move || cloud::verify_account(&id))
        .await
        .map_err(|error| format!("验证云账号任务异常结束：{error}"))?
}
#[tauri::command]
async fn list_cloud_regions(account_id: String) -> Result<Vec<cloud::CloudRegion>, String> {
    tauri::async_runtime::spawn_blocking(move || cloud::list_regions(&account_id))
        .await
        .map_err(|error| format!("读取云地域任务异常结束：{error}"))?
}
#[tauri::command]
async fn list_cloud_security_groups(
    account_id: String,
    region: String,
) -> Result<Vec<cloud::CloudSecurityGroup>, String> {
    tauri::async_runtime::spawn_blocking(move || cloud::list_security_groups(&account_id, &region))
        .await
        .map_err(|error| format!("读取安全组任务异常结束：{error}"))?
}
#[tauri::command]
async fn list_cloud_security_group_rules(
    account_id: String,
    region: String,
    security_group_id: String,
) -> Result<Vec<cloud::CloudSecurityRule>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        cloud::list_security_group_rules(&account_id, &region, &security_group_id)
    })
    .await
    .map_err(|error| format!("读取安全组规则任务异常结束：{error}"))?
}
#[tauri::command]
async fn list_cloud_managed_rules() -> Result<Vec<cloud::ManagedRuleConfig>, String> {
    tauri::async_runtime::spawn_blocking(cloud::list_managed_rules)
        .await
        .map_err(|error| format!("读取受管规则任务异常结束：{error}"))?
}
#[tauri::command]
async fn preview_cloud_managed_source() -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(trusted_public_ipv4)
        .await
        .map_err(|error| format!("探测授权对象任务异常结束：{error}"))?
}
#[tauri::command]
async fn create_cloud_managed_rule(
    input: cloud::CreateManagedRuleInput,
) -> Result<cloud::ManagedRuleSyncResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let source_cidr = trusted_public_ipv4()?;
        cloud::create_managed_rule(input, &source_cidr)
    })
    .await
    .map_err(|error| format!("创建受管规则任务异常结束：{error}"))?
}
#[tauri::command]
async fn sync_cloud_managed_rule(id: String) -> Result<cloud::ManagedRuleSyncResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let source_cidr = trusted_public_ipv4()?;
        cloud::sync_managed_rule(&id, &source_cidr)
    })
    .await
    .map_err(|error| format!("同步受管规则任务异常结束：{error}"))?
}
#[tauri::command]
async fn sync_all_cloud_managed_rules() -> Result<Vec<cloud::ManagedRuleSyncResult>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let source_cidr = trusted_public_ipv4()?;
        cloud::sync_all_managed_rules(&source_cidr)
    })
    .await
    .map_err(|error| format!("同步全部受管规则任务异常结束：{error}"))?
}
#[tauri::command]
async fn delete_cloud_managed_rule(id: String, revoke_remote: bool) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || cloud::delete_managed_rule(&id, revoke_remote))
        .await
        .map_err(|error| format!("删除受管规则任务异常结束：{error}"))?
}
#[tauri::command]
async fn delete_cloud_account(id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || cloud::delete_account(&id))
        .await
        .map_err(|error| format!("删除云账号任务异常结束：{error}"))?
}
fn icloud_config_path() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("." )).join("Library/Mobile Documents/iCloud~com~domainegress~client/Documents/DomainEgress/config.json")
}
#[tauri::command]
async fn icloud_status() -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(|| {
        Ok(icloud_config_path().parent().is_some_and(|p| p.exists()))
    })
    .await
    .map_err(|error| format!("检查 iCloud 状态任务异常结束：{error}"))?
}
#[tauri::command]
async fn icloud_sync(state: State<'_, AppState>) -> Result<String, String> {
    let config = state.config.read().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let path = icloud_config_path();
        let parent = path.parent().ok_or_else(|| "iCloud 目录无效".to_string())?;
        fs::create_dir_all(parent).map_err(|e| format!("无法创建 iCloud 同步目录：{e}"))?;
        let data = serde_json::to_vec_pretty(&config).map_err(|e| e.to_string())?;
        fs::write(&path, data).map_err(|e| format!("iCloud 配置写入失败：{e}"))?;
        Ok(path.display().to_string())
    })
    .await
    .map_err(|error| format!("iCloud 同步任务异常结束：{error}"))?
}
#[tauri::command]
async fn icloud_read() -> Result<Option<Config>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let path = icloud_config_path();
        if !path.exists() {
            return Ok(None);
        }
        let data = fs::read(&path).map_err(|e| e.to_string())?;
        serde_json::from_slice(&data)
            .map(Some)
            .map_err(|e| format!("iCloud 配置格式无效：{e}"))
    })
    .await
    .map_err(|error| format!("iCloud 读取任务异常结束：{error}"))?
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
    if !(1..=86400).contains(&config.port_refresh_interval_seconds) {
        return Err("监听端口刷新间隔应为 1–86400 秒".into());
    }
    if ![90, 100, 110, 125].contains(&config.font_scale) {
        return Err("文字大小仅支持 90%、100%、110% 或 125%".into());
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
        let host = rule
            .strip_prefix("*.")
            .or_else(|| rule.strip_prefix('.'))
            .unwrap_or(rule);
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
async fn save_config(config: Config, state: State<'_, AppState>) -> Result<(), String> {
    let operation = Arc::clone(&state.operation);
    let shared = Arc::clone(&state.config);
    let proxy = state.proxy.clone();
    tauri::async_runtime::spawn_blocking(move || {
        validate(&config)?;
        let _operation = operation.lock();
        let previous = shared.read().clone();
        let changed = previous.http_host != config.http_host
            || previous.http_port != config.http_port
            || previous.socks_host != config.socks_host
            || previous.socks_port != config.socks_port;
        if proxy.is_running() && changed {
            return Err("请先停止代理，再修改监听地址或端口".into());
        }
        config.save().map_err(|e| format!("配置保存失败：{e}"))?;
        *shared.write() = config;
        Ok(())
    })
    .await
    .map_err(|error| format!("保存配置任务异常结束：{error}"))?
}
#[tauri::command]
async fn set_running(running: bool, state: State<'_, AppState>) -> Result<(), String> {
    let operation = Arc::clone(&state.operation);
    let shared = Arc::clone(&state.config);
    let proxy = state.proxy.clone();
    let ssh = Arc::clone(&state.ssh);
    tauri::async_runtime::spawn_blocking(move || {
        let _operation = operation.lock();
        if running {
            validate(&shared.read())?;
            if let Some(id) = shared.read().active_ssh_profile.clone() {
                let profile = {
                    shared
                        .read()
                        .ssh_profiles
                        .iter()
                        .find(|profile| profile.id == id)
                        .cloned()
                };
                if let Some(profile) = profile {
                    let port = ssh.start(&profile).map_err(|e| e.to_string())?;
                    shared.write().ssh_proxy_port = Some(port);
                }
            }
            if let Err(error) = proxy.start() {
                let cleanup_error = ssh.stop().err();
                shared.write().ssh_proxy_port = None;
                return Err(match cleanup_error {
                    Some(cleanup_error) => {
                        format!("{error}；同时停止 SSH 代理失败：{cleanup_error}")
                    }
                    None => error.to_string(),
                });
            }
            Ok(())
        } else {
            let result = proxy.stop();
            let _ = ssh.stop();
            shared.write().ssh_proxy_port = None;
            result
        }
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|error| format!("切换代理状态任务异常结束：{error}"))??;
    update_tray_status(&state, state.proxy.is_running());
    Ok(())
}
#[tauri::command]
async fn ssh_forward_start(id: String, state: State<'_, AppState>) -> Result<u16, String> {
    let shared = Arc::clone(&state.config);
    let manager = Arc::clone(&state.ssh_forward);
    let operation = Arc::clone(&state.operation);
    tauri::async_runtime::spawn_blocking(move || {
        let _operation = operation.lock();
        let config_snapshot = shared.read().clone();
        let mut rule = config_snapshot
            .ssh_forwards
            .iter()
            .find(|rule| rule.id == id)
            .cloned()
            .ok_or_else(|| "SSH 转发配置不存在".to_string())?;
        rule.ssh_options.extend(config_snapshot.ssh_forward_options);
        let port = manager.start(&rule).map_err(|e| e.to_string())?;
        let mut config = shared.read().clone();
        if let Some(item) = config.ssh_forwards.iter_mut().find(|item| item.id == id) {
            item.local_port = Some(port);
        }
        config
            .save()
            .map_err(|e| format!("SSH 转发配置保存失败：{e}"))?;
        *shared.write() = config;
        Ok(port)
    })
    .await
    .map_err(|error| format!("启动 SSH 转发任务异常结束：{error}"))?
}
#[tauri::command]
async fn ssh_forward_stop(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let manager = Arc::clone(&state.ssh_forward);
    tauri::async_runtime::spawn_blocking(move || manager.stop(&id).map_err(|e| e.to_string()))
        .await
        .map_err(|error| format!("停止 SSH 转发任务异常结束：{error}"))?
}
fn update_tray_status(state: &AppState, running: bool) {
    if let Some(item) = state.tray_status.lock().as_ref() {
        let _ = item.set_text(if running {
            "🟢 代理运行中"
        } else {
            "⚪ 代理已停止"
        });
    }
    let menu = state.tray_menu.lock().clone();
    let start = state.tray_start.lock().clone();
    let stop = state.tray_stop.lock().clone();
    let quit = state.tray_quit.lock().clone();
    if let (Some(menu), Some(start), Some(stop), Some(quit)) = (menu, start, stop, quit) {
        let (active, inactive) = if running {
            (&stop, &start)
        } else {
            (&start, &stop)
        };
        let _ = menu.remove(inactive);
        let _ = menu.remove(active);
        let _ = menu.remove(&quit);
        let _ = menu.append(active);
        let _ = menu.append(&quit);
    }
    if let Some(icon) = state.tray_icon.lock().as_ref() {
        let bytes: &[u8] = if running {
            &include_bytes!("../icons/tray-running.png")[..]
        } else {
            &include_bytes!("../icons/tray-stopped.png")[..]
        };
        if let Ok(image) = tauri::image::Image::from_bytes(bytes) {
            let _ = icon.set_icon(Some(image));
        }
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
    if gist_id.trim().is_empty() {
        return Err("请填写 Gist ID".into());
    }
    match provider {
        "github" => Ok(format!("https://api.github.com/gists/{}", gist_id.trim())),
        "gitee" => Ok(format!("https://gitee.com/api/v5/gists/{}", gist_id.trim())),
        _ => Err("不支持的 Gist 服务商".into()),
    }
}
fn gist_request(
    url: &str,
    token: &str,
    method: &str,
    body: Option<String>,
) -> Result<serde_json::Value, String> {
    let mut command = Command::new("/usr/bin/curl");
    command.args(["-sS", "-f", "-X", method, "-H", "Accept: application/json"]);
    if !token.trim().is_empty() {
        command.args(["-H", &format!("Authorization: token {}", token.trim())]);
    }
    if let Some(body) = body {
        command.args(["-H", "Content-Type: application/json", "--data-raw", &body]);
    }
    let output = command
        .arg(url)
        .output()
        .map_err(|e| format!("请求 Gist 失败：{e}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    serde_json::from_slice(&output.stdout).map_err(|e| format!("Gist 返回格式无效：{e}"))
}
#[tauri::command]
async fn gist_pull(
    provider: String,
    gist_id: String,
    file_name: String,
    token: String,
) -> Result<GistRules, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let response = gist_request(&gist_api_url(&provider, &gist_id)?, &token, "GET", None)?;
        let file = response
            .get("files")
            .and_then(|files| files.get(&file_name))
            .and_then(|file| file.get("content"))
            .and_then(|content| content.as_str())
            .ok_or_else(|| format!("Gist 中未找到文件：{file_name}"))?;
        serde_json::from_str(file).map_err(|e| format!("规则文件格式无效：{e}"))
    })
    .await
    .map_err(|error| format!("拉取 Gist 任务异常结束：{error}"))?
}
#[tauri::command]
async fn gist_push(
    provider: String,
    gist_id: String,
    file_name: String,
    token: String,
    config: Config,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        if token.trim().is_empty() {
            return Err("推送 Gist 需要访问令牌".into());
        }
        let rules = GistRules {
            format: "domain-egress-rules".into(),
            version: 1,
            whitelist: config.whitelist,
            blacklist: config.blacklist,
        };
        let content = serde_json::to_string_pretty(&rules).map_err(|e| e.to_string())?;
        let body =
            serde_json::json!({ "files": { file_name: { "content": content } } }).to_string();
        gist_request(
            &gist_api_url(&provider, &gist_id)?,
            &token,
            "PATCH",
            Some(body),
        )
        .map(|_| ())
    })
    .await
    .map_err(|error| format!("推送 Gist 任务异常结束：{error}"))?
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
        .plugin(tauri_plugin_autostart::Builder::new().build())
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
            let ssh = Arc::new(ssh::SshManager::new());
            let ssh_forward = Arc::new(ssh_forward::SshForwardManager::new());
            let geoip =
                GeoIpDatabases::load(&app.path().resource_dir().unwrap_or_else(|_| {
                    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources")
                }));
            if config.auto_start {
                for rule in config.ssh_forwards.iter().filter(|rule| rule.auto_start) {
                    let mut effective = rule.clone();
                    effective
                        .ssh_options
                        .extend(config.ssh_forward_options.clone());
                    if let Err(error) = ssh_forward.start(&effective) {
                        message = Some(format!("SSH 转发 {} 自动启动失败：{error}", rule.name));
                    }
                }
            }
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
                ssh_forward,
                operation: Arc::new(Mutex::new(())),
                tray_status: Mutex::new(None),
                tray_icon: Mutex::new(None),
                tray_menu: Mutex::new(None),
                tray_start: Mutex::new(None),
                tray_stop: Mutex::new(None),
                tray_quit: Mutex::new(None),
                message: Mutex::new(message),
                geoip: Arc::new(geoip),
                network_cancel: Arc::new(std::sync::atomic::AtomicBool::new(false)),
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
                    "🟢 代理运行中"
                } else {
                    "⚪ 代理已停止"
                },
                false,
                None::<&str>,
            )?;
            let start = MenuItem::with_id(app, "start", "启动代理", true, None::<&str>)?;
            let stop = MenuItem::with_id(app, "stop", "停止代理", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &status, &start, &stop, &quit])?;
            *app.state::<AppState>().tray_status.lock() = Some(status.clone());
            *app.state::<AppState>().tray_menu.lock() = Some(menu.clone());
            *app.state::<AppState>().tray_start.lock() = Some(start.clone());
            *app.state::<AppState>().tray_stop.lock() = Some(stop.clone());
            *app.state::<AppState>().tray_quit.lock() = Some(quit.clone());
            let initially_running = app.state::<AppState>().proxy.is_running();
            if initially_running {
                let _ = menu.remove(&start);
            } else {
                let _ = menu.remove(&stop);
            }
            let initial_icon: &[u8] = if app.state::<AppState>().proxy.is_running() {
                &include_bytes!("../icons/tray-running.png")[..]
            } else {
                &include_bytes!("../icons/tray-stopped.png")[..]
            };
            let tray = TrayIconBuilder::new()
                .icon(tauri::image::Image::from_bytes(initial_icon)?)
                .icon_as_template(true)
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
                            let running = event.id.as_ref() == "start";
                            let app = app.clone();
                            tauri::async_runtime::spawn(async move {
                                let state = app.state::<AppState>();
                                if let Err(error) = set_running(running, state.clone()).await {
                                    *state.message.lock() = Some(error);
                                    if let Some(window) = app.get_webview_window("main") {
                                        let _ = window.show();
                                    }
                                }
                            });
                        }
                        "quit" => {
                            let _ = state.proxy.stop();
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .build(app)?;
            *app.state::<AppState>().tray_icon.lock() = Some(tray);
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
            ssh_forward_start,
            ssh_forward_stop,
            clear_logs,
            gist_pull,
            gist_push,
            list_ports,
            terminate_process,
            keychain_set,
            keychain_delete,
            run_network_probe,
            cancel_network_probe,
            list_host_mappings,
            list_host_groups,
            read_system_hosts,
            save_host_group,
            delete_host_group,
            add_host_mapping,
            remove_host_mapping,
            update_host_mapping,
            list_cloud_accounts,
            save_cloud_account,
            verify_cloud_account,
            list_cloud_regions,
            list_cloud_security_groups,
            list_cloud_security_group_rules,
            list_cloud_managed_rules,
            preview_cloud_managed_source,
            create_cloud_managed_rule,
            sync_cloud_managed_rule,
            sync_all_cloud_managed_rules,
            delete_cloud_managed_rule,
            delete_cloud_account,
            icloud_status,
            icloud_sync,
            icloud_read
        ])
        .build(tauri::generate_context!())
        .expect("启动 DomainEgress 失败")
        .run(|app, event| match event {
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { .. } => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            tauri::RunEvent::Exit => {
                let _ = app.state::<AppState>().proxy.stop();
                app.state::<AppState>().ssh_forward.stop_all();
            }
            _ => {}
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn config_validation() {
        let mut config = Config::default();
        assert!(validate(&config).is_ok());
        assert!(!config.auto_start);
        assert_eq!(config.port_refresh_interval_seconds, 30);
        config.port_refresh_interval_seconds = 0;
        assert!(validate(&config).is_err());
        config.port_refresh_interval_seconds = 30;
        config.whitelist.push("https://example.com/path".into());
        assert!(validate(&config).is_err());
        config.whitelist = vec!["*.example.com".into(), "::1".into()];
        assert!(validate(&config).is_ok());
        config.font_scale = 105;
        assert!(validate(&config).is_err());
        config.font_scale = 110;
        assert!(validate(&config).is_ok());
        config.socks_port = config.http_port;
        assert!(validate(&config).is_err());
    }

    #[test]
    fn parses_ip2region_location_and_discards_reserved_records() {
        let location = parse_ip2region_location("中国|广东省|深圳市|电信|CN").unwrap();
        assert_eq!(location.country, "中国");
        assert_eq!(location.city, "深圳市");
        assert_eq!(location.country_code, "CN");
        assert_eq!(location.confidence, "离线兜底");
        assert!(parse_ip2region_location("0|0|Reserved|Reserved|Reserved").is_none());
    }

    #[test]
    fn public_ip_family_rejects_wrong_address_family() {
        assert!(public_ip_family("8.8.8.8", "IPv4").is_ok());
        assert!(public_ip_family("2001:4860:4860::8888", "IPv6").is_ok());
        assert!(public_ip_family("8.8.8.8", "IPv6").is_err());
        assert!(public_ip_family("2001:4860:4860::8888", "IPv4").is_err());
    }

    #[test]
    fn trusted_public_ipv4_accepts_any_available_source() {
        assert_eq!(
            select_trusted_public_ipv4(Ok("8.8.8.8".into()), Err("dns failed".into())).unwrap(),
            "8.8.8.8/32"
        );
        assert_eq!(
            select_trusted_public_ipv4(Err("http failed".into()), Ok("1.1.1.1".into())).unwrap(),
            "1.1.1.1/32"
        );
    }

    #[test]
    fn trusted_public_ipv4_rejects_conflicting_or_missing_sources() {
        assert!(select_trusted_public_ipv4(Ok("8.8.8.8".into()), Ok("1.1.1.1".into())).is_err());
        assert!(
            select_trusted_public_ipv4(Err("http failed".into()), Err("dns failed".into()))
                .is_err()
        );
    }
}
