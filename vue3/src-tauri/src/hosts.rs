use base64::Engine;
use serde::Serialize;
use std::{fs, net::IpAddr, process::Command};

const BEGIN: &str = "# BEGIN DOMAIN-EGRESS HOSTS";
const END: &str = "# END DOMAIN-EGRESS HOSTS";
const GROUP_PREFIX: &str = "# DomainEgress Group:";

#[derive(Clone, Serialize)]
pub struct HostMapping {
    pub ip: String,
    pub domains: Vec<String>,
    pub group: String,
}

#[derive(Clone, Serialize)]
pub struct HostGroup {
    pub name: String,
    pub enabled: bool,
    pub content: String,
}

fn validate_group(value: &str) -> Result<String, String> {
    let group = value.trim();
    if group.len() > 80 || group.chars().any(|c| matches!(c, '\r' | '\n' | '#')) {
        return Err("分组名称不能超过 80 个字符，且不能包含换行或 #".into());
    }
    Ok(group.into())
}

fn group_marker(line: &str) -> Option<(String, bool)> {
    let value = line.trim().strip_prefix(GROUP_PREFIX)?.trim();
    let enabled = !value.ends_with("[disabled]");
    let name = value.trim_end_matches("[disabled]").trim();
    Some((name.into(), enabled))
}

fn parse_groups(content: &str) -> Result<(Vec<String>, Vec<HostGroup>), String> {
    let mut before = Vec::new();
    let mut groups: Vec<HostGroup> = Vec::new();
    let mut in_block = false;
    let mut current: Option<usize> = None;
    for line in content.lines() {
        if line.trim() == BEGIN {
            in_block = true;
            continue;
        }
        if line.trim() == END {
            in_block = false;
            current = None;
            continue;
        }
        if !in_block {
            before.push(line.to_string());
            continue;
        }
        if let Some((name, enabled)) = group_marker(line) {
            let name = validate_group(&name)?;
            groups.push(HostGroup {
                name,
                enabled,
                content: String::new(),
            });
            current = Some(groups.len() - 1);
            continue;
        }
        if current.is_none() {
            groups.push(HostGroup {
                name: "未分组".into(),
                enabled: true,
                content: String::new(),
            });
            current = Some(groups.len() - 1);
        }
        let group = &mut groups[current.unwrap()];
        let line = if !group.enabled {
            line.strip_prefix("# ").unwrap_or(line)
        } else {
            line
        };
        if !group.content.is_empty() {
            group.content.push('\n');
        }
        group.content.push_str(line);
    }
    if in_block {
        return Err("/etc/hosts 的 DomainEgress 区块未闭合".into());
    }
    Ok((before, groups))
}

fn render_groups(before: &[String], groups: &[HostGroup]) -> String {
    let mut lines = before.to_vec();
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    lines.push(String::new());
    lines.push(BEGIN.into());
    for group in groups {
        lines.push(format!(
            "{GROUP_PREFIX} {}{}",
            group.name,
            if group.enabled { "" } else { " [disabled]" }
        ));
        for line in group.content.lines() {
            if group.enabled || line.trim().is_empty() {
                lines.push(line.into());
            } else {
                lines.push(format!("# {line}"));
            }
        }
    }
    lines.push(END.into());
    lines.push(String::new());
    lines.join("\n")
}

pub fn list_groups() -> Result<Vec<HostGroup>, String> {
    Ok(parse_groups(&read_hosts()?)?.1)
}

pub fn read_system() -> Result<String, String> {
    read_hosts()
}

pub fn save_group(name: String, content: String, enabled: bool) -> Result<Vec<HostGroup>, String> {
    let name = validate_group(&name)?;
    if name.is_empty() {
        return Err("分组名称不能为空".into());
    }
    let (before, mut groups) = parse_groups(&read_hosts()?)?;
    if let Some(group) = groups.iter_mut().find(|group| group.name == name) {
        group.content = content;
        group.enabled = enabled;
    } else {
        groups.push(HostGroup {
            name,
            enabled,
            content,
        });
    }
    write_privileged(render_groups(&before, &groups))?;
    Ok(groups)
}

pub fn delete_group(name: String) -> Result<Vec<HostGroup>, String> {
    let (before, mut groups) = parse_groups(&read_hosts()?)?;
    let count = groups.len();
    groups.retain(|group| group.name != name);
    if count == groups.len() {
        return Err("未找到要删除的本地 DNS 分组".into());
    }
    write_privileged(render_groups(&before, &groups))?;
    Ok(groups)
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
    let mut group = String::new();
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
            if let Some(value) = line.trim().strip_prefix(GROUP_PREFIX) {
                group = validate_group(value)?;
                continue;
            }
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
                    group: group.clone(),
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
    let mut current_group = String::new();
    for mapping in mappings {
        if mapping.group != current_group {
            lines.push(format!("{GROUP_PREFIX} {}", mapping.group));
            current_group = mapping.group.clone();
        }
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

pub fn add(ip: String, domains: Vec<String>, group: String) -> Result<Vec<HostMapping>, String> {
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
    let group = validate_group(&group)?;
    let (before, mut mappings) = parse(&read_hosts()?)?;
    let used = mappings
        .iter()
        .flat_map(|m| m.domains.iter())
        .collect::<std::collections::HashSet<_>>();
    if domains.iter().any(|d| used.contains(d)) {
        return Err("域名已存在于 DomainEgress hosts 区块".into());
    }
    mappings.push(HostMapping { ip, domains, group });
    write_privileged(render(&before, &mappings))?;
    Ok(mappings)
}

pub fn update(
    old_ip: String,
    old_domains: Vec<String>,
    ip: String,
    domains: Vec<String>,
    group: String,
) -> Result<Vec<HostMapping>, String> {
    let old_ip = old_ip
        .trim()
        .parse::<IpAddr>()
        .map_err(|_| "原 IP 地址无效".to_string())?
        .to_string();
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
    if domains
        .iter()
        .collect::<std::collections::HashSet<_>>()
        .len()
        != domains.len()
    {
        return Err("域名不能重复".into());
    }
    let group = validate_group(&group)?;
    let old_domains = old_domains
        .into_iter()
        .map(|d| validate_domain(&d))
        .collect::<Result<Vec<_>, _>>()?;
    let (before, mut mappings) = parse(&read_hosts()?)?;
    let index = mappings
        .iter()
        .position(|mapping| mapping.ip == old_ip && mapping.domains == old_domains)
        .ok_or("未找到要修改的 DomainEgress hosts 映射")?;
    let used = mappings
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != index)
        .flat_map(|(_, mapping)| mapping.domains.iter())
        .collect::<std::collections::HashSet<_>>();
    if domains.iter().any(|domain| used.contains(domain)) {
        return Err("域名已存在于其他 DomainEgress hosts 映射".into());
    }
    mappings[index] = HostMapping { ip, domains, group };
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
        assert_eq!(mappings[0].group, "");
    }

    #[test]
    fn parses_and_renders_custom_groups() {
        let text = "# BEGIN DOMAIN-EGRESS HOSTS\n# DomainEgress Group: development\n127.0.0.1 one.example\n# DomainEgress Group: production\n::1 two.example\n# END DOMAIN-EGRESS HOSTS\n";
        let (_, mappings) = parse(text).unwrap();
        assert_eq!(mappings[0].group, "development");
        assert_eq!(mappings[1].group, "production");
        let rendered = render(&[], &mappings);
        assert!(rendered.contains("# DomainEgress Group: development"));
        assert!(rendered.contains("# DomainEgress Group: production"));
    }
}
