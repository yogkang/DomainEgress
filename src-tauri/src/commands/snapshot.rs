use crate::services::network::{local_hostname, local_interfaces};
use crate::{state::AppState, types::*};
use tauri::State;
#[tauri::command]
pub(crate) async fn snapshot(state: State<'_, AppState>) -> Result<Snapshot, String> {
    let config = state.config.read().clone();
    let running = state.proxy.is_running();
    let logs = state.proxy.logs();
    let traffic = state.proxy.traffic();
    let message = state.message.lock().take();
    let ssh_running = state.ssh.is_running();
    let ssh_local_port = state.ssh.local_port();
    let ssh_forward_ports = state.ssh_forward.running_ports();
    let (interfaces, hostname) =
        tauri::async_runtime::spawn_blocking(|| (local_interfaces(), local_hostname()))
            .await
            .map_err(|error| format!("读取本机网络信息任务异常结束：{error}"))?;
    Ok(Snapshot {
        config,
        running,
        logs,
        traffic,
        message,
        ssh_running,
        ssh_local_port,
        interfaces,
        hostname,
        ssh_forward_ports,
    })
}
