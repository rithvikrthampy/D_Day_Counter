// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|arg| arg == "--register-mcp") {
        let _ = tauri_app_lib::set_mcp_registration(true);
        return;
    }

    if args.iter().any(|arg| arg == "--unregister-mcp") {
        let _ = tauri_app_lib::set_mcp_registration(false);
        return;
    }

    if args.iter().any(|arg| arg == "--mcp") {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime for MCP server");
        rt.block_on(async {
            tauri_app_lib::run_mcp_server().await;
        });
    } else {
        tauri_app_lib::run();
    }
}
