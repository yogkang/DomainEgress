use crate::config::{SshHop, SshProfile};
use anyhow::{anyhow, Result};
use parking_lot::Mutex;
use std::process::{Child, Command, Stdio};

pub struct SshManager {
    child: Mutex<Option<Child>>,
    local_port: Mutex<Option<u16>>,
}

impl SshManager {
    pub fn new() -> Self { Self { child: Mutex::new(None), local_port: Mutex::new(None) } }
    pub fn is_running(&self) -> bool { self.child.lock().as_mut().is_some_and(|child| child.try_wait().ok().flatten().is_none()) }
    pub fn local_port(&self) -> Option<u16> { *self.local_port.lock() }
    pub fn stop(&self) -> Result<()> {
        if let Some(mut child) = self.child.lock().take() { let _ = child.kill(); let _ = child.wait(); }
        *self.local_port.lock() = None;
        Ok(())
    }
    pub fn start(&self, profile: &SshProfile) -> Result<u16> {
        self.stop()?;
        validate_profile(profile)?;
        let local_port = 21000u16.checked_add(profile.id.bytes().map(u16::from).sum::<u16>() % 1000).ok_or_else(|| anyhow!("SSH 本地端口计算失败"))?;
        let destination = profile.hops.last().ok_or_else(|| anyhow!("SSH 链路至少需要一个服务器"))?;
        if destination.auth == "password" { return Err(anyhow!("密码认证需要桌面密码输入流程，当前版本请使用 ssh-agent 或钥匙串私钥")); }
        let mut cmd = Command::new("/usr/bin/ssh");
        cmd.args(["-N", "-T", "-A", "-D", &format!("127.0.0.1:{local_port}"), "-o", "ExitOnForwardFailure=yes", "-o", "ServerAliveInterval=30", "-o", "ServerAliveCountMax=3", "-o", "BatchMode=yes"]);
        if profile.hops.len() > 1 {
            let jump = profile.hops[..profile.hops.len() - 1].iter().map(format_hop).collect::<Vec<_>>().join(",");
            cmd.args(["-J", &jump]);
        }
        if destination.auth == "keychain" {
            if let Some(key) = &destination.keychain_id { cmd.args(["-i", key]); }
        }
        cmd.arg(format!("{}@{}", destination.username, destination.host)).arg("-p").arg(destination.port.to_string()).stdout(Stdio::null()).stderr(Stdio::piped());
        let child = cmd.spawn().map_err(|e| anyhow!("启动 SSH 失败：{e}"))?;
        *self.child.lock() = Some(child);
        *self.local_port.lock() = Some(local_port);
        Ok(local_port)
    }
}

fn format_hop(hop: &SshHop) -> String { format!("{}@{}:{}", hop.username, hop.host, hop.port) }
fn validate_profile(profile: &SshProfile) -> Result<()> {
    if profile.name.trim().is_empty() || profile.hops.is_empty() { return Err(anyhow!("SSH 配置名称和服务器不能为空")); }
    for hop in &profile.hops {
        if hop.host.trim().is_empty() || hop.username.trim().is_empty() || hop.port == 0 { return Err(anyhow!("SSH 服务器地址、端口和用户名不能为空")); }
        if !["agent", "keychain", "password"].contains(&hop.auth.as_str()) { return Err(anyhow!("不支持的 SSH 认证方式：{}", hop.auth)); }
    }
    Ok(())
}
