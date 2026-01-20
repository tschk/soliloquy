//! Process manager for Chrome-like multi-process architecture

use std::collections::HashMap;
use std::process::{Command, Child};
use log::{info, debug, error, warn};
use tokio::sync::Mutex;

use super::TabId;
use crate::ipc::{RendererChannel, IpcServer};

/// Process manager for spawning and managing child processes
pub struct ProcessManager {
    /// Whether multi-process is enabled
    multi_process: bool,
    
    /// Active renderer processes (tab_id -> child process)
    renderers: Mutex<HashMap<TabId, ChildProcess>>,
    
    /// GPU process
    gpu_process: Mutex<Option<ChildProcess>>,
    
    /// Network process
    network_process: Mutex<Option<ChildProcess>>,
    
    /// IPC server for child process communication
    ipc_server: IpcServer,
}

/// Wrapper for child process
struct ChildProcess {
    #[allow(dead_code)]
    child: Child,
    channel_id: String,
}

impl ProcessManager {
    /// Create process manager for multi-process mode
    pub fn new_multi_process() -> Self {
        info!("Creating multi-process manager");
        ProcessManager {
            multi_process: true,
            renderers: Mutex::new(HashMap::new()),
            gpu_process: Mutex::new(None),
            network_process: Mutex::new(None),
            ipc_server: IpcServer::new(),
        }
    }
    
    /// Create process manager for single-process mode
    pub fn new_single_process() -> Self {
        info!("Creating single-process manager");
        ProcessManager {
            multi_process: false,
            renderers: Mutex::new(HashMap::new()),
            gpu_process: Mutex::new(None),
            network_process: Mutex::new(None),
            ipc_server: IpcServer::new(),
        }
    }
    
    /// Spawn a renderer process for a tab
    pub async fn spawn_renderer(&self, tab_id: TabId) -> Result<RendererChannel, String> {
        if self.multi_process {
            self.spawn_renderer_process(tab_id).await
        } else {
            // Single process mode - create in-process channel
            self.create_inprocess_renderer(tab_id).await
        }
    }
    
    /// Spawn an actual renderer subprocess
    async fn spawn_renderer_process(&self, tab_id: TabId) -> Result<RendererChannel, String> {
        let channel_id = format!("renderer-{}", tab_id.0);
        
        info!("Spawning renderer process for tab {} with channel {}", tab_id.0, channel_id);
        
        // Create IPC channel
        let channel = self.ipc_server.create_channel(&channel_id).await?;
        
        // Spawn child process
        let exe = std::env::current_exe()
            .map_err(|e| format!("Failed to get current exe: {}", e))?;
        
        let child = Command::new(exe)
            .arg("--type=renderer")
            .arg(format!("--channel-id={}", channel_id))
            .arg(format!("--tab-id={}", tab_id.0))
            .spawn()
            .map_err(|e| format!("Failed to spawn renderer: {}", e))?;
        
        // Store child process
        {
            let mut renderers = self.renderers.lock().await;
            renderers.insert(tab_id, ChildProcess {
                child,
                channel_id: channel_id.clone(),
            });
        }
        
        Ok(channel)
    }
    
    /// Create in-process renderer (single-process mode)
    async fn create_inprocess_renderer(&self, tab_id: TabId) -> Result<RendererChannel, String> {
        let channel_id = format!("inprocess-renderer-{}", tab_id.0);
        debug!("Creating in-process renderer for tab {}", tab_id.0);
        
        self.ipc_server.create_channel(&channel_id).await
    }
    
    /// Terminate a renderer process
    pub async fn terminate_renderer(&self, tab_id: TabId) {
        let mut renderers = self.renderers.lock().await;
        if let Some(mut process) = renderers.remove(&tab_id) {
            info!("Terminating renderer for tab {}", tab_id.0);
            let _ = process.child.kill();
            self.ipc_server.close_channel(&process.channel_id).await;
        }
    }
    
    /// Spawn GPU process (if not already running)
    pub async fn ensure_gpu_process(&self) -> Result<(), String> {
        if !self.multi_process {
            return Ok(());
        }
        
        let mut gpu = self.gpu_process.lock().await;
        if gpu.is_some() {
            return Ok(());
        }
        
        let channel_id = "gpu-process";
        info!("Spawning GPU process with channel {}", channel_id);
        
        let _ = self.ipc_server.create_channel(channel_id).await?;
        
        let exe = std::env::current_exe()
            .map_err(|e| format!("Failed to get current exe: {}", e))?;
        
        let child = Command::new(exe)
            .arg("--type=gpu")
            .arg(format!("--channel-id={}", channel_id))
            .spawn()
            .map_err(|e| format!("Failed to spawn GPU process: {}", e))?;
        
        *gpu = Some(ChildProcess {
            child,
            channel_id: channel_id.to_string(),
        });
        
        Ok(())
    }
    
    /// Spawn network process (if not already running)
    pub async fn ensure_network_process(&self) -> Result<(), String> {
        if !self.multi_process {
            return Ok(());
        }
        
        let mut network = self.network_process.lock().await;
        if network.is_some() {
            return Ok(());
        }
        
        let channel_id = "network-process";
        info!("Spawning network process with channel {}", channel_id);
        
        let _ = self.ipc_server.create_channel(channel_id).await?;
        
        let exe = std::env::current_exe()
            .map_err(|e| format!("Failed to get current exe: {}", e))?;
        
        let child = Command::new(exe)
            .arg("--type=network")
            .arg(format!("--channel-id={}", channel_id))
            .spawn()
            .map_err(|e| format!("Failed to spawn network process: {}", e))?;
        
        *network = Some(ChildProcess {
            child,
            channel_id: channel_id.to_string(),
        });
        
        Ok(())
    }
    
    /// Shutdown all processes
    pub async fn shutdown(&self) {
        info!("Shutting down all child processes");
        
        // Terminate all renderers
        let mut renderers = self.renderers.lock().await;
        for (tab_id, mut process) in renderers.drain() {
            debug!("Terminating renderer for tab {}", tab_id.0);
            let _ = process.child.kill();
        }
        
        // Terminate GPU process
        if let Some(mut process) = self.gpu_process.lock().await.take() {
            debug!("Terminating GPU process");
            let _ = process.child.kill();
        }
        
        // Terminate network process
        if let Some(mut process) = self.network_process.lock().await.take() {
            debug!("Terminating network process");
            let _ = process.child.kill();
        }
    }
}
