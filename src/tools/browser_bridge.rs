use base64::{Engine as _, engine::general_purpose};
use crate::Result;
use crate::tools::Tool;
use async_trait::async_trait;
use serde_json::json;
use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tracing::{info, warn, debug};

/// A tool that acts as a bridge to a Chrome Extension via WebSocket
#[derive(Clone)]
pub struct BrowserBridgeTool {
    // Channel to send commands to the active connection handler
    command_sender: Arc<Mutex<Option<mpsc::UnboundedSender<String>>>>,
    // Latest state/content received from browser
    last_content: Arc<Mutex<Option<String>>>,
}

impl BrowserBridgeTool {
    pub fn new() -> Self {
        let tool = Self {
            command_sender: Arc::new(Mutex::new(None)),
            last_content: Arc::new(Mutex::new(None)),
        };
        
        // Start the WebSocket server in the background
        tool.start_server();
        
        tool
    }

    fn start_server(&self) {
        let sender_store = self.command_sender.clone();
        let content_store = self.last_content.clone();

        tokio::spawn(async move {
            let addr = "127.0.0.1:2345";
            let mut retry_count = 0;
            let max_retries = 5;
            
            let listener = loop {
                match TcpListener::bind(&addr).await {
                    Ok(l) => {
                        info!("Browser Bridge listening on: {}", addr);
                        break l;
                    },
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::AddrInUse && retry_count < max_retries {
                            retry_count += 1;
                            // Only log first attempt, then stay quiet
                            if retry_count == 1 {
                                debug!("Browser Bridge port {} in use, waiting...", addr);
                            }
                            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                            continue;
                        }
                        // After retries exhausted, just note it's not available
                        warn!("Browser Bridge unavailable (port {} in use) - browser tool disabled", addr);
                        return;
                    }
                }
            };

            while let Ok((stream, _)) = listener.accept().await {
                info!("New browser connection incoming");
                let sender_store = sender_store.clone();
                let content_store = content_store.clone();
                
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, sender_store, content_store).await {
                        debug!("Browser connection ended: {}", e);
                    }
                });
            }
        });
    }
}

