use std::io::{self, BufRead, Write};
use serde_json::{json, Value};
use crate::mcp_types::{AppState, IpcRequest, JsonRpcError, JsonRpcRequest, JsonRpcResponse, Preset};
use crate::named_pipe::send_ipc_request;
use crate::state_manager;

/// Entrypoint for stdio MCP server execution (`D-daycounter.exe --mcp`)
pub async fn run_mcp_server() {
    let stdin = io::stdin();
    let mut stdin_lock = stdin.lock();
    let stdout = io::stdout();

    let mut line = String::new();
    while let Ok(n) = stdin_lock.read_line(&mut line) {
        if n == 0 {
            break;
        }

        let trimmed = line.trim();
        if !trimmed.is_empty() {
            if let Ok(req) = serde_json::from_str::<JsonRpcRequest>(trimmed) {
                let resp = handle_mcp_request(req).await;
                if let Some(resp) = resp {
                    if let Ok(json_str) = serde_json::to_string(&resp) {
                        let mut out = stdout.lock();
                        let _ = writeln!(out, "{}", json_str);
                        let _ = out.flush();
                    }
                }
            }
        }
        line.clear();
    }
}

async fn handle_mcp_request(req: JsonRpcRequest) -> Option<JsonRpcResponse> {
    // Notifications don't require responses if id is None
    let id = req.id.clone();

    match req.method.as_str() {
        "initialize" => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "d-day-counter",
                    "version": env!("CARGO_PKG_VERSION")
                }
            })),
            error: None,
        }),

        "notifications/initialized" => None,

        "tools/list" => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "tools": [
                    {
                        "name": "create_timer",
                        "description": "Create a new countdown timer preset for an event, target date, or deadline.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "title": {
                                    "type": "string",
                                    "description": "Title or event identifier for the timer (e.g., 'Product Launch', 'Exam Start')"
                                },
                                "target_timestamp": {
                                    "type": "string",
                                    "description": "Absolute target date/time in ISO 8601 or YYYY-MM-DDTHH:mm format."
                                },
                                "duration_minutes": {
                                    "type": "integer",
                                    "description": "Relative duration in minutes from right now."
                                },
                                "duration_hours": {
                                    "type": "integer",
                                    "description": "Relative duration in hours from right now."
                                },
                                "duration_seconds": {
                                    "type": "integer",
                                    "description": "Relative duration in seconds from right now."
                                },
                                "theme": {
                                    "type": "string",
                                    "enum": ["theme-cyan", "theme-orange", "theme-green", "theme-magenta"],
                                    "description": "Optional neon theme accent for this timer."
                                }
                            },
                            "required": ["title"]
                        }
                    },
                    {
                        "name": "list_timers",
                        "description": "List all existing countdown timers, target timestamps, remaining time, and currently active timer index.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    },
                    {
                        "name": "switch_timer",
                        "description": "Switch the active timer shown on the desktop widget display.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "identifier": {
                                    "type": "string",
                                    "description": "Timer ID, index (e.g. '0', '1'), or title substring to display."
                                }
                            },
                            "required": ["identifier"]
                        }
                    },
                    {
                        "name": "delete_timer",
                        "description": "Delete a countdown timer preset.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "identifier": {
                                    "type": "string",
                                    "description": "Timer ID, index, or title substring to remove."
                                }
                            },
                            "required": ["identifier"]
                        }
                    },
                    {
                        "name": "update_settings",
                        "description": "Update D-Day Counter widget configurations (opacity, theme, always-on-top, autostart, window visibility).",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "opacity": {
                                    "type": "number",
                                    "description": "Widget opacity percentage (20 to 100). Value is safely clamped to prevent total transparency."
                                },
                                "theme": {
                                    "type": "string",
                                    "enum": ["theme-cyan", "theme-orange", "theme-green", "theme-magenta"]
                                },
                                "always_on_top": {
                                    "type": "boolean",
                                    "description": "Pin widget above all other desktop windows."
                                },
                                "autostart": {
                                    "type": "boolean",
                                    "description": "Launch app on system startup."
                                },
                                "window_visibility": {
                                    "type": "string",
                                    "enum": ["show", "hide", "minimize", "unminimize"],
                                    "description": "Show, hide, minimize, or bring desktop widget window to focus."
                                }
                            }
                        }
                    }
                ]
            })),
            error: None,
        }),

        "tools/call" => {
            let params = req.params.clone().unwrap_or(json!({}));
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

            let result = handle_tool_call(name, arguments).await;
            Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: match result {
                    Ok(val) => Some(json!({ "content": [{ "type": "text", "text": val }] })),
                    Err(err_msg) => Some(json!({ "content": [{ "type": "text", "text": format!("Error: {}", err_msg) }], "isError": true })),
                },
                error: None,
            })
        }

        _ => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code: -32601,
                message: format!("Method '{}' not found", req.method),
                data: None,
            }),
        }),
    }
}

