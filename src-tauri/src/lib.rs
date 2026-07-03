// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![set_autostart, is_autostart_enabled])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
