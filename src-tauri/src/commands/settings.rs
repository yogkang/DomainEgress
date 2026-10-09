use crate::app::update_tray_status;
use crate::{config::Config, state::AppState};
use std::{fs, path::PathBuf, sync::Arc};
use tauri::State;
fn icloud_config_path() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("." )).join("Library/Mobile Documents/iCloud~com~domainegress~client/Documents/DomainEgress/config.json")
}
#[tauri::command]
pub(crate) async fn icloud_status() -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(|| {
        Ok(icloud_config_path().parent().is_some_and(|p| p.exists()))
    })
    .await
    .map_err(|error| format!("检查 iCloud 状态任务异常结束：{error}"))?
}
#[tauri::command]
pub(crate) async fn icloud_sync(state: State<'_, AppState>) -> Result<String, String> {
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
pub(crate) async fn icloud_read() -> Result<Option<Config>, String> {
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
pub(crate) fn validate(config: &Config) -> Result<(), String> {
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
pub(crate) async fn save_config(config: Config, state: State<'_, AppState>) -> Result<(), String> {
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
pub(crate) async fn set_running(running: bool, state: State<'_, AppState>) -> Result<(), String> {
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
}
