use crate::config::SshForwardRule;
use anyhow::{anyhow, Result};
use parking_lot::Mutex;
use std::{collections::HashMap, net::TcpListener, process::{Child, Command, Stdio}};

pub struct SshForwardManager { children: Mutex<HashMap<String, (Child, u16)>> }
impl SshForwardManager {
    pub fn new() -> Self { Self { children: Mutex::new(HashMap::new()) } }
    pub fn running_ports(&self) -> HashMap<String, u16> { self.children.lock().iter_mut().filter_map(|(id, (child, port))| if child.try_wait().ok().flatten().is_none() { Some((id.clone(), *port)) } else { None }).collect() }
    pub fn start(&self, rule: &SshForwardRule) -> Result<u16> {
        self.stop(&rule.id)?;
        if rule.remote_host.trim().is_empty() || rule.remote_port == 0 || rule.ssh_host.trim().is_empty() || rule.ssh_username.trim().is_empty() || rule.ssh_port == 0 { return Err(anyhow!("SSH 转发配置不完整")); }
        let port = rule.local_port.unwrap_or_else(|| (28000..=29000).find(|port| TcpListener::bind(("127.0.0.1", *port)).is_ok()).unwrap_or(0));
        if port == 0 { return Err(anyhow!("28000–29000 范围内没有可用本地端口")); }
        let mut command = Command::new("/usr/bin/ssh");
        command.args(["-N", "-T", "-C", "-o", "ServerAliveInterval=60", "-o", "ServerAliveCountMax=3", "-o", "ExitOnForwardFailure=yes"]);
        command.arg("-L").arg(format!("{}:{}:{}:{}", rule.bind_host, port, rule.remote_host, rule.remote_port));
        if let Some(key) = &rule.ssh_keychain_id { if !key.trim().is_empty() { command.args(["-i", key]); } }
        command.arg(format!("{}@{}", rule.ssh_username, rule.ssh_host)).arg("-p").arg(rule.ssh_port.to_string()).stdout(Stdio::null()).stderr(Stdio::piped());
        let child = command.spawn().map_err(|e| anyhow!("启动 SSH 转发失败：{e}"))?;
        self.children.lock().insert(rule.id.clone(), (child, port));
        Ok(port)
    }
    pub fn stop(&self, id: &str) -> Result<()> { if let Some((mut child, _)) = self.children.lock().remove(id) { let _ = child.kill(); let _ = child.wait(); } Ok(()) }
    pub fn stop_all(&self) { for (_, (mut child, _)) in self.children.lock().drain() { let _ = child.kill(); let _ = child.wait(); } }
}
