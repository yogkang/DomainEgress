use std::process::Command;
fn keychain_service() -> &'static str {
    "com.domainegress.client.ssh"
}
#[tauri::command]
pub(crate) async fn keychain_set(account: String, secret: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || keychain_set_blocking(account, secret))
        .await
        .map_err(|error| format!("保存钥匙串任务异常结束：{error}"))?
}

fn keychain_set_blocking(account: String, secret: String) -> Result<(), String> {
    if account.trim().is_empty() || secret.is_empty() {
        return Err("钥匙串账号和凭据不能为空".into());
    }
    let output = Command::new("/usr/bin/security")
        .args([
            "add-generic-password",
            "-a",
            &account,
            "-s",
            keychain_service(),
            "-w",
            &secret,
            "-U",
        ])
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}
#[tauri::command]
pub(crate) async fn keychain_delete(account: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || keychain_delete_blocking(account))
        .await
        .map_err(|error| format!("删除钥匙串任务异常结束：{error}"))?
}
fn keychain_delete_blocking(account: String) -> Result<(), String> {
    let output = Command::new("/usr/bin/security")
        .args([
            "delete-generic-password",
            "-a",
            &account,
            "-s",
            keychain_service(),
        ])
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}
