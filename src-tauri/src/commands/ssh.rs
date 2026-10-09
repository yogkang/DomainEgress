use crate::state::AppState;
use std::sync::Arc;
use tauri::State;
#[tauri::command]
pub(crate) async fn ssh_forward_start(
    id: String,
    state: State<'_, AppState>,
) -> Result<u16, String> {
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
pub(crate) async fn ssh_forward_stop(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let manager = Arc::clone(&state.ssh_forward);
    tauri::async_runtime::spawn_blocking(move || manager.stop(&id).map_err(|e| e.to_string()))
        .await
        .map_err(|error| format!("停止 SSH 转发任务异常结束：{error}"))?
}