async fn handle_tool_call(tool_name: &str, args: Value) -> Result<String, String> {
    match tool_name {
        "create_timer" => {
            let title = args.get("title").and_then(|v| v.as_str()).ok_or("Missing title")?.to_string();
            let theme = args.get("theme").and_then(|v| v.as_str()).map(|s| s.to_string());

            let target_date = if let Some(target_ts) = args.get("target_timestamp").and_then(|v| v.as_str()) {
                target_ts.to_string()
            } else {
                let secs_m = args.get("duration_minutes").and_then(|v| v.as_u64()).unwrap_or(0) * 60;
                let secs_h = args.get("duration_hours").and_then(|v| v.as_u64()).unwrap_or(0) * 3600;
                let secs_s = args.get("duration_seconds").and_then(|v| v.as_u64()).unwrap_or(0);
                let total_secs = secs_m + secs_h + secs_s;

                if total_secs == 0 {
                    return Err("Must specify target_timestamp or duration (minutes/hours/seconds)".to_string());
                }

                // Compute target ISO timestamp
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let target = now + total_secs;

                // Format as YYYY-MM-DDTHH:mm ISO string in UTC
                format_timestamp(target)
            };

            let ipc_req = IpcRequest::CreateTimer {
                title: title.clone(),
                target_date: target_date.clone(),
                theme,
            };

            // Attempt IPC live send first
            if let Ok(resp) = send_ipc_request(&ipc_req).await {
                if resp.success {
                    return Ok(format!("Timer '{}' created successfully (Target: {}). Live GUI updated.", title, target_date));
                }
            }

            // Fallback: Atomic state file modification
            let mut state = state_manager::load_state();
            let new_preset = Preset {
                id: format!("preset_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis()),
                event_name: title.clone(),
                target_date: target_date.clone(),
                creation_date: format_timestamp(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()),
            };
            state.presets.push(new_preset);
            state.current_preset_index = (state.presets.len() as i32) - 1;
            state_manager::save_state(&state)?;

            Ok(format!("Timer '{}' created successfully (Target: {}). Saved to state storage.", title, target_date))
        }

        "list_timers" => {
            let ipc_req = IpcRequest::GetState;
            let state = if let Ok(resp) = send_ipc_request(&ipc_req).await {
                if let Some(val) = resp.data {
                    serde_json::from_value::<AppState>(val).unwrap_or_else(|_| state_manager::load_state())
                } else {
                    state_manager::load_state()
                }
            } else {
                state_manager::load_state()
            };

            Ok(serde_json::to_string_pretty(&json!({
                "current_active_index": state.current_preset_index,
                "presets": state.presets,
                "settings": state.settings
            })).unwrap_or_default())
        }

        "switch_timer" => {
            let identifier = args.get("identifier").and_then(|v| v.as_str()).ok_or("Missing identifier")?.to_string();
            let ipc_req = IpcRequest::SwitchTimer { identifier: identifier.clone() };

            if let Ok(resp) = send_ipc_request(&ipc_req).await {
                if resp.success {
                    return Ok(format!("Switched active timer to '{}'. Live GUI updated.", identifier));
                }
            }

            let mut state = state_manager::load_state();
            if let Ok(idx) = identifier.parse::<usize>() {
                if idx < state.presets.len() {
                    state.current_preset_index = idx as i32;
                    state_manager::save_state(&state)?;
                    return Ok(format!("Switched active timer to index {}.", idx));
                }
            }

            for (idx, p) in state.presets.iter().enumerate() {
                if p.id == identifier || p.event_name.to_lowercase().contains(&identifier.to_lowercase()) {
                    state.current_preset_index = idx as i32;
                    state_manager::save_state(&state)?;
                    return Ok(format!("Switched active timer to '{}' (Index {}).", p.event_name, idx));
                }
            }

            Err(format!("Timer '{}' not found", identifier))
        }

        "delete_timer" => {
            let identifier = args.get("identifier").and_then(|v| v.as_str()).ok_or("Missing identifier")?.to_string();
            let ipc_req = IpcRequest::DeleteTimer { identifier: identifier.clone() };

            if let Ok(resp) = send_ipc_request(&ipc_req).await {
                if resp.success {
                    return Ok(format!("Deleted timer '{}'. Live GUI updated.", identifier));
                }
            }

            let mut state = state_manager::load_state();
            let initial_len = state.presets.len();
            state.presets.retain(|p| p.id != identifier && !p.event_name.to_lowercase().contains(&identifier.to_lowercase()));

            if state.presets.len() < initial_len {
                if state.presets.is_empty() {
                    state.current_preset_index = -1;
                } else if state.current_preset_index >= state.presets.len() as i32 {
                    state.current_preset_index = (state.presets.len() as i32) - 1;
                }
                state_manager::save_state(&state)?;
                Ok(format!("Deleted timer matching '{}'.", identifier))
            } else {
                Err(format!("Timer '{}' not found for deletion", identifier))
            }
        }

        "update_settings" => {
            // SCHEMA CLAMPING: Clamp opacity between 20 and 100
            let raw_opacity = args.get("opacity").and_then(|v| {
                if let Some(n) = v.as_u64() {
                    Some(n as u32)
                } else if let Some(f) = v.as_f64() {
                    if f <= 1.0 {
                        Some((f * 100.0) as u32)
                    } else {
                        Some(f as u32)
                    }
                } else {
                    None
                }
            });

            let clamped_opacity = raw_opacity.map(|op| op.clamp(20, 100));
            let theme = args.get("theme").and_then(|v| v.as_str()).map(|s| s.to_string());
            let always_on_top = args.get("always_on_top").and_then(|v| v.as_bool());
            let autostart = args.get("autostart").and_then(|v| v.as_bool());
            let window_visibility = args.get("window_visibility").and_then(|v| v.as_str()).map(|s| s.to_string());

            let ipc_req = IpcRequest::UpdateSettings {
                opacity: clamped_opacity,
                theme: theme.clone(),
                always_on_top,
                autostart,
                window_visibility: window_visibility.clone(),
            };

            if let Ok(resp) = send_ipc_request(&ipc_req).await {
                if resp.success {
                    return Ok(format!("Updated settings. Live GUI updated. (Opacity clamped to: {:?})", clamped_opacity));
                }
            }

            let mut state = state_manager::load_state();
            if let Some(op) = clamped_opacity {
                state.settings.widget_opacity = op;
            }
            if let Some(th) = theme {
                state.settings.theme = th;
            }
            if let Some(aot) = always_on_top {
                state.settings.always_on_top = aot;
            }
            if let Some(as_val) = autostart {
                state.settings.autostart = as_val;
            }

            state_manager::save_state(&state)?;
            Ok(format!("Settings updated in state file. Opacity: {}%", state.settings.widget_opacity))
        }

        _ => Err(format!("Unknown tool: {}", tool_name)),
    }
}

fn format_timestamp(unix_secs: u64) -> String {
    // Simple UTC ISO formatter: YYYY-MM-DDTHH:mm
    let days_since_epoch = unix_secs / 86400;
    let sec_in_day = unix_secs % 86400;

    let hour = sec_in_day / 3600;
    let minute = (sec_in_day % 3600) / 60;

    // Approximate date computation from epoch
    let mut days = days_since_epoch;
    let mut year = 1970;

    loop {
        let leap = if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) { 366 } else { 365 };
        if days < leap {
            break;
        }
        days -= leap;
        year += 1;
    }

    let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    let month_lengths = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

    let mut month = 1;
    for &m_len in &month_lengths {
        if days < m_len {
            break;
        }
        days -= m_len;
        month += 1;
    }
    let day = days + 1;

    format!("{:04}-{:02}-{:02}T{:02}:{:02}", year, month, day, hour, minute)
}
