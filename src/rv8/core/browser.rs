//! Main Browser struct - the coordinator for all browser functionality

use log::info;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use super::{BrowserConfig, ProcessManager, Tab, TabId};
use crate::compositor::Compositor;
use crate::networking::NetworkManager;
use crate::optimizations::{OptimizationFlags, PerformanceMonitor};
use crate::storage::StorageManager;

/// Main browser instance
pub struct Browser {
    /// Browser configuration
    config: BrowserConfig,

    /// Active tabs
    tabs: RwLock<HashMap<TabId, Arc<Mutex<Tab>>>>,

    /// Currently focused tab
    active_tab: RwLock<Option<TabId>>,

    /// Next tab ID counter
    next_tab_id: std::sync::atomic::AtomicU64,

    /// Process manager for child processes
    process_manager: Arc<ProcessManager>,

    /// GPU compositor
    compositor: Arc<Compositor>,

    /// Network manager
    network: Arc<NetworkManager>,

    /// Storage manager
    storage: Arc<StorageManager>,

    /// Performance monitor
    perf_monitor: Arc<PerformanceMonitor>,

    /// Optimization flags
    opt_flags: OptimizationFlags,

    /// Shutdown signal
    shutdown: tokio::sync::broadcast::Sender<()>,
}

impl Browser {
    /// Create a new browser instance
    pub async fn new(config: BrowserConfig) -> Result<Self, String> {
        info!("Initializing RV8 browser...");

        // Initialize storage first (needed for cookies, cache)
        let storage = StorageManager::new(&config.data_dirs.profile_dir)
            .await
            .map_err(|e| format!("Failed to init storage: {}", e))?;
        let storage = Arc::new(storage);
        info!("Storage manager initialized");

        // Initialize network manager
        let network = NetworkManager::new(storage.clone())
            .await
            .map_err(|e| format!("Failed to init network: {}", e))?;
        let network = Arc::new(network);
        info!("Network manager initialized");

        // Initialize process manager
        let process_manager = if config.multi_process {
            ProcessManager::new_multi_process()
        } else {
            ProcessManager::new_single_process()
        };
        let process_manager = Arc::new(process_manager);
        info!(
            "Process manager initialized (multi_process={})",
            config.multi_process
        );

        // Initialize compositor
        let compositor = Compositor::new(&config)
            .await
            .map_err(|e| format!("Failed to init compositor: {}", e))?;
        let compositor = Arc::new(compositor);
        info!("Compositor initialized");

        // Performance monitor
        let perf_monitor = Arc::new(PerformanceMonitor::new());

        // Optimization flags
        let opt_flags = OptimizationFlags::chrome_like();

        let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);

