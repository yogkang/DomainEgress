use serde::Serialize;
use std::process::Command;
#[derive(Serialize)]
pub(crate) struct PortRow {
    pub(crate) port: String,
    pub(crate) pid: u32,
    pub(crate) name: String,
    pub(crate) started: String,
    pub(crate) elapsed: String,
}
fn ps_value(pid: u32, field: &str) -> Result<String, String> {
    let output = Command::new("/bin/ps")
        .args(["-p", &pid.to_string(), "-o", &format!("{field}=")])
        .output()
        .map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
fn ports() -> Result<Vec<PortRow>, String> {
    let output = Command::new("/usr/sbin/lsof")
        .args(["-nP", "-iTCP", "-sTCP:LISTEN", "-F", "pcn"])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() && !output.stderr.is_empty() {
        return Err(String::from_utf8_lossy(&output.stderr).into());
    }
    let (mut pid, mut name, mut rows) = (0, String::new(), Vec::new());
    let mut times = std::collections::HashMap::new();
    for field in String::from_utf8_lossy(&output.stdout).lines() {
        if let Some(v) = field.strip_prefix('p') {
            pid = v.parse().unwrap_or(0);
        } else if let Some(v) = field.strip_prefix('c') {
            name = v.to_string();
        } else if let Some(v) = field.strip_prefix('n') {
            if pid > 0 {
                let (started, elapsed) = times.entry(pid).or_insert_with(|| {
                    (
                        ps_value(pid, "lstart").unwrap_or_default(),
                        ps_value(pid, "etime").unwrap_or_default(),
                    )
                });
                rows.push(PortRow {
                    port: v.into(),
                    pid,
                    name: name.clone(),
                    started: started.clone(),
                    elapsed: elapsed.clone(),
                });
            }
        }
    }
    rows.sort_by(|a, b| a.port.cmp(&b.port).then(a.pid.cmp(&b.pid)));
    rows.dedup_by(|a, b| a.pid == b.pid && a.port == b.port);
    Ok(rows)
}
#[tauri::command]
pub(crate) async fn list_ports() -> Result<Vec<PortRow>, String> {
    tauri::async_runtime::spawn_blocking(ports)
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub(crate) async fn terminate_process(pid: u32, started: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        if pid <= 1 || pid == std::process::id() {
            return Err("不允许终止此进程".into());
        }
        if !ports()?
            .iter()
            .any(|p| p.pid == pid && p.started == started)
            || started.is_empty()
        {
            return Err("进程已变化，请刷新列表后重试".into());
        }
        let output = Command::new("/bin/kill")
            .args(["-TERM", &pid.to_string()])
            .output()
            .map_err(|e| e.to_string())?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).into())
        }
    })
    .await
    .map_err(|e| e.to_string())?
}
