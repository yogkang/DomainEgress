use crate::commands::settings::set_running;
use crate::services::network::GeoIpDatabases;
use crate::{
    config::Config,
    services::{proxy, ssh, ssh_forward, static_http},
    state::AppState,
};
use parking_lot::{Mutex, RwLock};
use std::{path::PathBuf, sync::Arc};
use tauri::Manager;
pub(crate) fn update_tray_status(state: &AppState, running: bool) {
    if let Some(item) = state.tray_status.lock().as_ref() {
        let _ = item.set_text(if running {
            "🟢 代理运行中"
        } else {
            "⚪ 代理已停止"
        });
    }
    let menu = state.tray_menu.lock().clone();
    let start = state.tray_start.lock().clone();
    let stop = state.tray_stop.lock().clone();
    let quit = state.tray_quit.lock().clone();
    if let (Some(menu), Some(start), Some(stop), Some(quit)) = (menu, start, stop, quit) {
        let (active, inactive) = if running {
            (&stop, &start)
        } else {
            (&start, &stop)
        };
        let _ = menu.remove(inactive);
        let _ = menu.remove(active);
        let _ = menu.remove(&quit);
        let _ = menu.append(active);
        let _ = menu.append(&quit);
    }
    if let Some(icon) = state.tray_icon.lock().as_ref() {
        let bytes: &[u8] = if running {
            &include_bytes!("../icons/tray-running.png")[..]
        } else {
            &include_bytes!("../icons/tray-stopped.png")[..]
        };
        if let Ok(image) = tauri::image::Image::from_bytes(bytes) {
            let _ = icon.set_icon(Some(image));
        }
    }
}
pub(crate) fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .setup(|app| {
            let (config, mut message) = match Config::load() {
                Ok(c) => (c, None),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (Config::default(), None),
                Err(e) => {
                    let mut c = Config::default();
                    c.auto_start = false;
                    (c, Some(format!("配置读取失败，已暂停自动启动：{e}")))
                }
            };
            let shared = Arc::new(RwLock::new(config.clone()));
            let proxy = proxy::ProxyManager::new(shared.clone());
            let ssh = Arc::new(ssh::SshManager::new());
            let ssh_forward = Arc::new(ssh_forward::SshForwardManager::new());
            let geoip =
                GeoIpDatabases::load(&app.path().resource_dir().unwrap_or_else(|_| {
                    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources")
                }));
            if config.auto_start {
                for rule in config.ssh_forwards.iter().filter(|rule| rule.auto_start) {
                    let mut effective = rule.clone();
                    effective
                        .ssh_options
                        .extend(config.ssh_forward_options.clone());
                    if let Err(error) = ssh_forward.start(&effective) {
                        message = Some(format!("SSH 转发 {} 自动启动失败：{error}", rule.name));
                    }
                }
            }
            if config.auto_start {
                let startup = crate::commands::settings::validate(&config).and_then(|_| {
                    if let Some(id) = config.active_ssh_profile.as_ref() {
                        if let Some(profile) = config.ssh_profiles.iter().find(|p| &p.id == id) {
                            let port = ssh.start(profile).map_err(|e| e.to_string())?;
                            shared.write().ssh_proxy_port = Some(port);
                        }
                    }
                    proxy.start().map_err(|e| e.to_string())
                });
                if let Err(e) = startup {
                    let _ = ssh.stop();
                    message = Some(format!(
                        "代理自动启动失败：{e}。请检查是否有其他进程占用监听端口。"
                    ));
                }
            }
            app.manage(AppState {
                static_http: static_http::Manager::new(),
                config: shared,
                proxy,
                ssh,
                ssh_forward,
                operation: Arc::new(Mutex::new(())),
                tray_status: Mutex::new(None),
                tray_icon: Mutex::new(None),
                tray_menu: Mutex::new(None),
                tray_start: Mutex::new(None),
                tray_stop: Mutex::new(None),
                tray_quit: Mutex::new(None),
                message: Mutex::new(message),
                geoip: Arc::new(geoip),
                network_cancel: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            });
            use tauri::{
                menu::{Menu, MenuItem},
                tray::TrayIconBuilder,
            };
            let show = MenuItem::with_id(app, "show", "显示 DomainEgress", true, None::<&str>)?;
            let status = MenuItem::with_id(
                app,
                "status",
                if app.state::<AppState>().proxy.is_running() {
                    "🟢 代理运行中"
                } else {
                    "⚪ 代理已停止"
                },
                false,
                None::<&str>,
            )?;
            let start = MenuItem::with_id(app, "start", "启动代理", true, None::<&str>)?;
            let stop = MenuItem::with_id(app, "stop", "停止代理", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &status, &start, &stop, &quit])?;
            *app.state::<AppState>().tray_status.lock() = Some(status.clone());
            *app.state::<AppState>().tray_menu.lock() = Some(menu.clone());
            *app.state::<AppState>().tray_start.lock() = Some(start.clone());
            *app.state::<AppState>().tray_stop.lock() = Some(stop.clone());
            *app.state::<AppState>().tray_quit.lock() = Some(quit.clone());
            let initially_running = app.state::<AppState>().proxy.is_running();
            if initially_running {
                let _ = menu.remove(&start);
            } else {
                let _ = menu.remove(&stop);
            }
            let initial_icon: &[u8] = if app.state::<AppState>().proxy.is_running() {
                &include_bytes!("../icons/tray-running.png")[..]
            } else {
                &include_bytes!("../icons/tray-stopped.png")[..]
            };
            let tray = TrayIconBuilder::new()
                .icon(tauri::image::Image::from_bytes(initial_icon)?)
                .icon_as_template(true)
                .tooltip("DomainEgress")
                .menu(&menu)
                .on_menu_event(|app, event| {
                    let state = app.state::<AppState>();
                    match event.id.as_ref() {
                        "show" => {
                            if let Some(w) = app.get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                        "start" | "stop" => {
                            let running = event.id.as_ref() == "start";
                            let app = app.clone();
                            tauri::async_runtime::spawn(async move {
                                let state = app.state::<AppState>();
                                if let Err(error) = set_running(running, state.clone()).await {
                                    *state.message.lock() = Some(error);
                                    if let Some(window) = app.get_webview_window("main") {
                                        let _ = window.show();
                                    }
                                }
                            });
                        }
                        "quit" => {
                            let _ = state.proxy.stop();
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .build(app)?;
            *app.state::<AppState>().tray_icon.lock() = Some(tray);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            crate::commands::snapshot::snapshot,
            crate::commands::static_http::static_http_pick_directory,
            crate::commands::static_http::static_http_snapshot,
            crate::commands::static_http::static_http_save,
            crate::commands::static_http::static_http_start,
            crate::commands::static_http::static_http_stop,
            crate::commands::static_http::static_http_remove,
            crate::commands::static_http::static_http_clear_logs,
            crate::commands::network::probe_public_ip,
            crate::commands::update::check_update,
            crate::commands::update::open_update,
            crate::commands::settings::save_config,
            crate::commands::settings::set_running,
            crate::commands::ssh::ssh_forward_start,
            crate::commands::ssh::ssh_forward_stop,
            crate::commands::logs::clear_logs,
            crate::commands::gist::gist_pull,
            crate::commands::gist::gist_push,
            crate::commands::ports::list_ports,
            crate::commands::ports::terminate_process,
            crate::commands::keychain::keychain_set,
            crate::commands::keychain::keychain_delete,
            crate::commands::network::run_network_probe,
            crate::commands::network::cancel_network_probe,
            crate::commands::hosts::list_host_mappings,
            crate::commands::hosts::list_host_groups,
            crate::commands::hosts::read_system_hosts,
            crate::commands::hosts::save_host_group,
            crate::commands::hosts::delete_host_group,
            crate::commands::hosts::add_host_mapping,
            crate::commands::hosts::remove_host_mapping,
            crate::commands::hosts::update_host_mapping,
            crate::commands::cloud::list_cloud_accounts,
            crate::commands::cloud::save_cloud_account,
            crate::commands::cloud::verify_cloud_account,
            crate::commands::cloud::list_cloud_regions,
            crate::commands::cloud::list_cloud_security_groups,
            crate::commands::cloud::list_cloud_security_group_rules,
            crate::commands::cloud::list_cloud_managed_rules,
            crate::commands::cloud::preview_cloud_managed_source,
            crate::commands::cloud::create_cloud_managed_rule,
            crate::commands::cloud::sync_cloud_managed_rule,
            crate::commands::cloud::sync_all_cloud_managed_rules,
            crate::commands::cloud::delete_cloud_managed_rule,
            crate::commands::cloud::delete_cloud_account,
            crate::commands::settings::icloud_status,
            crate::commands::settings::icloud_sync,
            crate::commands::settings::icloud_read
        ])
        .build(tauri::generate_context!())
        .expect("启动 DomainEgress 失败")
        .run(|app, event| match event {
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { .. } => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            tauri::RunEvent::Exit => {
                let _ = app.state::<AppState>().proxy.stop();
                app.state::<AppState>().ssh_forward.stop_all();
            }
            _ => {}
        });
}