        Ok(Browser {
            config,
            tabs: RwLock::new(HashMap::new()),
            active_tab: RwLock::new(None),
            next_tab_id: std::sync::atomic::AtomicU64::new(1),
            process_manager,
            compositor,
            network,
            storage,
            perf_monitor,
            opt_flags,
            shutdown: shutdown_tx,
        })
    }

    /// Create a new tab and navigate to the given URL
    pub async fn new_tab(&mut self, url: &str) -> Result<TabId, String> {
        let tab_id = TabId(
            self.next_tab_id
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst),
        );

        info!("Creating new tab {} with URL: {}", tab_id.0, url);

        // Create renderer process for this tab
        let renderer_channel = self.process_manager.spawn_renderer(tab_id).await?;

        // Create tab
        let tab = Tab::new(
            tab_id,
            url.to_string(),
            renderer_channel,
            self.network.clone(),
        )
        .await?;

        // Store tab
        {
            let mut tabs = self.tabs.write().await;
            tabs.insert(tab_id, Arc::new(Mutex::new(tab)));
        }

        // Set as active if first tab
        {
            let mut active = self.active_tab.write().await;
            if active.is_none() {
                *active = Some(tab_id);
            }
        }

        // Navigate to URL
        self.navigate_tab(tab_id, url).await?;

        Ok(tab_id)
    }

    /// Navigate a tab to a URL
    pub async fn navigate_tab(&self, tab_id: TabId, url: &str) -> Result<(), String> {
        let tabs = self.tabs.read().await;
        let tab = tabs
            .get(&tab_id)
            .ok_or_else(|| format!("Tab {} not found", tab_id.0))?;

        let mut tab = tab.lock().await;
        tab.navigate(url).await?;
        drop(tab);
        drop(tabs);

        self.compositor.request_frame().await;
        Ok(())
    }

    /// Navigate the active tab to a URL
    pub async fn navigate(&self, url: &str) -> Result<(), String> {
        let active = self.active_tab.read().await;
        let tab_id = active.ok_or("No active tab")?;
        drop(active);

        self.navigate_tab(tab_id, url).await
    }

    /// Close a tab
    pub async fn close_tab(&mut self, tab_id: TabId) -> Result<(), String> {
        info!("Closing tab {}", tab_id.0);

        let mut tabs = self.tabs.write().await;
        if let Some(tab) = tabs.remove(&tab_id) {
            let tab = tab.lock().await;
            tab.close().await;
        }

        // Update active tab
        let mut active = self.active_tab.write().await;
        let was_active = *active == Some(tab_id);
        if *active == Some(tab_id) {
            *active = tabs.keys().next().copied();
        }
        drop(active);
        drop(tabs);

        if was_active {
            self.compositor.request_frame().await;
        }

        // Terminate renderer process
        self.process_manager.terminate_renderer(tab_id).await;

        Ok(())
    }

    /// Get the active tab ID
    pub async fn active_tab(&self) -> Option<TabId> {
        *self.active_tab.read().await
    }

    /// Set the active tab
    pub async fn set_active_tab(&self, tab_id: TabId) -> Result<(), String> {
        let tabs = self.tabs.read().await;
        if !tabs.contains_key(&tab_id) {
            return Err(format!("Tab {} not found", tab_id.0));
        }
        drop(tabs);

        let mut active = self.active_tab.write().await;
        *active = Some(tab_id);
        drop(active);

        self.compositor.request_frame().await;

        Ok(())
    }

    /// Get tab count
    pub async fn tab_count(&self) -> usize {
        self.tabs.read().await.len()
    }

    /// Run the browser event loop
    pub async fn run(&mut self) {
        info!("Starting browser event loop");

        let mut shutdown_rx = self.shutdown.subscribe();

        // Main event loop
        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    info!("Shutdown signal received");
                    break;
                }

                // Compositor frame
                _ = self.compositor.wait_for_frame() => {
                    self.render_frame().await;
                }

                // Performance sampling (every second)
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {
                    self.perf_monitor.sample();
                }
            }
        }

        self.shutdown().await;
    }

    /// Render a frame
    async fn render_frame(&self) {
        // Get active tab
        let active_id = match *self.active_tab.read().await {
            Some(id) => id,
            None => return,
        };

        // Get tab's render frame
        let tabs = self.tabs.read().await;
        if let Some(tab) = tabs.get(&active_id) {
            let tab = tab.lock().await;
            if let Some(frame) = tab.get_render_frame().await {
                self.compositor.submit_frame(frame).await;
            }
        }
    }

    /// Shutdown the browser
    async fn shutdown(&mut self) {
        info!("Shutting down browser...");

        // Close all tabs
        let tab_ids: Vec<TabId> = self.tabs.read().await.keys().copied().collect();
        for tab_id in tab_ids {
            let _ = self.close_tab(tab_id).await;
        }

        // Shutdown process manager
        self.process_manager.shutdown().await;

        // Flush storage
        self.storage.flush().await;

        info!("Browser shutdown complete");
    }

    /// Request shutdown
    pub fn request_shutdown(&self) {
        let _ = self.shutdown.send(());
    }
}
