use crate::{services::network_tools, state::AppState};
use std::sync::Arc;
use tauri::State;
#[tauri::command]
pub(crate) async fn run_network_probe(
    request: network_tools::NetworkProbeRequest,
    state: State<'_, AppState>,
) -> Result<network_tools::NetworkProbeResult, String> {
    let cancel = Arc::clone(&state.network_cancel);
    cancel.store(false, std::sync::atomic::Ordering::Relaxed);
    tauri::async_runtime::spawn_blocking(move || network_tools::run(request, cancel))
        .await
        .map_err(|error| format!("网络探测任务异常结束：{error}"))
}

#[tauri::command]
pub(crate) fn cancel_network_probe(state: State<'_, AppState>) -> Result<(), String> {
    state
        .network_cancel
        .store(true, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
pub(crate) async fn probe_public_ip(
    state: State<'_, AppState>,
) -> Result<crate::types::PublicIpProbe, String> {
    let geoip = Arc::clone(&state.geoip);
    tauri::async_runtime::spawn_blocking(move || {
        crate::services::network::probe_public_addresses(&geoip)
    })
    .await
    .map_err(|error| format!("公网 IP 探测任务异常结束：{error}"))
}
