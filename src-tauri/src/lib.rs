// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

pub mod mcp_types;
pub mod state_manager;
pub mod named_pipe;
pub mod mcp_server;
pub mod mcp_installer;

pub use mcp_server::run_mcp_server;
pub use mcp_installer::set_mcp_registration;

use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tauri_plugin_updater::UpdaterExt;
use std::sync::Mutex;
use mcp_types::{IpcRequest, IpcResponse, Preset};

struct UpdateState {
    pending_update: Mutex<Option<tauri_plugin_updater::Update>>,
}

const RUN_KEY: &str = "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const RUN_VALUE: &str = "SciFiChronoWidget";

/// Reads the path currently registered under the autostart Run value, if any.
fn registered_autostart_path() -> Option<String> {
    let output = std::process::Command::new("reg")
        .args(&["query", RUN_KEY, "/v", RUN_VALUE])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        if line.contains(RUN_VALUE) {
            if let Some(idx) = line.find("REG_SZ") {
                let value = line[idx + "REG_SZ".len()..].trim();
                if !value.is_empty() {
                    return Some(value.trim_matches('"').to_string());
                }
            }
        }
    }
    None
}

#[tauri::command]
fn set_autostart(enable: bool) -> Result<(), String> {
    let exe_path = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .to_string();

    if enable {
        let lowered = exe_path.to_lowercase();
        if lowered.contains("\\target\\debug\\") || lowered.contains("\\target\\release\\") {
            return Err(
                "Autostart can only be enabled from the installed app, not a dev/build binary. \
                 Install the app first, then enable autostart from the installed copy."
                    .into(),
            );
        }
    }

    let status = if enable {
        std::process::Command::new("reg")
            .args(&[
                "add", RUN_KEY, "/v", RUN_VALUE, "/t", "REG_SZ", "/d", &exe_path, "/f",
            ])
            .status()
            .map_err(|e| e.to_string())?
    } else {
        std::process::Command::new("reg")
            .args(&["delete", RUN_KEY, "/v", RUN_VALUE, "/f"])
            .status()
            .map_err(|e| e.to_string())?
    };

    if status.success() {
        Ok(())
    } else {
        Err("Registry command failed".into())
    }
}

#[tauri::command]
fn is_autostart_enabled() -> bool {
    let current = match std::env::current_exe() {
        Ok(p) => p,
        Err(_) => return false,
    };

    match registered_autostart_path() {
        Some(registered) => {
            std::path::Path::new(&registered)
                .canonicalize()
                .ok()
                .zip(current.canonicalize().ok())
                .map(|(a, b)| a == b)
                .unwrap_or_else(|| registered.eq_ignore_ascii_case(&current.to_string_lossy()))
        }
        None => false,
    }
}

#[tauri::command]
fn set_mcp_enabled(enable: bool) -> Result<Vec<String>, String> {
    mcp_installer::set_mcp_registration(enable)
}

#[tauri::command]
fn is_mcp_enabled() -> bool {
    mcp_installer::is_mcp_registered()
}

#[tauri::command]
async fn check_for_updates(app: tauri::AppHandle, state: tauri::State<'_, UpdateState>) -> Result<Option<String>, String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    if let Some(update) = updater.check().await.map_err(|e| e.to_string())? {
        let version = update.version.clone();
        let mut pending = state.pending_update.lock().unwrap();
        *pending = Some(update);
        Ok(Some(version))
    } else {
        Ok(None)
    }
}