async fn handle_connection(
    stream: TcpStream, 
    sender_store: Arc<Mutex<Option<mpsc::UnboundedSender<String>>>>,
    content_store: Arc<Mutex<Option<String>>>
) -> Result<()> {
    let ws_stream = accept_async(stream).await.map_err(|e| anyhow::anyhow!("Failed to accept WS: {}", e))?;
    info!("WebSocket connection established");

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    
    // Create channel for this connection
    let (tx, mut rx) = mpsc::unbounded_channel();
    
    // Store the sender so the Tool execute method can use it
    {
        let mut guard = sender_store.lock().unwrap();
        *guard = Some(tx);
    } // Drop lock

    // Loop to handle incoming messages (browser -> leo) and outgoing commands (leo -> browser)
    loop {
        tokio::select! {
            // Receive from browser
            msg = ws_receiver.next() => {
                match msg {
                    Some(Ok(WsMessage::Text(text))) => {
                        // info!("Received from browser: {}", text); // Too noisy for large messages
                        
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                            if json["type"] == "result" && (json["action"] == "screenshot" || json["action"] == "moment") {
                                if let Some(screenshot_b64) = json["data"]["screenshot"].as_str() {
                                    // Remove data:image/png;base64, prefix
                                    let b64_data = screenshot_b64.split(",").nth(1).unwrap_or(screenshot_b64);
                                    
                                    if let Ok(bytes) = general_purpose::STANDARD.decode(b64_data) {
                                        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
                                        let slug = if json["action"] == "moment" {
                                            json["data"]["page"]["title"].as_str().unwrap_or("snapshot")
                                                .chars().filter(|c| c.is_alphanumeric() || *c == ' ').collect::<String>()
                                                .replace(" ", "_").to_lowercase()
                                        } else {
                                            "screenshot".to_string()
                                        };
                                        
                                        let moments_dir = dirs::home_dir().unwrap_or_default().join(".leo").join("moments").join(&slug).join(&timestamp);
                                        std::fs::create_dir_all(&moments_dir).ok();
                                        
                                        let img_path = moments_dir.join("screenshot.png");
                                        std::fs::write(&img_path, bytes).ok();
                                        info!("Saved {} to {:?}", json["action"], img_path);
                                        
                                        if json["action"] == "moment" {
                                            if let Some(page) = json["data"]["page"].as_object() {
                                                let meta_path = moments_dir.join("metadata.json");
                                                std::fs::write(&meta_path, serde_json::to_string_pretty(page).unwrap()).ok();
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        let mut guard = content_store.lock().unwrap();
                        *guard = Some(text.to_string());
                    }
                    Some(Ok(WsMessage::Close(_))) => {
                        info!("Browser closed connection");
                        break;
                    }
                    Some(Err(e)) => {
                        debug!("WebSocket error: {}", e);
                        break;
                    }
                    None => break,
                    _ => {}
                }
            }
            
            // Send to browser
            cmd = rx.recv() => {
                match cmd {
                    Some(text) => {
                        info!("Sending to browser: {}", text);
                        ws_sender.send(WsMessage::Text(text.into())).await.ok();
                    }
                    None => break, // Channel closed
                }
            }
        }
    }
    
    // Cleanup
    let mut guard = sender_store.lock().unwrap();
    *guard = None;
    info!("Connection handler finished");
    
    Ok(())
}

#[async_trait]
impl Tool for BrowserBridgeTool {
    fn name(&self) -> &str {
        "browser"
    }

    fn description(&self) -> &str {
        "Control Chrome via extension. Actions: open, search, click, type, read, scroll, screenshot, moment (snapshot)."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["open", "click", "type", "read", "scroll", "search", "screenshot", "moment", "get_elements", "wait"],
                    "description": "Action to perform. 'moment' captures screenshot + text. 'get_elements' lists buttons/links."
                },
                "url": { "type": "string", "description": "URL to open (for 'open')" },
                "query": { "type": "string", "description": "Search query (for 'search')" },
                "selector": { "type": "string", "description": "CSS selector OR visible text/label (e.g. 'Find Jobs', '#search-btn')" },
                "text": { "type": "string", "description": "Text to type" },
                "y": { "type": "number", "description": "Pixels to scroll down (default 500)" },
                "ms": { "type": "number", "description": "Milliseconds to wait (for 'wait')" },
                "max_length": { "type": "number", "description": "Max text length for 'read' (default 10000)" }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<String> {
        let action = args["action"].as_str().unwrap_or("help").to_string();
        
        // Handle 'search' by converting it to an 'open' action
        let final_args = if action == "search" {
            if let Some(query) = args["query"].as_str() {
                let encoded = url::form_urlencoded::byte_serialize(query.as_bytes()).collect::<String>();
                json!({
                    "action": "open",
                    "url": format!("https://www.google.com/search?q={}", encoded)
                })
            } else {
                return Ok("Error: 'query' parameter required for search action".to_string());
            }
        } else {
            args
        };

        // Construct the JSON command to send to the extension
        let command = serde_json::to_string(&final_args)?;
        
        let tx = {
            let guard = self.command_sender.lock().unwrap();
            guard.clone()
        };

        if let Some(sender) = tx {
            sender.send(command).map_err(|_| anyhow::anyhow!("Failed to send command to browser"))?;
            
            if action == "search" {
                Ok("Search results opened in browser! Check your browser tabs.".to_string())
            } else {
                Ok(format!("Browser action '{}' sent!", action))
            }
        } else {
            Err(anyhow::anyhow!("No browser connected! Please install the Leo Link extension and ensure Chrome is open.").into())
        }
    }
}
