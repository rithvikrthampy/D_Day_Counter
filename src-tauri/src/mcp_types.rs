use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub id: String,
    pub event_name: String,
    pub target_date: String,   // ISO 8601 string or datetime-local format
    pub creation_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub theme: String,
    pub always_on_top: bool,
    pub widget_opacity: u32,
    pub autostart: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "theme-cyan".to_string(),
            always_on_top: false,
            widget_opacity: 100,
            autostart: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    pub presets: Vec<Preset>,
    pub current_preset_index: i32,
    pub settings: AppSettings,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            presets: vec![],
            current_preset_index: -1,
            settings: AppSettings::default(),
        }
    }
}

// IPC Request / Response definitions for Named Pipe communication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", content = "payload")]
pub enum IpcRequest {
    GetState,
    SetState(AppState),
    CreateTimer {
        title: String,
        target_date: String,
        theme: Option<String>,
    },
    SwitchTimer {
        identifier: String, // ID or title or index string
    },
    DeleteTimer {
        identifier: String,
    },
    UpdateSettings {
        opacity: Option<u32>,
        theme: Option<String>,
        always_on_top: Option<bool>,
        autostart: Option<bool>,
        window_visibility: Option<String>, // "show", "hide", "minimize", "unminimize"
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

// Standard Model Context Protocol (MCP) JSON-RPC Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}
