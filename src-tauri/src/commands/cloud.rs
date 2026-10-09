use crate::services::cloud;
use crate::services::network::trusted_public_ipv4;
#[tauri::command]
pub(crate) async fn list_cloud_accounts() -> Result<Vec<cloud::CloudAccountSummary>, String> {
    tauri::async_runtime::spawn_blocking(cloud::list_accounts)
        .await
        .map_err(|error| format!("读取云账号任务异常结束：{error}"))?
}
#[tauri::command]
pub(crate) async fn save_cloud_account(
    input: cloud::SaveCloudAccountInput,
) -> Result<cloud::CloudAccountSummary, String> {
    tauri::async_runtime::spawn_blocking(move || cloud::save_account(input))
        .await
        .map_err(|error| format!("保存云账号任务异常结束：{error}"))?
}
#[tauri::command]
pub(crate) async fn verify_cloud_account(id: String) -> Result<cloud::CloudAccountSummary, String> {
    tauri::async_runtime::spawn_blocking(move || cloud::verify_account(&id))
        .await
        .map_err(|error| format!("验证云账号任务异常结束：{error}"))?
}
#[tauri::command]
pub(crate) async fn list_cloud_regions(
    account_id: String,
) -> Result<Vec<cloud::CloudRegion>, String> {
    tauri::async_runtime::spawn_blocking(move || cloud::list_regions(&account_id))
        .await
        .map_err(|error| format!("读取云地域任务异常结束：{error}"))?
}
#[tauri::command]
pub(crate) async fn list_cloud_security_groups(
    account_id: String,
    region: String,
) -> Result<Vec<cloud::CloudSecurityGroup>, String> {
    tauri::async_runtime::spawn_blocking(move || cloud::list_security_groups(&account_id, &region))
        .await
        .map_err(|error| format!("读取安全组任务异常结束：{error}"))?
}
#[tauri::command]
pub(crate) async fn list_cloud_security_group_rules(
    account_id: String,
    region: String,
    security_group_id: String,
) -> Result<Vec<cloud::CloudSecurityRule>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        cloud::list_security_group_rules(&account_id, &region, &security_group_id)
    })
    .await
    .map_err(|error| format!("读取安全组规则任务异常结束：{error}"))?
}
#[tauri::command]
pub(crate) async fn list_cloud_managed_rules() -> Result<Vec<cloud::ManagedRuleConfig>, String> {
    tauri::async_runtime::spawn_blocking(cloud::list_managed_rules)
        .await
        .map_err(|error| format!("读取受管规则任务异常结束：{error}"))?
}
#[tauri::command]
pub(crate) async fn preview_cloud_managed_source() -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(trusted_public_ipv4)
        .await
        .map_err(|error| format!("探测授权对象任务异常结束：{error}"))?
}
#[tauri::command]
pub(crate) async fn create_cloud_managed_rule(
    input: cloud::CreateManagedRuleInput,
) -> Result<cloud::ManagedRuleSyncResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let source_cidr = trusted_public_ipv4()?;
        cloud::create_managed_rule(input, &source_cidr)
    })
    .await
    .map_err(|error| format!("创建受管规则任务异常结束：{error}"))?
}
#[tauri::command]
pub(crate) async fn sync_cloud_managed_rule(
    id: String,
) -> Result<cloud::ManagedRuleSyncResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let source_cidr = trusted_public_ipv4()?;
        cloud::sync_managed_rule(&id, &source_cidr)
    })
    .await
    .map_err(|error| format!("同步受管规则任务异常结束：{error}"))?
}
#[tauri::command]
pub(crate) async fn sync_all_cloud_managed_rules(
) -> Result<Vec<cloud::ManagedRuleSyncResult>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let source_cidr = trusted_public_ipv4()?;
        cloud::sync_all_managed_rules(&source_cidr)
    })
    .await
    .map_err(|error| format!("同步全部受管规则任务异常结束：{error}"))?
}
#[tauri::command]
pub(crate) async fn delete_cloud_managed_rule(
    id: String,
    revoke_remote: bool,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || cloud::delete_managed_rule(&id, revoke_remote))
        .await
        .map_err(|error| format!("删除受管规则任务异常结束：{error}"))?
}
#[tauri::command]
pub(crate) async fn delete_cloud_account(id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || cloud::delete_account(&id))
        .await
        .map_err(|error| format!("删除云账号任务异常结束：{error}"))?
}
