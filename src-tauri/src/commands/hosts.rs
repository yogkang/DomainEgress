use crate::services::hosts;
#[tauri::command]
pub(crate) async fn list_host_mappings() -> Result<Vec<hosts::HostMapping>, String> {
    tauri::async_runtime::spawn_blocking(hosts::list)
        .await
        .map_err(|error| format!("读取本地 DNS 任务异常结束：{error}"))?
}

#[tauri::command]
pub(crate) async fn list_host_groups() -> Result<Vec<hosts::HostGroup>, String> {
    tauri::async_runtime::spawn_blocking(hosts::list_groups)
        .await
        .map_err(|error| format!("读取本地 DNS 分组异常结束：{error}"))?
}

#[tauri::command]
pub(crate) async fn read_system_hosts() -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(hosts::read_system)
        .await
        .map_err(|error| format!("读取系统 Hosts 任务异常结束：{error}"))?
}

#[tauri::command]
pub(crate) async fn save_host_group(
    name: String,
    content: String,
    enabled: bool,
) -> Result<Vec<hosts::HostGroup>, String> {
    tauri::async_runtime::spawn_blocking(move || hosts::save_group(name, content, enabled))
        .await
        .map_err(|error| format!("保存本地 DNS 分组异常结束：{error}"))?
}

#[tauri::command]
pub(crate) async fn delete_host_group(name: String) -> Result<Vec<hosts::HostGroup>, String> {
    tauri::async_runtime::spawn_blocking(move || hosts::delete_group(name))
        .await
        .map_err(|error| format!("删除本地 DNS 分组异常结束：{error}"))?
}

#[tauri::command]
pub(crate) async fn add_host_mapping(
    ip: String,
    domains: Vec<String>,
    group: String,
) -> Result<Vec<hosts::HostMapping>, String> {
    tauri::async_runtime::spawn_blocking(move || hosts::add(ip, domains, group))
        .await
        .map_err(|error| format!("添加本地 DNS 任务异常结束：{error}"))?
}

#[tauri::command]
pub(crate) async fn remove_host_mapping(
    ip: String,
    domains: Vec<String>,
) -> Result<Vec<hosts::HostMapping>, String> {
    tauri::async_runtime::spawn_blocking(move || hosts::remove(ip, domains))
        .await
        .map_err(|error| format!("删除本地 DNS 任务异常结束：{error}"))?
}

#[tauri::command]
pub(crate) async fn update_host_mapping(
    old_ip: String,
    old_domains: Vec<String>,
    ip: String,
    domains: Vec<String>,
    group: String,
) -> Result<Vec<hosts::HostMapping>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        hosts::update(old_ip, old_domains, ip, domains, group)
    })
    .await
    .map_err(|error| format!("修改本地 DNS 任务异常结束：{error}"))?
}
