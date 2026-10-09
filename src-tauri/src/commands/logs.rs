use crate::state::AppState;
use tauri::State;
#[tauri::command]
pub(crate) fn clear_logs(state: State<AppState>) {
    state.proxy.clear_logs();
}
