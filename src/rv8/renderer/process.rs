//! Renderer Process
//!
//! This module implements the renderer process which is responsible for
//! parsing HTML/CSS, executing JavaScript (via V8), and generating render frames.

use log::{debug, error, info};
use tokio::sync::mpsc;

use crate::ipc::{BrowserMessage, IpcSender, RendererChannel, RendererMessage};
use crate::servo_embed::{ServoConfig, ServoEmbedder};

/// Renderer process state
pub struct RendererProcess {
    /// Tab ID this renderer is for
    tab_id: u64,
    /// Channel to browser process
    browser_channel: RendererChannel,
    /// Servo embedder for rendering
    embedder: ServoEmbedder,
    /// Current URL
    current_url: String,
    /// Is shutting down
    shutting_down: bool,
}

impl RendererProcess {
    /// Create a new renderer process
    pub async fn new(
        tab_id: u64,
        browser_tx: IpcSender<BrowserMessage>,
        config: ServoConfig,
    ) -> Result<Self, String> {
        info!("Creating renderer process for tab {}", tab_id);

        let embedder = ServoEmbedder::new(config).await?;
        let browser_channel = RendererChannel::new(tab_id, browser_tx);

        Ok(RendererProcess {
            tab_id,
            browser_channel,
            embedder,
            current_url: String::new(),
            shutting_down: false,
        })
    }

    /// Run the renderer event loop
    pub async fn run(&mut self, mut rx: mpsc::UnboundedReceiver<RendererMessage>) {
        info!("Renderer {} starting event loop", self.tab_id);

        while !self.shutting_down {
            tokio::select! {
                Some(msg) = rx.recv() => {
                    self.handle_message(msg).await;
                }
                else => break,
            }
        }

        info!("Renderer {} shutting down", self.tab_id);
    }

    /// Handle incoming message
    async fn handle_message(&mut self, msg: RendererMessage) {
        match msg {
            RendererMessage::Navigate { url } => {
                self.navigate(&url).await;
            }
            RendererMessage::Reload => {
                self.navigate(&self.current_url.clone()).await;
            }
            RendererMessage::Stop => {
                // TODO: Stop loading
                debug!("Stop loading requested");
            }
            RendererMessage::GoBack => {
                // TODO: History navigation
                debug!("Go back requested");
            }
            RendererMessage::GoForward => {
                // TODO: History navigation
                debug!("Go forward requested");
            }
            RendererMessage::ExecuteScript {
                script,
                callback_id,
            } => {
                self.execute_script(&script, callback_id).await;
            }
            RendererMessage::Resize { width, height } => {
                self.embedder.resize(width, height);
            }
            RendererMessage::MouseEvent {
                event_type,
                x,
                y,
                button: _,
            } => {
                use crate::ipc::MouseEventType;
                match event_type {
                    MouseEventType::Move => self.embedder.handle_mouse_move(x, y),
                    MouseEventType::Click => {
                        self.embedder.handle_mouse_click(
                            x,
                            y,
                            crate::servo_embed::MouseButton::Left,
                        );
                    }
                    _ => {}
                }
            }
            RendererMessage::KeyEvent {
                event_type: _,
                key,
                modifiers: _,
            } => {
                self.embedder.handle_key(&key, true);
            }
            RendererMessage::Scroll { delta_x, delta_y } => {
                self.embedder.handle_scroll(delta_x, delta_y);
            }
            RendererMessage::Focus { focused: _ } => {
                // TODO: Handle focus
            }
            RendererMessage::Visibility { visible: _ } => {
                // TODO: Handle visibility
            }
            RendererMessage::JsDialogResponse {
                accepted: _,
                response: _,
            } => {
                // TODO: Handle dialog response
            }
            RendererMessage::Shutdown => {
                self.shutting_down = true;
            }
        }
    }

    /// Navigate to URL
    async fn navigate(&mut self, url: &str) {
        info!("Renderer {} navigating to: {}", self.tab_id, url);
        self.current_url = url.to_string();

        // Send progress updates
        let _ = self
            .browser_channel
            .to_browser
            .send(BrowserMessage::LoadProgress {
                tab_id: self.tab_id,
                progress: 10,
            });

        // Navigate via embedder
        if let Err(e) = self.embedder.navigate(url).await {
            error!("Navigation failed: {}", e);
            return;
        }

        // Notify browser of completion
        let _ = self.browser_channel.send_load_complete();
    }

    /// Execute JavaScript
    async fn execute_script(&self, script: &str, _callback_id: u64) {
        match self.embedder.execute_script(script).await {
            Ok(result) => {
                debug!("Script result: {}", result);
                // TODO: Send result back via callback
            }
            Err(e) => {
                error!("Script execution failed: {}", e);
            }
        }
    }

    /// Get current URL
    pub fn current_url(&self) -> &str {
        &self.current_url
    }

    /// Get tab ID
    pub fn tab_id(&self) -> u64 {
        self.tab_id
    }
}
