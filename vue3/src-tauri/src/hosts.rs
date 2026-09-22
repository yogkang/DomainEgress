use base64::Engine;
use serde::Serialize;
use std::{fs, net::IpAddr, process::Command};

const BEGIN: &str = "# BEGIN DOMAIN-EGRESS HOSTS";
const END: &str = "# END DOMAIN-EGRESS HOSTS";

#[derive(Clone, Serialize)]
pub struct HostMapping {
    pub ip: String,
    pub domains: Vec<String>,
}

fn validate_domain(value: &str) -> Result<String, String> {
    let domain = value.trim().trim_end_matches('.').to_ascii_lowercase();
    if domain.is_empty() || domain.len() > 253 || domain == "localhost" {
        return Err(format!("无效域名：{value}"));
    }
    if !domain.split('.').all(|part| {
        !part.is_empty()
            && part.len() <= 63
            && !part.starts_with('-')
            && !part.ends_with('-')
            && part.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
    }) {
        return Err(format!("无效域名：{value}"));
    }
    Ok(domain)
}

fn parse(content: &str) -> Result<(Vec<String>, Vec<HostMapping>), String> {
    let mut before = Vec::new();
    let mut managed = Vec::new();
    let mut in_block = false;
    for line in content.lines() {
        if line.trim() == BEGIN {
            if in_block {
                return Err("/etc/hosts 存在重复的 DomainEgress 区块".into());
            }
            in_block = true;
            continue;
        }
        if line.trim() == END {
            if !in_block {
                return Err("/etc/hosts 的 DomainEgress 结束标记不匹配".into());
            }
            in_block = false;
            continue;
        }
        if in_block {
            let line = line.split('#').next().unwrap_or_default();
            let mut parts = line.split_whitespace();
            let Some(ip) = parts.next() else { continue };
            ip.parse::<IpAddr>()
                .map_err(|_| format!("hosts 区块中的 IP 无效：{ip}"))?;
            let domains = parts.map(validate_domain).collect::<Result<Vec<_>, _>>()?;
            if !domains.is_empty() {
                managed.push(HostMapping {
                    ip: ip.into(),
                    domains,
                });
            }
        } else {
            before.push(line.to_string());
        }
    }
    if in_block {
        return Err("/etc/hosts 的 DomainEgress 区块未闭合".into());
    }
    Ok((before, managed))
}

fn read_hosts() -> Result<String, String> {
    fs::read_to_string("/etc/hosts").map_err(|e| format!("读取 /etc/hosts 失败：{e}"))
}

fn render(before: &[String], mappings: &[HostMapping]) -> String {
    let mut lines = before.to_vec();
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    lines.push(String::new());
    lines.push(BEGIN.into());
    for mapping in mappings {
        lines.push(format!("{} {}", mapping.ip, mapping.domains.join(" ")));
    }
    lines.push(END.into());
    lines.push(String::new());
    lines.join("\n")
}

pub fn list() -> Result<Vec<HostMapping>, String> {
    Ok(parse(&read_hosts()?)?.1)
}

fn write_privileged(content: String) -> Result<(), String> {
    let encoded = base64::engine::general_purpose::STANDARD.encode(content.as_bytes());
    let temp = format!("/tmp/domain-egress-hosts-{}", uuid::Uuid::new_v4());
    let script = format!(
        "/usr/bin/printf '%s' '{}' | /usr/bin/base64 --decode > '{}' && /bin/mv '{}' /etc/hosts && /usr/bin/dscacheutil -flushcache && /usr/bin/killall -HUP mDNSResponder 2>/dev/null || true",
        encoded, temp, temp
    );
    let output = Command::new("/usr/bin/osascript")
        .args([
            "-e",
            &format!("do shell script {:?} with administrator privileges", script),
        ])
        .output()
        .map_err(|e| format!("请求管理员授权失败：{e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr)
            .trim()
            .to_string()
            .if_empty_then("管理员取消了 /etc/hosts 修改"))
    }
}

trait EmptyFallback {
    fn if_empty_then(self, fallback: &str) -> String;
}
impl EmptyFallback for String {
    fn if_empty_then(self, fallback: &str) -> String {
        if self.is_empty() {
            fallback.into()
        } else {
            self
        }
    }
}

pub fn add(ip: String, domains: Vec<String>) -> Result<Vec<HostMapping>, String> {
    let ip = ip
        .trim()
        .parse::<IpAddr>()
        .map_err(|_| "IP 地址无效".to_string())?
        .to_string();
    let domains = domains
        .into_iter()
        .map(|d| validate_domain(&d))
        .collect::<Result<Vec<_>, _>>()?;
    if domains.is_empty() || domains.len() > 64 {
        return Err("至少需要一个域名，最多支持 64 个域名".into());
    }
    let unique = domains.iter().collect::<std::collections::HashSet<_>>();
    if unique.len() != domains.len() {
        return Err("域名不能重复".into());
    }
    let (before, mut mappings) = parse(&read_hosts()?)?;
    let used = mappings
        .iter()
        .flat_map(|m| m.domains.iter())
        .collect::<std::collections::HashSet<_>>();
    if domains.iter().any(|d| used.contains(d)) {
        return Err("域名已存在于 DomainEgress hosts 区块".into());
    }
    mappings.push(HostMapping { ip, domains });
    write_privileged(render(&before, &mappings))?;
    Ok(mappings)
}

pub fn remove(ip: String, domains: Vec<String>) -> Result<Vec<HostMapping>, String> {
    let (before, mut mappings) = parse(&read_hosts()?)?;
    let before_len = mappings.len();
    mappings.retain(|m| !(m.ip == ip && (domains.is_empty() || m.domains == domains)));
    if before_len == mappings.len() {
        return Err("未找到要删除的 DomainEgress hosts 映射".into());
    }
    write_privileged(render(&before, &mappings))?;
    Ok(mappings)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_external_lines_and_parses_managed_block() {
        let text = "127.0.0.1 localhost\n\n# BEGIN DOMAIN-EGRESS HOSTS\n::1 one.example two.example\n# END DOMAIN-EGRESS HOSTS\n";
        let (before, mappings) = parse(text).unwrap();
        assert_eq!(before[0], "127.0.0.1 localhost");
        assert_eq!(mappings[0].domains, vec!["one.example", "two.example"]);
    }
}
