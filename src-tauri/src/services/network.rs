use crate::types::*;
use std::{path::PathBuf, process::Command};
pub(crate) struct GeoIpDatabases {
    pub(crate) ipv4: Option<ip2region::Searcher>,
    pub(crate) ipv6: Option<ip2region::Searcher>,
}
#[derive(Clone)]
struct OnlineGeoLocation {
    pub(crate) country: String,
    pub(crate) country_code: String,
    pub(crate) region: String,
    pub(crate) city: String,
    pub(crate) isp: String,
    pub(crate) source: &'static str,
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
    pub(crate) fn load(resource_dir: &std::path::Path) -> Self {
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
pub(crate) fn probe_public_addresses(geoip: &GeoIpDatabases) -> PublicIpProbe {
    let system = probe_ip_family_route(geoip, "IPv4", &IPV4_HTTP_SOURCES, "A");
    let ipv6_probe = probe_ip_family_route(geoip, "IPv6", &IPV6_HTTP_SOURCES, "AAAA");
    PublicIpProbe {
        system,
        ipv6: (!ipv6_probe.addresses.is_empty()).then_some(ipv6_probe),
    }
}
pub(crate) fn trusted_public_ipv4() -> Result<String, String> {
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
pub(crate) fn local_interfaces() -> Vec<NetworkInterface> {
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
pub(crate) fn local_hostname() -> String {
    Command::new("/bin/hostname")
        .arg("-s")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "本机".into())
}

#[cfg(test)]
mod tests {
    use super::*;
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
