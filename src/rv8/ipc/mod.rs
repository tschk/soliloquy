//! Inter-Process Communication for RV8
//!
//! This module provides IPC mechanisms for communication between
//! browser, renderer, GPU, and network processes.

use ipc_channel::ipc;
pub use ipc_channel::ipc::{IpcOneShotServer, IpcReceiver, IpcSender};
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::js::JsValue;

pub mod messages;

// Re-exports
pub use messages::*;

/// Create a new IPC channel pair
pub fn channel<T>() -> Result<(IpcSender<T>, IpcReceiver<T>), String>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    ipc::channel().map_err(|e| e.to_string())
}

/// IPC endpoint identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProcessType {
    Browser,
    Renderer(u64), // Tab ID
    Gpu,
    Network,
    Utility,
}

/// IPC connection manager
pub struct IpcRouter {
    /// Browser process channels
    browser_tx: Option<IpcSender<BrowserMessage>>,
    /// Renderer process channels (by tab ID)
    renderer_channels: HashMap<u64, IpcSender<RendererMessage>>,
    /// GPU process channel
    gpu_tx: Option<IpcSender<GpuMessage>>,
    /// Network process channel
    network_tx: Option<IpcSender<NetworkMessage>>,
}

impl IpcRouter {
    pub fn new() -> Self {
        IpcRouter {
            browser_tx: None,
            renderer_channels: HashMap::new(),
            gpu_tx: None,
            network_tx: None,
        }
    }

    /// Register browser process channel
    pub fn register_browser(&mut self, tx: IpcSender<BrowserMessage>) {
        info!("Registered browser process channel");
        self.browser_tx = Some(tx);
    }

    /// Register renderer process channel
    pub fn register_renderer(&mut self, tab_id: u64, tx: IpcSender<RendererMessage>) {
        info!("Registered renderer process for tab {}", tab_id);
        self.renderer_channels.insert(tab_id, tx);
    }

    /// Register GPU process channel
    pub fn register_gpu(&mut self, tx: IpcSender<GpuMessage>) {
        info!("Registered GPU process channel");
        self.gpu_tx = Some(tx);
    }

    /// Register network process channel
    pub fn register_network(&mut self, tx: IpcSender<NetworkMessage>) {
        info!("Registered network process channel");
        self.network_tx = Some(tx);
    }

    /// Unregister renderer when tab closes
    pub fn unregister_renderer(&mut self, tab_id: u64) {
        self.renderer_channels.remove(&tab_id);
        debug!("Unregistered renderer for tab {}", tab_id);
    }

    /// Send message to browser process
    pub fn send_to_browser(&self, msg: BrowserMessage) -> Result<(), String> {
        if let Some(tx) = &self.browser_tx {
            tx.send(msg)
                .map_err(|e| format!("Failed to send to browser: {}", e))
        } else {
            Err("Browser channel not registered".to_string())
        }
    }

    /// Send message to renderer process
    pub fn send_to_renderer(&self, tab_id: u64, msg: RendererMessage) -> Result<(), String> {
        if let Some(tx) = self.renderer_channels.get(&tab_id) {
            tx.send(msg)
                .map_err(|e| format!("Failed to send to renderer {}: {}", tab_id, e))
        } else {
            Err(format!("Renderer channel not found for tab {}", tab_id))
        }
    }

    /// Send message to GPU process
    pub fn send_to_gpu(&self, msg: GpuMessage) -> Result<(), String> {
        if let Some(tx) = &self.gpu_tx {
            tx.send(msg)
                .map_err(|e| format!("Failed to send to GPU: {}", e))
        } else {
            Err("GPU channel not registered".to_string())
        }
    }

    /// Send message to network process
    pub fn send_to_network(&self, msg: NetworkMessage) -> Result<(), String> {
        if let Some(tx) = &self.network_tx {
            tx.send(msg)
                .map_err(|e| format!("Failed to send to network: {}", e))
        } else {
            Err("Network channel not registered".to_string())
        }
    }

    /// Broadcast to all renderers
    pub fn broadcast_to_renderers(&self, msg: RendererMessage) {
        for (tab_id, tx) in &self.renderer_channels {
            if let Err(e) = tx.send(msg.clone()) {
                error!("Failed to broadcast to renderer {}: {}", tab_id, e);
            }
        }
    }
}

impl Default for IpcRouter {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RendererChannel {
    pub tab_id: u64,
    pub to_browser: IpcSender<BrowserMessage>,
}

impl RendererChannel {
    pub fn new(tab_id: u64, to_browser: IpcSender<BrowserMessage>) -> Self {
        RendererChannel { tab_id, to_browser }
    }