#[tauri::command]
async fn start_update_install(app: tauri::AppHandle, state: tauri::State<'_, UpdateState>) -> Result<(), String> {
    let update = {
        let mut pending = state.pending_update.lock().unwrap();
        pending.take()
    };
    if let Some(update) = update {
        update.download_and_install(|_received, _total| {}, || {}).await.map_err(|e| e.to_string())?;
        app.restart();
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .manage(UpdateState {
            pending_update: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            set_autostart,
            is_autostart_enabled,
            set_mcp_enabled,
            is_mcp_enabled,
            check_for_updates,
            start_update_install
        ])
        .setup(|app| {
            // Register global shortcut
            let shortcut = Shortcut::new(Some(Modifiers::ALT | Modifiers::SHIFT), Code::KeyC);
            let _ = app.global_shortcut().register(shortcut);

            // 1. Create Menu items
            let show_i = MenuItemBuilder::with_id("show", "Show Widget").build(app)?;
            let hide_i = MenuItemBuilder::with_id("hide", "Hide Widget").build(app)?;
            let quit_i = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

            // 2. Build the menu
            let menu = MenuBuilder::new(app)
                .items(&[&show_i, &hide_i, &quit_i])
                .build()?;

            // 3. Build the tray with the event handlers
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let is_minimized = window.is_minimized().unwrap_or(false);
                            let is_visible = window.is_visible().unwrap_or(false);
                            if is_minimized {
                                let _ = window.unminimize();
                                let _ = window.show();
                                let _ = window.set_focus();
                            } else if is_visible {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .on_menu_event(|app, event| {
                    match event.id().as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "hide" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.hide();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            // 4. Start Windows Named Pipe IPC Server
            let handle = app.handle().clone();
            named_pipe::start_ipc_server(move |req| {
                let mut state = state_manager::load_state();
                let mut message = "OK".to_string();
                let mut success = true;

                match req {
                    IpcRequest::GetState => {
                        return IpcResponse {
                            success: true,
                            message: "State retrieved".to_string(),
                            data: Some(serde_json::to_value(&state).unwrap_or_default()),
                        };
                    }
                    IpcRequest::SetState(new_state) => {
                        state = new_state;
                    }
                    IpcRequest::CreateTimer { title, target_date, theme } => {
                        let new_preset = Preset {
                            id: format!("preset_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis()),
                            event_name: title,
                            target_date,
                            creation_date: "".to_string(),
                        };
                        state.presets.push(new_preset);
                        state.current_preset_index = (state.presets.len() as i32) - 1;
                        if let Some(t) = theme {
                            state.settings.theme = t;
                        }
                    }
                    IpcRequest::SwitchTimer { identifier } => {
                        if let Ok(idx) = identifier.parse::<usize>() {
                            if idx < state.presets.len() {
                                state.current_preset_index = idx as i32;
                            } else {
                                success = false;
                                message = "Timer index out of bounds".to_string();
                            }
                        } else if let Some(idx) = state.presets.iter().position(|p| p.id == identifier || p.event_name.to_lowercase().contains(&identifier.to_lowercase())) {
                            state.current_preset_index = idx as i32;
                        } else {
                            success = false;
                            message = "Timer not found".to_string();
                        }
                    }
                    IpcRequest::DeleteTimer { identifier } => {
                        let initial_len = state.presets.len();
                        state.presets.retain(|p| p.id != identifier && !p.event_name.to_lowercase().contains(&identifier.to_lowercase()));
                        if state.presets.len() < initial_len {
                            if state.presets.is_empty() {
                                state.current_preset_index = -1;
                            } else if state.current_preset_index >= state.presets.len() as i32 {
                                state.current_preset_index = (state.presets.len() as i32) - 1;
                            }
                        } else {
                            success = false;
                            message = "Timer not found".to_string();
                        }
                    }
                    IpcRequest::UpdateSettings { opacity, theme, always_on_top, autostart, window_visibility } => {
                        if let Some(op) = opacity {
                            state.settings.widget_opacity = op.clamp(20, 100);
                        }
                        if let Some(th) = theme {
                            state.settings.theme = th;
                        }
                        if let Some(aot) = always_on_top {
                            state.settings.always_on_top = aot;
                            if let Some(window) = handle.get_webview_window("main") {
                                let _ = window.set_always_on_top(aot);
                            }
                        }
                        if let Some(as_val) = autostart {
                            state.settings.autostart = as_val;
                            let _ = set_autostart(as_val);
                        }
                        if let Some(vis) = window_visibility {
                            if let Some(window) = handle.get_webview_window("main") {
                                match vis.as_str() {
                                    "show" => { let _ = window.show(); let _ = window.set_focus(); },
                                    "hide" => { let _ = window.hide(); },
                                    "minimize" => { let _ = window.minimize(); },
                                    "unminimize" => { let _ = window.unminimize(); let _ = window.show(); let _ = window.set_focus(); },
                                    _ => {}
                                }
                            }
                        }
                    }
                }

                if success {
                    let _ = state_manager::save_state(&state);
                    let _ = handle.emit("mcp-state-updated", serde_json::to_value(&state).unwrap_or_default());
                }

                IpcResponse {
                    success,
                    message,
                    data: Some(serde_json::to_value(&state).unwrap_or_default()),
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
