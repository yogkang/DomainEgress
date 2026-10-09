use crate::types::*;
use std::process::Command;
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
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
pub(crate) async fn check_update() -> Result<UpdateInfo, String> {
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
pub(crate) async fn open_update(url: String) -> Result<(), String> {
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
