use std::fs;
use std::path::PathBuf;
use serde_json::{json, Value};
use std::thread::sleep;
use std::time::Duration;

/// Returns a list of target AI application MCP configuration paths on Windows.
pub fn get_ai_config_paths() -> Vec<(String, PathBuf)> {
    let mut targets = Vec::new();

    let appdata = std::env::var_os("APPDATA").map(PathBuf::from);
    let userprofile = std::env::var_os("USERPROFILE").map(PathBuf::from);

    // 1. Claude Desktop
    if let Some(ref path) = appdata {
        let claude_path = path.join("Claude").join("claude_desktop_config.json");
        targets.push(("Claude Desktop".to_string(), claude_path));
    }

    // 2. Cursor (User globalStorage)
    if let Some(ref path) = appdata {
        let cursor_path = path.join("Cursor").join("User").join("globalStorage").join("mcp_config.json");
        targets.push(("Cursor".to_string(), cursor_path));
    }

    // 3. Cursor (.cursor home)
    if let Some(ref path) = userprofile {
        let cursor_home = path.join(".cursor").join("mcp.json");
        targets.push(("Cursor (Home)".to_string(), cursor_home));
    }

    // 4. Antigravity / Gemini IDE
    if let Some(ref path) = userprofile {
        let antigravity_path = path.join(".gemini").join("antigravity-ide").join("mcp_config.json");
        targets.push(("Antigravity / Gemini IDE".to_string(), antigravity_path));
    }

    targets
}

/// Automatically registers or unregisters D-Day Counter in installed AI configuration files.
pub fn set_mcp_registration(enable: bool) -> Result<Vec<String>, String> {
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("Failed to resolve executable path: {}", e))?
        .to_string_lossy()
        .to_string();

    let targets = get_ai_config_paths();
    let mut updated_apps = Vec::new();

    for (app_name, config_path) in targets {
        let parent_dir = match config_path.parent() {
            Some(p) => p,
            None => continue,
        };

        // If configuring ON: proceed if file exists OR parent app folder exists (meaning app is installed)
        // If configuring OFF: proceed only if file exists
        if enable {
            if !parent_dir.exists() && !config_path.exists() {
                continue; // App is not installed on this machine
            }
        } else {
            if !config_path.exists() {
                continue;
            }
        }

        // Retry loop for transient file lock contention (3 attempts with 500ms delay)
        let mut attempts = 0;
        let max_attempts = 3;
        let mut success = false;
        let mut last_error = String::new();

        while attempts < max_attempts && !success {
            attempts += 1;

            let mut root_json: Value = if config_path.exists() {
                match fs::read_to_string(&config_path) {
                    Ok(content) => {
                        // Use string-literal-aware JSONC parser (json5) to preserve URLs like https:// while stripping comments
                        match json5::from_str::<Value>(&content) {
                            Ok(val) => val,
                            Err(_) => {
                                // Fallback: try standard serde_json
                                serde_json::from_str::<Value>(&content).unwrap_or_else(|_| json!({}))
                            }
                        }
                    }
                    Err(e) => {
                        last_error = e.to_string();
                        sleep(Duration::from_millis(500));
                        continue;
                    }
                }
            } else {
                json!({})
            };

            if !root_json.is_object() {
                root_json = json!({});
            }

            if enable {
                // Ensure mcpServers key exists as an object
                if root_json.get("mcpServers").is_none() || !root_json["mcpServers"].is_object() {
                    root_json["mcpServers"] = json!({});
                }

                let mcp_servers = root_json.get_mut("mcpServers").unwrap();
                mcp_servers["d-day-counter"] = json!({
                    "command": current_exe,
                    "args": ["--mcp"]
                });
            } else {
                if let Some(mcp_servers) = root_json.get_mut("mcpServers") {
                    if let Some(map) = mcp_servers.as_object_mut() {
                        map.remove("d-day-counter");
                    }
                }
            }

            // Write updated JSON atomically using a .tmp file + rename / copy fallback
            let tmp_path = config_path.with_extension("json.tmp");
            if let Ok(pretty_str) = serde_json::to_string_pretty(&root_json) {
                let _ = fs::create_dir_all(parent_dir);
                if fs::write(&tmp_path, &pretty_str).is_ok() {
                    let mut write_success = fs::rename(&tmp_path, &config_path).is_ok();
                    if !write_success {
                        let _ = fs::remove_file(&config_path);
                        write_success = fs::rename(&tmp_path, &config_path).is_ok();
                    }
                    if !write_success {
                        write_success = fs::copy(&tmp_path, &config_path).is_ok();
                        let _ = fs::remove_file(&tmp_path);
                    }

                    if write_success {
                        success = true;
                        updated_apps.push(app_name.clone());
                        break;
                    }
                }
            }


            sleep(Duration::from_millis(500));
        }

        if !success && !last_error.is_empty() {
            eprintln!("Failed to update {} config after {} retries: {}", app_name, max_attempts, last_error);
        }
    }

    Ok(updated_apps)
}

/// Checks if d-day-counter is currently registered in any detected AI config files.
pub fn is_mcp_registered() -> bool {
    let targets = get_ai_config_paths();
    for (_, config_path) in targets {
        if config_path.exists() {
            if let Ok(content) = fs::read_to_string(&config_path) {
                let parsed: Result<Value, _> = json5::from_str(&content).or_else(|_| serde_json::from_str(&content));
                if let Ok(root) = parsed {
                    if let Some(mcp_servers) = root.get("mcpServers") {
                        if mcp_servers.get("d-day-counter").is_some() {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}
