use crate::config::Config;
use serde::{Deserialize, Serialize};
use std::process::Command;
#[derive(Serialize, Deserialize)]
pub(crate) struct GistRules {
    pub(crate) format: String,
    pub(crate) version: u32,
    pub(crate) whitelist: Vec<String>,
    pub(crate) blacklist: Vec<String>,
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
pub(crate) async fn gist_pull(
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
pub(crate) async fn gist_push(
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
