use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::windows::named_pipe::{ClientOptions, ServerOptions};
use crate::mcp_types::{IpcRequest, IpcResponse};
use std::time::Duration;

pub const PIPE_NAME: &str = r"\\.\pipe\d-day-counter-ipc";

/// Spawns the async Named Pipe IPC server task inside Tauri's Tokio runtime.
/// When a request is received, it invokes the provided `handler` callback.
pub fn start_ipc_server<F>(handler: F)
where
    F: Fn(IpcRequest) -> IpcResponse + Send + Sync + 'static,
{
    let handler = std::sync::Arc::new(handler);

    tauri::async_runtime::spawn(async move {
        let mut first_instance = true;
        loop {
            let server_res = if first_instance {
                ServerOptions::new()
                    .first_pipe_instance(true)
                    .create(PIPE_NAME)
            } else {
                ServerOptions::new().create(PIPE_NAME)
            };

            match server_res {
                Ok(mut server) => {
                    first_instance = false;
                    if server.connect().await.is_ok() {
                        let handler_clone = handler.clone();
                        tauri::async_runtime::spawn(async move {

                            let mut buffer = Vec::new();
                            let mut tmp = [0u8; 4096];

                            loop {
                                match server.read(&mut tmp).await {
                                    Ok(0) => break, // Connection closed
                                    Ok(n) => {
                                        buffer.extend_from_slice(&tmp[..n]);
                                        // Attempt to parse JSON message from buffer
                                        if let Ok(req) = serde_json::from_slice::<IpcRequest>(&buffer) {
                                            let resp = handler_clone(req);
                                            if let Ok(resp_bytes) = serde_json::to_vec(&resp) {
                                                let _ = server.write_all(&resp_bytes).await;
                                                let _ = server.flush().await;
                                            }
                                            break;
                                        }
                                    }
                                    Err(_) => break,
                                }
                            }
                        });
                    }
                }
                Err(e) => {
                    eprintln!("Failed to create IPC named pipe: {}", e);
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }
    });
}

/// Attempts to send an IPC request over the Named Pipe to the running GUI app.
/// Returns `Ok(IpcResponse)` if the GUI is online and responds, or `Err` if GUI is offline.
pub async fn send_ipc_request(request: &IpcRequest) -> Result<IpcResponse, String> {
    let client = ClientOptions::new()
        .open(PIPE_NAME)
        .map_err(|e| format!("Named pipe connect error: {}", e))?;

    let (mut reader, mut writer) = tokio::io::split(client);

    let req_bytes = serde_json::to_vec(request).map_err(|e| e.to_string())?;

    // Send request
    writer
        .write_all(&req_bytes)
        .await
        .map_err(|e| format!("Named pipe write error: {}", e))?;
    writer.flush().await.map_err(|e| e.to_string())?;

    // Read response with 3-second timeout
    let read_future = async {
        let mut buffer = Vec::new();
        let mut tmp = [0u8; 4096];
        loop {
            match reader.read(&mut tmp).await {
                Ok(0) => break,
                Ok(n) => {
                    buffer.extend_from_slice(&tmp[..n]);
                    if let Ok(resp) = serde_json::from_slice::<IpcResponse>(&buffer) {
                        return Ok(resp);
                    }
                }
                Err(e) => return Err(e.to_string()),
            }
        }
        Err("Incomplete IPC response".to_string())
    };

    match tokio::time::timeout(Duration::from_secs(3), read_future).await {
        Ok(res) => res,
        Err(_) => Err("IPC timeout".to_string()),
    }
}
