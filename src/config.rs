use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, io, path::PathBuf};
fn normalize_rules(rules: &mut Vec<String>, added_at: &mut HashMap<String, u64>) {
    let mut seen = std::collections::HashSet::new();
    let original = std::mem::take(rules);
    *rules = original.into_iter().filter_map(|rule| {
        let normalized = rule.trim().trim_end_matches('.').to_lowercase();
        if !normalized.is_empty() && seen.insert(normalized.clone()) { Some(normalized) } else { None }
    }).collect();
    added_at.retain(|rule, _| rules.iter().any(|item| item == rule));
}
#[derive(Clone, Serialize, Deserialize)]
pub struct SshHop {
    pub host: String,
    pub port: u16,
    pub username: String,
    #[serde(default = "default_ssh_auth")]
    pub auth: String,
    #[serde(default)]
    pub keychain_id: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct SshProfile {
    pub id: String,
    pub name: String,
    pub hops: Vec<SshHop>,
    #[serde(default)]
    pub enabled: bool,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub version: u32,
    pub access_mode: String,
    pub whitelist: Vec<String>,
    pub blacklist: Vec<String>,
    pub http_host: String,
    pub http_port: u16,
    pub socks_host: String,
    pub socks_port: u16,
    pub auto_start: bool,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default = "default_log_retention_days")]
    pub log_retention_days: u32,
    #[serde(default = "default_trend_retention_days")]
    pub trend_retention_days: u32,
    #[serde(default)]
    pub whitelist_added_at: HashMap<String, u64>,
    #[serde(default)]
    pub blacklist_added_at: HashMap<String, u64>,
    #[serde(default)]
    pub ssh_profiles: Vec<SshProfile>,
    #[serde(default)]
    pub active_ssh_profile: Option<String>,
    #[serde(default)]
    pub icloud_sync_enabled: bool,
    #[serde(skip)]
    pub ssh_proxy_port: Option<u16>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            version: 2,
            access_mode: "whitelist".into(),
            whitelist: vec![
                "agentclientprotocol.com".into(),
                "alicdn.com".into(),
                "aliyun.com".into(),
                "aliyuncs.com".into(),
                "apache.org".into(),
                "apipost.net".into(),
                "bcebos.com".into(),
                "bdstatic.com".into(),
                "brucege.com".into(),
                "byteacctimg.com".into(),
                "cloudfront.net".into(),
                "codebuddy.cn".into(),
                "corretto.aws".into(),
                "ctobsnssdk.com".into(),
                "getcomposer.org".into(),
                "gitee.com".into(),
                "github.com".into(),
                "github.io".into(),
                "githubcopilot.com".into(),
                "githubusercontent.com".into(),
                "golang.org".into(),
                "google.cn".into(),
                "google.com".into(),
                "googleapis.com".into(),
                "googletagmanager.co".into(),
                "goproxy.cn".into(),
                "gradle.org".into(),
                "ibb.co".into(),
                "jboss.org".into(),
                "laravel-idea.com".into(),
                "lierda.com".into(),
                "marscode.cn".into(),
                "marscode.com".into(),
                "maven.org".into(),
                "mixpanel.com".into(),
                "mvnrepository.com".into(),
                "myqcloud.com".into(),
                "pypi.org".into(),
                "python.org".into(),
                "qcloudimg.com".into(),
                "qoder.com".into(),
                "qoder.com.cn".into(),
                "qodo.ai".into(),
                "rumt-zh.com".into(),
                "schemastore.org".into(),
                "senthink.com".into(),
                "sinfere.com".into(),
                "spring.io".into(),
                "tencent-cloud.com".into(),
                "tencent.com".into(),
                "trae.com.cn".into(),
                "zijieapi.com".into(),
                "api.app.prod.grazie.aws.intellij.net".into(),
                "api.jetbrains.ai".into(),
                "cache-redirector.jetbrains.com".into(),
                "cloudconfig.jetbrains.com".into(),
                "code-with-me.jetbrains.com".into(),
                "code-with-me.jetbrains.com.cn".into(),
                "download-alibaba.jetbrains.com.cn".into(),
                "download-cdn.clf.jetbrains.com.cn".into(),
                "download-cdn.jetbrains.com".into(),
                "download-cdn.jetbrains.com.cn".into(),
                "download.clf.jetbrains.com.cn".into(),
                "download.jetbrains.com".into(),
                "downloads.marketplace.jetbrains.com".into(),
                "eks.jetbrains.com.cn".into(),
                "frameworks.jetbrains.com".into(),
                "marketplace.jetbrains.com".into(),
                "marketplace.jetbrains.com.cn".into(),
                "oauth.account.jetbrains.com".into(),
                "plugins.jetbrains.com".into(),
                "redirector.jetbrains.com.cn".into(),
                "resources.jetbrains.com".into(),
                "resources.jetbrains.com.cn".into(),
                "vulnerability-search.jetbrains.com".into(),
                "www.jetbrains.com".into(),
            ],
            blacklist: vec![],
            http_host: "127.0.0.1".into(),
            http_port: 19876,
            socks_host: "127.0.0.1".into(),
            socks_port: 16789,
            auto_start: true,
            log_level: "info".into(),
            log_retention_days: 30,
            trend_retention_days: 21,
            whitelist_added_at: HashMap::new(),
            blacklist_added_at: HashMap::new(),
            ssh_profiles: Vec::new(),
            active_ssh_profile: None,
            icloud_sync_enabled: false,
            ssh_proxy_port: None,
        }
    }
}
fn default_log_level() -> String {
    "info".into()
}
fn default_log_retention_days() -> u32 {
    30
}
fn default_trend_retention_days() -> u32 { 21 }
fn default_ssh_auth() -> String { "agent".into() }
impl Config {
    fn path() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("DomainEgress/config.json")
    }
    pub fn load() -> io::Result<Self> {
        let s = fs::read_to_string(Self::path())?;
        let mut config: Self = serde_json::from_str(&s).map_err(io::Error::other)?;
        normalize_rules(&mut config.whitelist, &mut config.whitelist_added_at);
        normalize_rules(&mut config.blacklist, &mut config.blacklist_added_at);
        if config.version < 2 {
            config.version = 2;
            config.auto_start = true;
            let _ = config.save();
        }
        let _ = config.save();
        Ok(config)
    }
    pub fn save(&self) -> io::Result<()> {
        let p = Self::path();
        fs::create_dir_all(p.parent().unwrap())?;
        let t = p.with_extension("json.tmp");
        fs::write(
            &t,
            serde_json::to_vec_pretty(self).map_err(io::Error::other)?,
        )?;
        fs::rename(t, p)
    }
}