    pub fn send_navigate(&self, url: &str) -> Result<(), String> {
        self.to_browser
            .send(BrowserMessage::Navigate {
                tab_id: self.tab_id,
                url: url.to_string(),
            })
            .map_err(|e| e.to_string())
    }

    pub fn send_title_changed(&self, title: &str) -> Result<(), String> {
        self.to_browser
            .send(BrowserMessage::TitleChanged {
                tab_id: self.tab_id,
                title: title.to_string(),
            })
            .map_err(|e| e.to_string())
    }

    pub fn send_load_complete(&self) -> Result<(), String> {
        self.to_browser
            .send(BrowserMessage::LoadComplete {
                tab_id: self.tab_id,
            })
            .map_err(|e| e.to_string())
    }

    pub fn send_reload(&self) -> Result<(), String> {
        self.to_browser
            .send(BrowserMessage::Reload {
                tab_id: self.tab_id,
            })
            .map_err(|e| e.to_string())
    }

    pub fn send_stop(&self) -> Result<(), String> {
        self.to_browser
            .send(BrowserMessage::Stop {
                tab_id: self.tab_id,
            })
            .map_err(|e| e.to_string())
    }

    pub fn send_close(&self) -> Result<(), String> {
        self.to_browser
            .send(BrowserMessage::CloseTab {
                tab_id: self.tab_id,
            })
            .map_err(|e| e.to_string())
    }

    pub fn send_script_result(
        &self,
        callback_id: u64,
        result: Result<JsValue, String>,
    ) -> Result<(), String> {
        self.to_browser
            .send(BrowserMessage::ScriptResult {
                tab_id: self.tab_id,
                callback_id,
                result,
            })
            .map_err(|e| e.to_string())
    }
}

/// Client for communicating with a renderer process (from Browser)
pub struct RendererClient {
    pub tab_id: u64,
    pub tx: IpcSender<RendererMessage>,
}

impl RendererClient {
    pub fn new(tab_id: u64, tx: IpcSender<RendererMessage>) -> Self {
        RendererClient { tab_id, tx }
    }

    pub fn send_navigate(&self, url: &str) -> Result<(), String> {
        self.tx
            .send(RendererMessage::Navigate {
                url: url.to_string(),
            })
            .map_err(|e| e.to_string())
    }

    pub fn send_reload(&self) -> Result<(), String> {
        self.tx
            .send(RendererMessage::Reload)
            .map_err(|e| e.to_string())
    }

    pub fn send_stop(&self) -> Result<(), String> {
        self.tx
            .send(RendererMessage::Stop)
            .map_err(|e| e.to_string())
    }

    pub fn send_close(&self) -> Result<(), String> {
        self.tx
            .send(RendererMessage::Shutdown)
            .map_err(|e| e.to_string())
    }

    pub fn send_execute_script(&self, script: &str, callback_id: u64) -> Result<(), String> {
        self.tx
            .send(RendererMessage::ExecuteScript {
                script: script.to_string(),
                callback_id,
            })
            .map_err(|e| e.to_string())
    }
}

/// IPC Server for managing channels to child processes
pub struct IpcServer {
    channels: std::sync::Mutex<std::collections::HashMap<String, IpcSender<BrowserMessage>>>,
}

impl IpcServer {
    pub fn new() -> Self {
        IpcServer {
            channels: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// Create a bootstrap server for a new process
    pub fn create_bootstrap_server(
    ) -> Result<(String, IpcOneShotServer<IpcSender<RendererMessage>>), String> {
        let (server, name) = IpcOneShotServer::new()
            .map_err(|e| format!("Failed to create bootstrap server: {}", e))?;
        Ok((name, server))
    }

    /// Create a channel for in-process or manual connection
    /// Returns the renderer channel wrapper and the receiver for the browser
    pub fn create_channel(
        &self,
        channel_id: &str,
    ) -> Result<(RendererChannel, IpcReceiver<BrowserMessage>), String> {
        let (tx, rx) = ipc::channel().map_err(|e| e.to_string())?;

        {
            let mut channels = self.channels.lock().unwrap();
            channels.insert(channel_id.to_string(), tx.clone());
        }

        // Extract tab_id from channel name
        let tab_id = channel_id
            .split('-')
            .last()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);

        Ok((RendererChannel::new(tab_id, tx), rx))
    }

    pub async fn close_channel(&self, channel_id: &str) {
        let mut channels = self.channels.lock().unwrap();
        channels.remove(channel_id);
    }
}

impl Default for IpcServer {
    fn default() -> Self {
        Self::new()
    }
}
