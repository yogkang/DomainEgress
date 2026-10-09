use crate::config::Config;
use crate::services::proxy;
use serde::Serialize;
#[derive(Serialize)]
pub(crate) struct Snapshot {
    pub(crate) config: Config,
    pub(crate) running: bool,
    pub(crate) logs: Vec<proxy::LogEntry>,
    pub(crate) traffic: Vec<u64>,
    pub(crate) message: Option<String>,
    pub(crate) ssh_running: bool,
    pub(crate) ssh_local_port: Option<u16>,
    pub(crate) interfaces: Vec<NetworkInterface>,
    pub(crate) hostname: String,
    pub(crate) ssh_forward_ports: std::collections::HashMap<String, u16>,
}
#[derive(Clone, Serialize)]
pub(crate) struct NetworkInterface {
    pub(crate) name: String,
    pub(crate) kind: String,
    pub(crate) addresses: Vec<String>,
}
#[derive(Clone, Serialize)]
pub(crate) struct GeoLocation {
    pub(crate) country: String,
    pub(crate) country_code: String,
    pub(crate) region: String,
    pub(crate) city: String,
    pub(crate) isp: String,
    pub(crate) source: String,
    pub(crate) confidence: String,
}
#[derive(Clone, Serialize)]
pub(crate) struct EgressAddress {
    pub(crate) ip: String,
    pub(crate) source: String,
    pub(crate) location: Option<GeoLocation>,
}
#[derive(Clone, Serialize)]
pub(crate) struct EgressProbe {
    pub(crate) addresses: Vec<EgressAddress>,
    pub(crate) confidence: String,
    pub(crate) error: Option<String>,
}
#[derive(Clone, Serialize)]
pub(crate) struct PublicIpProbe {
    pub(crate) system: EgressProbe,
    pub(crate) ipv6: Option<EgressProbe>,
}
#[derive(Clone, Serialize)]
pub(crate) struct UpdateInfo {
    pub(crate) current_version: String,
    pub(crate) latest_version: Option<String>,
    pub(crate) release_url: Option<String>,
    pub(crate) available: bool,
    pub(crate) error: Option<String>,
}
