use crate::{services::static_http, state::AppState};
use tauri::State;
#[tauri::command]
pub(crate) async fn static_http_pick_directory(
    app: tauri::AppHandle,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .blocking_pick_folder()
            .map(|path| path.to_string())
    })
    .await
    .map_err(|e| e.to_string())
}
#[tauri::command]
pub(crate) fn static_http_snapshot(state: State<'_, AppState>) -> static_http::Snapshot {
    state.static_http.snapshot()
}
#[tauri::command]
pub(crate) fn static_http_save(
    config: static_http::ServiceConfig,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.static_http.save(config)
}
#[tauri::command]
pub(crate) async fn static_http_start(port: u16, state: State<'_, AppState>) -> Result<(), String> {
    state.static_http.start(port)
}
#[tauri::command]
pub(crate) async fn static_http_stop(port: u16, state: State<'_, AppState>) -> Result<(), String> {
    state.static_http.stop(port).await;
    Ok(())
}
#[tauri::command]
pub(crate) async fn static_http_remove(
    port: u16,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.static_http.remove(port).await
}
#[tauri::command]
pub(crate) fn static_http_clear_logs(state: State<'_, AppState>) {
    state.static_http.clear();
}
