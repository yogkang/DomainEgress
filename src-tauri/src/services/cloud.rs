use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::Utc;
use hmac::{Hmac, Mac};
use keyring::Entry;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use std::{
    collections::HashSet,
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

const CLOUD_KEYCHAIN_SERVICE: &str = "com.domainegress.client.cloud";
const MANAGED_RULE_PREFIX: &str = "DomainEgress:v1:";
static STORE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static CLOUD_OPERATION_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CloudAccountSummary {
    pub id: String,
    pub provider: String,
    pub display_name: String,
    pub auth_method: String,
    pub access_key_hint: String,
    pub verified_account_id: Option<String>,
    pub verification_status: String,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct StoredCloudAccount {
    #[serde(flatten)]
    summary: CloudAccountSummary,
    credential_key: String,
}

#[derive(Debug, Deserialize)]
pub struct SaveCloudAccountInput {
    pub display_name: String,
    pub auth_method: String,
    pub access_key_id: String,
    pub access_key_secret: String,
}

#[derive(Serialize)]
struct CloudCredentials<'a> {
    access_key_id: &'a str,
    access_key_secret: &'a str,
}

#[derive(Deserialize)]
struct StoredCredentials {
    access_key_id: String,
    access_key_secret: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct AliyunIdentity {
    account_id: String,
    arn: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct CloudRegion {
    pub id: String,
    pub name: String,
    pub endpoint: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct CloudSecurityGroup {
    pub id: String,
    pub name: String,
    pub vpc_id: String,
    pub group_type: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct CloudSecurityRule {
    pub id: String,
    pub direction: String,
    pub protocol: String,
    pub port_range: String,
    pub priority: i32,
    pub action: String,
    pub source_cidr: String,
    pub description: String,
    pub managed: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManagedRuleConfig {
    pub id: String,
    pub account_id: String,
    pub region: String,
    pub security_group_id: String,
    pub protocol: String,
    pub port_range: String,
    pub priority: i32,
    pub description: String,
    pub enabled: bool,
    pub last_source_cidr: Option<String>,
    pub last_synced_at: Option<u64>,
    pub last_error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateManagedRuleInput {
    pub account_id: String,
    pub region: String,
    pub security_group_id: String,
    pub protocol: String,
    pub port_range: String,
    pub priority: i32,
}

#[derive(Clone, Debug, Serialize)]
pub struct ManagedRuleSyncResult {
    pub config: ManagedRuleConfig,
    pub action: String,
    pub rule_id: String,
}

fn path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("DomainEgress/cloud-accounts.json")
}
fn managed_rules_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("DomainEgress/cloud-managed-rules.json")
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn store_lock() -> &'static Mutex<()> {
    STORE_LOCK.get_or_init(|| Mutex::new(()))
}
fn cloud_operation_lock() -> &'static Mutex<()> {
    CLOUD_OPERATION_LOCK.get_or_init(|| Mutex::new(()))
}

fn protect(path: &PathBuf, mode: u32) -> Result<(), String> {
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
        .map_err(|error| format!("设置云账号文件权限失败：{error}"))
}

fn load() -> Result<Vec<StoredCloudAccount>, String> {
    let path = path();
    if !path.exists() {
        return Ok(vec![]);
    }
    protect(&path, 0o600)?;
    serde_json::from_slice(&fs::read(&path).map_err(|error| format!("读取云账号失败：{error}"))?)
        .map_err(|error| format!("云账号数据格式无效：{error}"))
}

fn save(accounts: &[StoredCloudAccount]) -> Result<(), String> {
    let path = path();
    let parent = path.parent().ok_or_else(|| "云账号目录无效".to_string())?;
    fs::create_dir_all(parent).map_err(|error| format!("创建云账号目录失败：{error}"))?;
    protect(&parent.to_path_buf(), 0o700)?;
    let temporary = parent.join(format!(".cloud-accounts-{}.tmp", Uuid::new_v4()));
    fs::write(
        &temporary,
        serde_json::to_vec_pretty(accounts).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("保存云账号失败：{error}"))?;
    protect(&temporary, 0o600)?;
    fs::rename(&temporary, &path).map_err(|error| format!("提交云账号失败：{error}"))?;
    protect(&path, 0o600)
}

fn load_managed_rules() -> Result<Vec<ManagedRuleConfig>, String> {
    let path = managed_rules_path();
    if !path.exists() {
        return Ok(vec![]);
    }
    protect(&path, 0o600)?;
    serde_json::from_slice(&fs::read(&path).map_err(|error| format!("读取受管规则失败：{error}"))?)
        .map_err(|error| format!("受管规则数据格式无效：{error}"))
}

fn save_managed_rules(rules: &[ManagedRuleConfig]) -> Result<(), String> {
    let path = managed_rules_path();
    let parent = path
        .parent()
        .ok_or_else(|| "受管规则目录无效".to_string())?;
    fs::create_dir_all(parent).map_err(|error| format!("创建受管规则目录失败：{error}"))?;
    protect(&parent.to_path_buf(), 0o700)?;
    let temporary = parent.join(format!(".cloud-managed-rules-{}.tmp", Uuid::new_v4()));
    fs::write(
        &temporary,
        serde_json::to_vec_pretty(rules).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("保存受管规则失败：{error}"))?;
    protect(&temporary, 0o600)?;
    fs::rename(&temporary, &path).map_err(|error| format!("提交受管规则失败：{error}"))?;
    protect(&path, 0o600)
}

fn keychain_entry(account: &str) -> Result<Entry, String> {
    Entry::new(CLOUD_KEYCHAIN_SERVICE, account)
        .map_err(|error| format!("访问 macOS 钥匙串失败：{error}"))
}
fn keychain_store(account: &str, secret: &str) -> Result<(), String> {
    keychain_entry(account)?
        .set_password(secret)
        .map_err(|error| format!("写入 macOS 钥匙串失败：{error}"))
}
fn keychain_delete(account: &str) -> Result<(), String> {
    keychain_entry(account)?
        .delete_credential()
        .map_err(|error| format!("从 macOS 钥匙串删除凭据失败：{error}"))
}

fn percent_encode(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (*byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

fn signed_query(
    access_key_id: &str,
    access_key_secret: &str,
    action: &str,
    version: &str,
) -> Result<String, String> {
    signed_query_with_extra(access_key_id, access_key_secret, action, version, &[])
}

fn signed_query_with_extra(
    access_key_id: &str,
    access_key_secret: &str,
    action: &str,
    version: &str,
    extra: &[(&str, String)],
) -> Result<String, String> {
    signed_query_with_parts(
        access_key_id,
        access_key_secret,
        action,
        version,
        extra,
        &Uuid::new_v4().to_string(),
        &Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
    )
}

fn signed_query_with_parts(
    access_key_id: &str,
    access_key_secret: &str,
    action: &str,
    version: &str,
    extra: &[(&str, String)],
    nonce: &str,
    timestamp: &str,
) -> Result<String, String> {
    let mut parameters = vec![
        ("AccessKeyId", access_key_id.to_string()),
        ("Action", action.to_string()),
        ("Format", "JSON".to_string()),
        ("SignatureMethod", "HMAC-SHA1".to_string()),
        ("SignatureNonce", nonce.to_string()),
        ("SignatureVersion", "1.0".to_string()),
        ("Timestamp", timestamp.to_string()),
        ("Version", version.to_string()),
    ];
    parameters.extend(extra.iter().map(|(key, value)| (*key, value.clone())));
    parameters.sort();
    let canonical = parameters
        .iter()
        .map(|(key, value)| format!("{}={}", percent_encode(key), percent_encode(value)))
        .collect::<Vec<_>>()
        .join("&");
    let string_to_sign = format!("GET&%2F&{}", percent_encode(&canonical));
    let mut signer = Hmac::<Sha1>::new_from_slice(format!("{access_key_secret}&").as_bytes())
        .map_err(|_| "生成阿里云请求签名失败".to_string())?;
    signer.update(string_to_sign.as_bytes());
    let signature = STANDARD.encode(signer.finalize().into_bytes());
    Ok(format!(
        "{canonical}&Signature={}",
        percent_encode(&signature)
    ))
}

fn verify_identity(access_key_id: &str, access_key_secret: &str) -> Result<AliyunIdentity, String> {
    let query = signed_query(
        access_key_id,
        access_key_secret,
        "GetCallerIdentity",
        "2015-04-01",
    )?;
    let response = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|_| "初始化阿里云网络客户端失败".to_string())?
        .get(format!("https://sts.aliyuncs.com/?{query}"))
        .send()
        .map_err(|_| "阿里云身份校验请求失败，请检查网络或代理。".to_string())?;
    if !response.status().is_success() {
        return Err("阿里云身份校验失败，请检查 RAM AccessKey、网络及 STS 权限。".into());
    }
    let identity: AliyunIdentity = response
        .json()
        .map_err(|_| "阿里云身份校验返回无效，请检查账号与网络。".to_string())?;
    if identity.account_id.is_empty() || identity.arn.is_empty() {
        return Err("阿里云身份校验未返回完整账号信息。".into());
    }
    Ok(identity)
}

fn ecs_rpc(
    access_key_id: &str,
    access_key_secret: &str,
    endpoint: &str,
    action: &str,
    extra: &[(&str, String)],
) -> Result<serde_json::Value, String> {
    let query = signed_query_with_extra(
        access_key_id,
        access_key_secret,
        action,
        "2014-05-26",
        extra,
    )?;
    let response = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|_| "初始化阿里云网络客户端失败".to_string())?
        .get(format!("https://{endpoint}/?{query}"))
        .send()
        .map_err(|_| "阿里云 ECS 请求失败，请检查网络或代理。".to_string())?;
    let status = response.status();
    let response_text = response
        .text()
        .map_err(|_| format!("阿里云 ECS {action} 返回无法读取"))?;
    let body: serde_json::Value = match serde_json::from_str(&response_text) {
        Ok(body) => body,
        Err(_) if !status.is_success() => {
            return Err(format!(
                "阿里云 ECS {action} 失败（HTTP {status}，响应不是有效 JSON）"
            ));
        }
        Err(_) => return Err(format!("阿里云 ECS {action} 返回无效，请稍后重试。")),
    };
    if !status.is_success() || body.get("Code").is_some() {
        let code = value_string(&body, "Code");
        let message = value_string(&body, "Message");
        let request_id = value_string(&body, "RequestId");
        return Err(format!(
            "阿里云 ECS {action} 失败（HTTP {status}，Code={}，Message={}，RequestId={}）",
            if code.is_empty() { "Unknown" } else { &code },
            if message.is_empty() {
                "未返回"
            } else {
                &message
            },
            if request_id.is_empty() {
                "未返回"
            } else {
                &request_id
            }
        ));
    }
    Ok(body)
}

fn verified_credentials(account_id: &str) -> Result<StoredCredentials, String> {
    let credential_key = {
        let _guard = store_lock().lock();
        let accounts = load()?;
        let account = accounts
            .iter()
            .find(|account| account.summary.id == account_id)
            .ok_or_else(|| "云账号不存在".to_string())?;
        if account.summary.verification_status != "已验证" {
            return Err("请先验证阿里云账号".into());
        }
        account.credential_key.clone()
    };
    read_credentials(&credential_key)
}

fn value_string(value: &serde_json::Value, key: &str) -> String {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string()
}

pub fn list_regions(account_id: &str) -> Result<Vec<CloudRegion>, String> {
    let credentials = verified_credentials(account_id)?;
    let response = ecs_rpc(
        &credentials.access_key_id,
        &credentials.access_key_secret,
        "ecs.aliyuncs.com",
        "DescribeRegions",
        &[
            ("AcceptLanguage", "zh-CN".into()),
            ("ResourceType", "instance".into()),
        ],
    )?;
    let regions = response
        .pointer("/Regions/Region")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "阿里云地域响应缺少 Regions.Region".to_string())?;
    Ok(regions
        .iter()
        .filter_map(|value| {
            let id = value_string(value, "RegionId");
            (!id.is_empty()).then(|| CloudRegion {
                name: {
                    let local_name = value_string(value, "LocalName");
                    if local_name.is_empty() {
                        id.clone()
                    } else {
                        local_name
                    }
                },
                endpoint: value_string(value, "RegionEndpoint"),
                id,
            })
        })
        .collect())
}

fn validated_region(account_id: &str, region: &str) -> Result<CloudRegion, String> {
    list_regions(account_id)?
        .into_iter()
        .find(|item| item.id == region)
        .ok_or_else(|| "所选地域不在该账号可用地域列表中".to_string())
}

pub fn list_security_groups(
    account_id: &str,
    region: &str,
) -> Result<Vec<CloudSecurityGroup>, String> {
    if region.is_empty()
        || !region.is_ascii()
        || !region
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
    {
        return Err("地域标识无效".into());
    }
    let region_info = validated_region(account_id, region)?;
    let credentials = verified_credentials(account_id)?;
    let endpoint = if region_info.endpoint.is_empty() {
        format!("ecs.{region}.aliyuncs.com")
    } else {
        region_info.endpoint
    };
    let mut groups = Vec::new();
    let mut next_token = String::new();
    let mut seen_tokens = HashSet::new();
    loop {
        let mut parameters = vec![("RegionId", region.into()), ("MaxResults", "100".into())];
        if !next_token.is_empty() {
            parameters.push(("NextToken", next_token.clone()));
        }
        let response = ecs_rpc(
            &credentials.access_key_id,
            &credentials.access_key_secret,
            &endpoint,
            "DescribeSecurityGroups",
            &parameters,
        )?;
        let page = response
            .pointer("/SecurityGroups/SecurityGroup")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| "阿里云安全组响应缺少 SecurityGroups.SecurityGroup".to_string())?;
        groups.extend(page.iter().filter_map(|value| {
            let id = value_string(value, "SecurityGroupId");
            (!id.is_empty()).then(|| CloudSecurityGroup {
                name: value_string(value, "SecurityGroupName"),
                vpc_id: value_string(value, "VpcId"),
                group_type: value_string(value, "SecurityGroupType"),
                id,
            })
        }));
        next_token = value_string(&response, "NextToken");
        if next_token.is_empty() {
            break;
        }
        if !seen_tokens.insert(next_token.clone()) {
            return Err("阿里云安全组分页返回了重复 NextToken，已停止读取".into());
        }
    }
    Ok(groups)
}

fn region_endpoint(account_id: &str, region: &str) -> Result<String, String> {
    let item = validated_region(account_id, region)?;
    let endpoint = if item.endpoint.is_empty() {
        format!("ecs.{region}.aliyuncs.com")
    } else {
        item.endpoint
    };
    if !endpoint.ends_with(".aliyuncs.com") || endpoint.contains('/') {
        return Err("阿里云返回的地域 Endpoint 无效".into());
    }
    Ok(endpoint)
}

fn parse_priority(value: &serde_json::Value) -> i32 {
    value
        .get("Priority")
        .and_then(|item| item.as_i64().or_else(|| item.as_str()?.parse::<i64>().ok()))
        .unwrap_or(0) as i32
}

pub fn list_security_group_rules(
    account_id: &str,
    region: &str,
    security_group_id: &str,
) -> Result<Vec<CloudSecurityRule>, String> {
    if security_group_id.trim().is_empty() {
        return Err("请选择安全组".into());
    }
    let endpoint = region_endpoint(account_id, region)?;
    let group = list_security_groups(account_id, region)?
        .into_iter()
        .find(|group| group.id == security_group_id)
        .ok_or_else(|| "所选安全组不在当前账号与地域中".to_string())?;
    let credentials = verified_credentials(account_id)?;
    let mut rules = Vec::new();
    let queries: Vec<(&str, Option<&str>)> = if group.vpc_id.is_empty() {
        vec![
            ("ingress", Some("internet")),
            ("ingress", Some("intranet")),
            ("egress", Some("internet")),
            ("egress", Some("intranet")),
        ]
    } else {
        vec![("ingress", Some("intranet")), ("egress", None)]
    };
    for (direction, nic_type) in queries {
        let mut next_token = String::new();
        let mut seen_tokens = HashSet::new();
        loop {
            let mut parameters = vec![
                ("RegionId", region.into()),
                ("SecurityGroupId", security_group_id.into()),
                ("Direction", direction.into()),
                ("MaxResults", "100".into()),
            ];
            if let Some(nic_type) = nic_type {
                parameters.push(("NicType", nic_type.into()));
            }
            if !next_token.is_empty() {
                parameters.push(("NextToken", next_token.clone()));
            }
            let response = ecs_rpc(
                &credentials.access_key_id,
                &credentials.access_key_secret,
                &endpoint,
                "DescribeSecurityGroupAttribute",
                &parameters,
            )?;
            let page = response
                .pointer("/Permissions/Permission")
                .and_then(serde_json::Value::as_array)
                .ok_or_else(|| "阿里云规则响应缺少 Permissions.Permission".to_string())?;
            rules.extend(page.iter().map(|value| {
                let actual_direction = {
                    let value = value_string(value, "Direction");
                    if value.is_empty() {
                        direction.into()
                    } else {
                        value
                    }
                };
                let source_cidr = if actual_direction == "ingress" {
                    let ipv4 = value_string(value, "SourceCidrIp");
                    if ipv4.is_empty() {
                        value_string(value, "Ipv6SourceCidrIp")
                    } else {
                        ipv4
                    }
                } else {
                    let ipv4 = value_string(value, "DestCidrIp");
                    if ipv4.is_empty() {
                        value_string(value, "Ipv6DestCidrIp")
                    } else {
                        ipv4
                    }
                };
                let description = value_string(value, "Description");
                CloudSecurityRule {
                    id: value_string(value, "SecurityGroupRuleId"),
                    direction: actual_direction,
                    protocol: value_string(value, "IpProtocol").to_uppercase(),
                    port_range: value_string(value, "PortRange"),
                    priority: parse_priority(value),
                    action: value_string(value, "Policy"),
                    source_cidr,
                    managed: description.starts_with(MANAGED_RULE_PREFIX),
                    description,
                }
            }));
            next_token = value_string(&response, "NextToken");
            if next_token.is_empty() {
                break;
            }
            if !seen_tokens.insert(next_token.clone()) {
                return Err("阿里云安全组规则分页返回了重复 NextToken，已停止读取".into());
            }
        }
    }
    Ok(rules)
}

fn validate_rule_spec(
    protocol: &str,
    port_range: &str,
    priority: i32,
) -> Result<(String, String), String> {
    let protocol = protocol.trim().to_uppercase();
    let port_range = port_range.trim().to_string();
    if !(1..=100).contains(&priority) {
        return Err("优先级必须在 1 到 100 之间".into());
    }
    if matches!(protocol.as_str(), "ICMP" | "GRE" | "ALL") {
        if port_range != "-1/-1" {
            return Err(format!("{protocol} 协议端口范围必须为 -1/-1"));
        }
    } else if matches!(protocol.as_str(), "TCP" | "UDP") {
        let values = port_range
            .split('/')
            .map(str::parse::<u16>)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| "端口范围格式应为起始/结束，例如 22/22".to_string())?;
        if values.len() != 2 || values[0] == 0 || values[0] > values[1] {
            return Err("端口范围无效".into());
        }
    } else {
        return Err("仅支持 TCP、UDP、ICMP、GRE 或 ALL".into());
    }
    Ok((protocol, port_range))
}

fn validate_source_cidr(source_cidr: &str) -> Result<(), String> {
    let ip = source_cidr
        .strip_suffix("/32")
        .ok_or_else(|| "受管授权对象必须是单个 IPv4 的 /32 CIDR".to_string())?;
    ip.parse::<std::net::Ipv4Addr>()
        .map_err(|_| "当前出口 IPv4 无效".to_string())?;
    Ok(())
}

fn sync_remote_rule(
    config: &ManagedRuleConfig,
    source_cidr: &str,
) -> Result<ManagedRuleSyncResult, String> {
    validate_source_cidr(source_cidr)?;
    let (protocol, port_range) =
        validate_rule_spec(&config.protocol, &config.port_range, config.priority)?;
    let endpoint = region_endpoint(&config.account_id, &config.region)?;
    let credentials = verified_credentials(&config.account_id)?;
    let group = list_security_groups(&config.account_id, &config.region)?
        .into_iter()
        .find(|group| group.id == config.security_group_id)
        .ok_or_else(|| "所选安全组不在当前账号与地域中".to_string())?;
    let nic_type = if group.vpc_id.is_empty() {
        "internet"
    } else {
        "intranet"
    };
    let rules = list_security_group_rules(
        &config.account_id,
        &config.region,
        &config.security_group_id,
    )?;
    let managed: Vec<_> = rules
        .iter()
        .filter(|rule| rule.direction == "ingress" && rule.description == config.description)
        .collect();
    if managed.len() > 1 {
        return Err("发现多个相同固定标识的云端规则，已停止自动更新，请人工处理重复规则".into());
    }
    let (action, rule_id) = if let Some(rule) = managed.first() {
        let parameters = vec![
            ("RegionId", config.region.clone()),
            ("SecurityGroupId", config.security_group_id.clone()),
            ("SecurityGroupRuleId", rule.id.clone()),
            ("IpProtocol", protocol),
            ("PortRange", port_range),
            ("SourceCidrIp", source_cidr.into()),
            ("Policy", "accept".into()),
            ("Priority", config.priority.to_string()),
            ("ClientToken", format!("de-{}", Uuid::new_v4())),
        ];
        ecs_rpc(
            &credentials.access_key_id,
            &credentials.access_key_secret,
            &endpoint,
            "ModifySecurityGroupRule",
            &parameters,
        )?;
        ("updated".to_string(), rule.id.clone())
    } else {
        let parameters = vec![
            ("RegionId", config.region.clone()),
            ("SecurityGroupId", config.security_group_id.clone()),
            ("IpProtocol", protocol),
            ("PortRange", port_range),
            ("SourceCidrIp", source_cidr.into()),
            ("Policy", "accept".into()),
            ("Priority", config.priority.to_string()),
            ("Description", config.description.clone()),
            ("NicType", nic_type.into()),
            ("ClientToken", format!("de-{}", Uuid::new_v4())),
        ];
        ecs_rpc(
            &credentials.access_key_id,
            &credentials.access_key_secret,
            &endpoint,
            "AuthorizeSecurityGroup",
            &parameters,
        )?;
        ("created".to_string(), String::new())
    };
    let verified = list_security_group_rules(
        &config.account_id,
        &config.region,
        &config.security_group_id,
    )?
    .into_iter()
    .find(|rule| {
        rule.direction == "ingress"
            && rule.description == config.description
            && rule.source_cidr == source_cidr
    })
    .ok_or_else(|| "阿里云已接受请求，但回读未找到期望的受管规则".to_string())?;
    let mut updated = config.clone();
    updated.last_source_cidr = Some(source_cidr.into());
    updated.last_synced_at = Some(now());
    updated.last_error = None;
    Ok(ManagedRuleSyncResult {
        config: updated,
        action,
        rule_id: if rule_id.is_empty() {
            verified.id
        } else {
            rule_id
        },
    })
}

pub fn list_managed_rules() -> Result<Vec<ManagedRuleConfig>, String> {
    load_managed_rules()
}

pub fn create_managed_rule(
    input: CreateManagedRuleInput,
    source_cidr: &str,
) -> Result<ManagedRuleSyncResult, String> {
    let _operation_guard = cloud_operation_lock().lock();
    let (protocol, port_range) =
        validate_rule_spec(&input.protocol, &input.port_range, input.priority)?;
    validate_source_cidr(source_cidr)?;
    if !list_security_groups(&input.account_id, &input.region)?
        .iter()
        .any(|group| group.id == input.security_group_id)
    {
        return Err("所选安全组不在当前账号与地域中".into());
    }
    let id = Uuid::new_v4().to_string();
    let config = ManagedRuleConfig {
        id: id.clone(),
        account_id: input.account_id,
        region: input.region,
        security_group_id: input.security_group_id,
        protocol,
        port_range,
        priority: input.priority,
        description: format!("{MANAGED_RULE_PREFIX}{id}"),
        enabled: true,
        last_source_cidr: None,
        last_synced_at: None,
        last_error: None,
    };
    {
        let _guard = store_lock().lock();
        let mut configs = load_managed_rules()?;
        configs.push(config.clone());
        save_managed_rules(&configs)?;
    }
    match sync_remote_rule(&config, source_cidr) {
        Ok(result) => {
            update_managed_config(&result.config)?;
            Ok(result)
        }
        Err(error) => {
            let mut failed = config;
            failed.last_error = Some(error.clone());
            update_managed_config(&failed)?;
            Err(error)
        }
    }
}

fn update_managed_config(config: &ManagedRuleConfig) -> Result<(), String> {
    let _guard = store_lock().lock();
    let mut configs = load_managed_rules()?;
    let target = configs
        .iter_mut()
        .find(|item| item.id == config.id)
        .ok_or_else(|| "受管规则配置不存在".to_string())?;
    *target = config.clone();
    save_managed_rules(&configs)
}

pub fn sync_managed_rule(id: &str, source_cidr: &str) -> Result<ManagedRuleSyncResult, String> {
    let _operation_guard = cloud_operation_lock().lock();
    sync_managed_rule_locked(id, source_cidr)
}

fn sync_managed_rule_locked(id: &str, source_cidr: &str) -> Result<ManagedRuleSyncResult, String> {
    let config = {
        let _guard = store_lock().lock();
        load_managed_rules()?
            .into_iter()
            .find(|item| item.id == id)
            .ok_or_else(|| "受管规则配置不存在".to_string())?
    };
    match sync_remote_rule(&config, source_cidr) {
        Ok(result) => {
            update_managed_config(&result.config)?;
            Ok(result)
        }
        Err(error) => {
            let mut failed = config;
            failed.last_error = Some(error.clone());
            update_managed_config(&failed)?;
            Err(error)
        }
    }
}

pub fn sync_all_managed_rules(source_cidr: &str) -> Result<Vec<ManagedRuleSyncResult>, String> {
    let _operation_guard = cloud_operation_lock().lock();
    let configs = {
        let _guard = store_lock().lock();
        load_managed_rules()?
    };
    let mut results = Vec::new();
    let mut errors = Vec::new();
    for config in configs.into_iter().filter(|item| item.enabled) {
        match sync_managed_rule_locked(&config.id, source_cidr) {
            Ok(result) => results.push(result),
            Err(error) => errors.push(format!("{}: {error}", config.description)),
        }
    }
    if !errors.is_empty() {
        return Err(format!("部分受管规则更新失败：{}", errors.join("；")));
    }
    Ok(results)
}

pub fn delete_managed_rule(id: &str, revoke_remote: bool) -> Result<(), String> {
    let _operation_guard = cloud_operation_lock().lock();
    let config = {
        let _guard = store_lock().lock();
        load_managed_rules()?
            .into_iter()
            .find(|item| item.id == id)
            .ok_or_else(|| "受管规则配置不存在".to_string())?
    };
    if revoke_remote {
        let rules = list_security_group_rules(
            &config.account_id,
            &config.region,
            &config.security_group_id,
        )?;
        let managed: Vec<_> = rules
            .into_iter()
            .filter(|rule| rule.direction == "ingress" && rule.description == config.description)
            .collect();
        if managed.len() > 1 {
            return Err("发现多个相同固定标识的云端规则，拒绝自动删除".into());
        }
        if let Some(rule) = managed.first() {
            let endpoint = region_endpoint(&config.account_id, &config.region)?;
            let credentials = verified_credentials(&config.account_id)?;
            let parameters = vec![
                ("RegionId", config.region.clone()),
                ("SecurityGroupId", config.security_group_id.clone()),
                ("SecurityGroupRuleId.1", rule.id.clone()),
                ("ClientToken", format!("de-{}", Uuid::new_v4())),
            ];
            ecs_rpc(
                &credentials.access_key_id,
                &credentials.access_key_secret,
                &endpoint,
                "RevokeSecurityGroup",
                &parameters,
            )?;
            if list_security_group_rules(
                &config.account_id,
                &config.region,
                &config.security_group_id,
            )?
            .iter()
            .any(|item| item.description == config.description)
            {
                return Err("阿里云已接受删除请求，但回读仍发现受管规则".into());
            }
        }
    }
    let _guard = store_lock().lock();
    let mut configs = load_managed_rules()?;
    configs.retain(|item| item.id != id);
    save_managed_rules(&configs)
}

fn read_credentials(credential_key: &str) -> Result<StoredCredentials, String> {
    let value = keychain_entry(credential_key)?
        .get_password()
        .map_err(|error| format!("读取 macOS 钥匙串凭据失败：{error}"))?;
    serde_json::from_str(&value).map_err(|_| "macOS 钥匙串中的云账号凭据格式无效".to_string())
}

fn validate(input: &SaveCloudAccountInput) -> Result<(), String> {
    let access_key_id = input.access_key_id.trim();
    if input.display_name.trim().is_empty() {
        return Err("请输入云账号备注".into());
    }
    if !["ram_access_key", "access_key"].contains(&input.auth_method.as_str()) {
        return Err("当前仅支持 RAM 子账户凭据或 AccessKey".into());
    }
    if !(8..=128).contains(&access_key_id.len())
        || !access_key_id.is_ascii()
        || !access_key_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
        || input.access_key_secret.trim().len() < 16
    {
        return Err("AccessKey ID 或 Secret 格式无效".into());
    }
    Ok(())
}

fn hint(access_key_id: &str) -> String {
    let value = access_key_id.trim();
    format!("{}****{}", &value[..4], &value[value.len() - 4..])
}
pub fn list_accounts() -> Result<Vec<CloudAccountSummary>, String> {
    Ok(load()?.into_iter().map(|account| account.summary).collect())
}

pub fn save_account(input: SaveCloudAccountInput) -> Result<CloudAccountSummary, String> {
    validate(&input)?;
    let identity = verify_identity(input.access_key_id.trim(), input.access_key_secret.trim())?;
    let _guard = store_lock().lock();
    let mut accounts = load()?;
    let timestamp = now();
    let id = format!("aliyun-{}", Uuid::new_v4());
    let credential_key = format!("cloud-account-{id}");
    let credentials = serde_json::to_string(&CloudCredentials {
        access_key_id: input.access_key_id.trim(),
        access_key_secret: input.access_key_secret.trim(),
    })
    .map_err(|error| error.to_string())?;
    keychain_store(&credential_key, &credentials)?;
    let account = StoredCloudAccount {
        summary: CloudAccountSummary {
            id,
            provider: "aliyun".into(),
            display_name: input.display_name.trim().into(),
            auth_method: input.auth_method,
            access_key_hint: hint(&input.access_key_id),
            verified_account_id: Some(identity.account_id),
            verification_status: "已验证".into(),
            created_at: timestamp,
            updated_at: timestamp,
        },
        credential_key,
    };
    accounts.push(account.clone());
    if let Err(error) = save(&accounts) {
        let _ = keychain_delete(&account.credential_key);
        return Err(error);
    }
    Ok(account.summary)
}

pub fn verify_account(id: &str) -> Result<CloudAccountSummary, String> {
    let credential_key = {
        let _guard = store_lock().lock();
        load()?
            .iter()
            .find(|account| account.summary.id == id)
            .ok_or_else(|| "云账号不存在".to_string())?
            .credential_key
            .clone()
    };
    let credentials = read_credentials(&credential_key)?;
    let identity = verify_identity(&credentials.access_key_id, &credentials.access_key_secret)?;
    let _guard = store_lock().lock();
    let mut accounts = load()?;
    let account = accounts
        .iter_mut()
        .find(|account| account.summary.id == id)
        .ok_or_else(|| "云账号在验证期间已被删除".to_string())?;
    account.summary.verified_account_id = Some(identity.account_id);
    account.summary.verification_status = "已验证".into();
    account.summary.updated_at = now();
    let summary = account.summary.clone();
    save(&accounts)?;
    Ok(summary)
}

pub fn delete_account(id: &str) -> Result<(), String> {
    let _guard = store_lock().lock();
    if load_managed_rules()?
        .iter()
        .any(|rule| rule.account_id == id)
    {
        return Err("该账号仍有关联的受管安全组规则，请先在安全组页面删除受管规则".into());
    }
    let mut accounts = load()?;
    let index = accounts
        .iter()
        .position(|account| account.summary.id == id)
        .ok_or_else(|| "云账号不存在".to_string())?;
    let account = accounts[index].clone();
    let credential = keychain_entry(&account.credential_key)?
        .get_password()
        .map_err(|error| format!("读取 macOS 钥匙串凭据失败：{error}"))?;
    keychain_delete(&account.credential_key)?;
    accounts.remove(index);
    if let Err(error) = save(&accounts) {
        let _ = keychain_store(&account.credential_key, &credential);
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_aliyun_account_input() {
        let valid = SaveCloudAccountInput {
            display_name: "生产".into(),
            auth_method: "access_key".into(),
            access_key_id: "LTAI12345678".into(),
            access_key_secret: "1234567890123456".into(),
        };
        assert!(validate(&valid).is_ok());
        assert!(validate(&SaveCloudAccountInput {
            auth_method: "password".into(),
            ..valid
        })
        .is_err());
    }
    #[test]
    fn rejects_non_ascii_access_key_id() {
        let input = SaveCloudAccountInput {
            display_name: "生产".into(),
            auth_method: "access_key".into(),
            access_key_id: "LTAI测试1234".into(),
            access_key_secret: "1234567890123456".into(),
        };
        assert!(validate(&input).is_err());
    }
    #[test]
    fn encodes_aliyun_signature_components_as_rfc3986() {
        assert_eq!(percent_encode("a-_.~ /+"), "a-_.~%20%2F%2B");
    }
    #[test]
    fn creates_stable_aliyun_v1_signature_for_fixed_inputs() {
        let query = signed_query_with_parts(
            "testid",
            "testsecret",
            "GetCallerIdentity",
            "2015-04-01",
            &[],
            "nonce",
            "2026-09-20T00:00:00Z",
        )
        .unwrap();
        assert_eq!(query, "AccessKeyId=testid&Action=GetCallerIdentity&Format=JSON&SignatureMethod=HMAC-SHA1&SignatureNonce=nonce&SignatureVersion=1.0&Timestamp=2026-09-20T00%3A00%3A00Z&Version=2015-04-01&Signature=xY4T4qW85WK4B31WkNaPC6tDuv4%3D");
    }
    #[test]
    fn validates_managed_rule_shape_and_source() {
        assert_eq!(
            validate_rule_spec("tcp", "22/22", 1).unwrap(),
            ("TCP".into(), "22/22".into())
        );
        assert!(validate_rule_spec("TCP", "0/22", 1).is_err());
        assert!(validate_rule_spec("ALL", "22/22", 1).is_err());
        assert!(validate_source_cidr("203.0.113.8/32").is_ok());
        assert!(validate_source_cidr("0.0.0.0/0").is_err());
    }
    #[test]
    fn managed_description_is_unique_and_fixed() {
        let id = Uuid::new_v4().to_string();
        let description = format!("{MANAGED_RULE_PREFIX}{id}");
        assert!(description.starts_with("DomainEgress:v1:"));
        assert_eq!(description, format!("DomainEgress:v1:{id}"));
    }
}
