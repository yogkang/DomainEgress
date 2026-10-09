use crate::services::network::GeoIpDatabases;
use crate::{
    config::Config,
    services::{proxy, ssh, ssh_forward, static_http},
};
use parking_lot::{Mutex, RwLock};
use std::sync::Arc;
pub(crate) struct AppState {
    pub(crate) static_http: static_http::Manager,
    pub(crate) config: Arc<RwLock<Config>>,
    pub(crate) proxy: proxy::ProxyManager,
    pub(crate) ssh: Arc<ssh::SshManager>,
    pub(crate) ssh_forward: Arc<ssh_forward::SshForwardManager>,
    pub(crate) operation: Arc<Mutex<()>>,
    pub(crate) tray_status: Mutex<Option<tauri::menu::MenuItem<tauri::Wry>>>,
    pub(crate) tray_icon: Mutex<Option<tauri::tray::TrayIcon<tauri::Wry>>>,
    pub(crate) tray_menu: Mutex<Option<tauri::menu::Menu<tauri::Wry>>>,
    pub(crate) tray_start: Mutex<Option<tauri::menu::MenuItem<tauri::Wry>>>,
    pub(crate) tray_stop: Mutex<Option<tauri::menu::MenuItem<tauri::Wry>>>,
    pub(crate) tray_quit: Mutex<Option<tauri::menu::MenuItem<tauri::Wry>>>,
    pub(crate) message: Mutex<Option<String>>,
    pub(crate) geoip: Arc<GeoIpDatabases>,
    pub(crate) network_cancel: Arc<std::sync::atomic::AtomicBool>,
}
