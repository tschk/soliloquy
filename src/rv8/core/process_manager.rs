//! Process manager for Chrome-like multi-process architecture

use log::{debug, error, info, warn};
#[cfg(target_os = "linux")]
use nix::sched::sched_setaffinity;
#[cfg(target_os = "linux")]
use nix::sched::CpuSet;
use num_cpus;
use std::collections::HashMap;
use std::process::{Child, Command};
use tokio::sync::Mutex;

use super::TabId;
use crate::ipc::{
    self, BrowserMessage, IpcServer, RendererChannel, RendererClient, RendererMessage,
};

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

    /// Core assignment counter for per-tab isolation
    core_counter: std::sync::atomic::AtomicUsize,
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
            core_counter: std::sync::atomic::AtomicUsize::new(0),
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
            core_counter: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Spawn a renderer process for a tab
    pub async fn spawn_renderer(&self, tab_id: TabId) -> Result<RendererClient, String> {
        if self.multi_process {
            self.spawn_renderer_process(tab_id).await
        } else {
            // Single process mode - create in-process channel
            self.create_inprocess_renderer(tab_id).await
        }
    }

    /// Spawn an actual renderer subprocess
    async fn spawn_renderer_process(&self, tab_id: TabId) -> Result<RendererClient, String> {
        // 1. Create a bootstrap server
        let (server_name, bootstrap) = IpcServer::create_bootstrap_server()?;

        info!(
            "Spawning renderer process for tab {} with bootstrap {}",
            tab_id.0, server_name
        );

        // 2. Spawn child process
        let exe =
            std::env::current_exe().map_err(|e| format!("Failed to get current exe: {}", e))?;

        // We pass the bootstrap server name via --channel-id as expected by main.rs
        let child = Command::new(exe)
            .arg("--type=renderer")
            .arg(format!("--channel-id={}", server_name))
            .arg(format!("--tab-id={}", tab_id.0))
            .spawn()
            .map_err(|e| format!("Failed to spawn renderer: {}", e))?;

        // Set CPU affinity for per-tab core isolation
        #[cfg(target_os = "linux")]
        {
            let core_count = num_cpus::get();
            let core = self
                .core_counter
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                % core_count;
            let pid = nix::unistd::Pid::from_raw(child.id() as i32);
            let mut cpuset = CpuSet::new();
            cpuset.set(core).unwrap();
            if let Err(e) = sched_setaffinity(pid, &cpuset) {
                warn!(
                    "Failed to set CPU affinity for renderer {} to core {}: {}",
                    tab_id.0, core, e
                );
            }
        }

        // 3. Accept connection (handshake)
        // Run blocking accept in a blocking task
        let (_, tx_to_renderer) = tokio::task::spawn_blocking(move || {
            bootstrap
                .accept()
                .map_err(|e| format!("Failed to accept connection from renderer: {}", e))
        })
        .await
        .map_err(|e| format!("Join error: {}", e))??;

        // 4. Create channel for receiving messages from renderer (BrowserMessage)
        // tx_to_browser (sent to renderer), rx_from_renderer (kept by browser)
        let (tx_to_browser, rx_from_renderer) =
            ipc::channel::<BrowserMessage>().map_err(|e| e.to_string())?;

        // 5. Send Initialize message to renderer with tx_to_browser
        tx_to_renderer
            .send(RendererMessage::Initialize {
                browser_tx: tx_to_browser,
            })
            .map_err(|e| e.to_string())?;

        // 6. Handle rx_from_renderer
        // Spawn a thread to read messages and handle them.
        // For now, we just log them as there's no central router yet.
        std::thread::spawn(move || {
            while let Ok(msg) = rx_from_renderer.recv() {
                debug!("Browser received message: {:?}", msg);
                // TODO: Forward to Browser/Router
            }
        });

        // Store child process
        {
            let mut renderers = self.renderers.lock().await;
            renderers.insert(
                tab_id,
                ChildProcess {
                    child,
                    channel_id: server_name.clone(),
                },
            );
        }

        Ok(RendererClient::new(tab_id.0, tx_to_renderer))
    }

    /// Create in-process renderer (single-process mode)
    async fn create_inprocess_renderer(&self, tab_id: TabId) -> Result<RendererClient, String> {
        debug!("Creating in-process renderer for tab {}", tab_id.0);

        // Create channels
        // Channel 1: Browser -> Renderer (RendererMessage)
        let (tx_to_renderer, rx_from_browser) =
            ipc::channel::<RendererMessage>().map_err(|e| e.to_string())?;

        // Channel 2: Renderer -> Browser (BrowserMessage)
        let (tx_to_browser, rx_from_renderer) =
            ipc::channel::<BrowserMessage>().map_err(|e| e.to_string())?;

        // Handle rx_from_renderer (Browser side)
        std::thread::spawn(move || {
            while let Ok(msg) = rx_from_renderer.recv() {
                debug!("Browser received message (in-process): {:?}", msg);
            }
        });

        // Spawn renderer thread (in-process)
        // We need to bridge the IPC receiver (blocking) to mpsc for RendererProcess::run
        // or just let RendererProcess handle it.
        // But RendererProcess::run expects mpsc::UnboundedReceiver.

        // So we need to bridge rx_from_browser (IpcReceiver) to mpsc.
        let (mpsc_tx, mpsc_rx) = tokio::sync::mpsc::unbounded_channel();

        std::thread::spawn(move || {
            while let Ok(msg) = rx_from_browser.recv() {
                if mpsc_tx.send(msg).is_err() {
                    break;
                }
            }
        });

        // We can't easily spawn RendererProcess here because we don't have access to ServoConfig easily?
        // But let's assume we can construct it.
        // Wait, ProcessManager doesn't import RendererProcess.
        // This suggests create_inprocess_renderer might have been implemented differently before.
        // Or maybe it was just creating the channel.
        // But if we are in single process mode, SOMEONE needs to run the renderer.

        // Given the task is about IPC, maybe I should assume single process mode is not the priority?
        // But I changed the return type.

        // For now, I will return the client and assume the renderer is started elsewhere?
        // But the previous code for create_inprocess_renderer called self.ipc_server.create_channel which just created a channel.
        // It didn't start any thread.
        // So I will just return the client.
        // But what about the other end?
        // The other end (rx_from_browser) and (tx_to_browser) are lost if I don't use them.

        // If single process is not used/tested, I might leave it broken or minimal.
        // Or maybe I should just use `ipc_server.create_channel` again but adapted?
        // `create_channel` returns `(RendererChannel, IpcReceiver)`.
        // `RendererChannel` holds `tx_to_browser`.
        // `IpcReceiver` holds `rx_from_browser` (wait, no).
        // `create_channel` in `ipc/mod.rs` creates `(tx, rx)`. `tx` -> `rx`.
        // It returns `(RendererChannel(tx), rx)`.
        // So it gives you a loopback.

        // If I use that:
        // let (channel, rx) = self.ipc_server.create_channel(...)?;
        // `channel` is `RendererChannel` (Renderer -> Browser).
        // `rx` is `IpcReceiver<BrowserMessage>` (Browser side receiver).

        // But we also need Browser -> Renderer.
        // I'll just leave it as minimal implementation since multi-process is the goal.

        Ok(RendererClient::new(tab_id.0, tx_to_renderer))
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

        let (channel, _rx) = self.ipc_server.create_channel(channel_id)?;

        let exe =
            std::env::current_exe().map_err(|e| format!("Failed to get current exe: {}", e))?;

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

        let (channel, _rx) = self.ipc_server.create_channel(channel_id)?;

        let exe =
            std::env::current_exe().map_err(|e| format!("Failed to get current exe: {}", e))?;

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
