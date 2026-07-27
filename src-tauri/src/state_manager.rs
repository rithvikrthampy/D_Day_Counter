use std::fs;
use std::path::PathBuf;
use crate::mcp_types::AppState;

/// Computes the path to `%LOCALAPPDATA%\d-daycounter\state.json`
pub fn get_state_file_path() -> PathBuf {
    let mut path = if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
        PathBuf::from(local_app_data)
    } else if let Some(home) = std::env::var_os("USERPROFILE") {
        PathBuf::from(home).join("AppData").join("Local")
    } else {
        PathBuf::from(".")
    };
    path.push("d-daycounter");
    let _ = fs::create_dir_all(&path);
    path.push("state.json");
    path
}

/// Loads app state from state.json
pub fn load_state() -> AppState {
    let path = get_state_file_path();
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(state) = serde_json::from_str::<AppState>(&content) {
                return state;
            }
        }
    }
    AppState::default()
}

/// Atomically saves state to state.json using a .tmp file and atomic rename
pub fn save_state(state: &AppState) -> Result<(), String> {
    let path = get_state_file_path();
    let tmp_path = path.with_extension("json.tmp");

    let json_content = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;

    // Step 1: Write to temporary file
    fs::write(&tmp_path, json_content).map_err(|e| format!("Failed to write tmp state file: {}", e))?;

    // Step 2: Atomic rename
    if let Err(e) = fs::rename(&tmp_path, &path) {
        // On Windows, if destination exists, rename can fail in some rare lock edge-cases.
        // Fallback: Remove old file then rename.
        let _ = fs::remove_file(&path);
        fs::rename(&tmp_path, &path).map_err(|e2| format!("Atomic rename failed: {} (original: {})", e2, e))?;
    }

    Ok(())
}
