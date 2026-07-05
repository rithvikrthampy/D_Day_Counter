// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tauri_plugin_updater::UpdaterExt;
use std::sync::Mutex;

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

    // A matching line looks like:
    //     SciFiChronoWidget    REG_SZ    C:\path\to\app.exe
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

    // Refuse to register a transient build-output binary. Those live under
    // target\debug or target\release: the debug one needs the dev server and
    // both get wiped by `cargo clean`, so autostarting them shows a blank /
    // "can't reach this page" window at boot. Autostart must point at the
    // installed app.
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
    // Only report "enabled" when the registered path matches *this* running
    // executable. A stale entry pointing elsewhere (e.g. an old dev build)
    // reads as disabled, so re-enabling rewrites it to the correct path.
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
async fn check_for_updates(app: tauri::AppHandle, state: tauri::State<'_, UpdateState>) -> Result<bool, String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    if let Some(update) = updater.check().await.map_err(|e| e.to_string())? {
        let mut pending = state.pending_update.lock().unwrap();
        *pending = Some(update);
        Ok(true)
    } else {
        Ok(false)
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
        .manage(UpdateState {
            pending_update: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            set_autostart,
            is_autostart_enabled,
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
                            if window.is_visible().unwrap_or(false) {
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

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
