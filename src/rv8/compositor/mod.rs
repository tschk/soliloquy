//! GPU compositor module

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::core::BrowserConfig;
use crate::renderer::RenderFrame;
use log::{debug, info};
use tokio::sync::{Mutex, Notify};

const FRAME_INTERVAL: Duration = Duration::from_millis(16);
const DAMAGE_HISTORY_LEN: usize = 3;
const PRESENT_QUEUE_LEN: usize = 3;

/// Compositor-space damage region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl DamageRegion {
    pub fn full_frame(width: u32, height: u32) -> Self {
        Self {
            x: 0,
            y: 0,
            width,
            height,
        }
    }
}

/// Snapshot of compositor state for diagnostics and tests.
#[derive(Debug, Clone, PartialEq)]
pub struct CompositorStats {
    pub pending_frame: bool,
    pub submitted_frames: u64,
    pub last_frame_id: Option<u64>,
    pub damage_history_len: usize,
    pub present_queue_len: usize,
    pub last_present_ms: Option<u128>,
}

/// Queued render pass ready for platform presentation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderPass {
    pub frame_id: u64,
    pub damage: DamageRegion,
}

#[derive(Debug)]
struct CompositorState {
    pending_frame: bool,
    submitted_frames: u64,
    last_frame_id: Option<u64>,
    last_present: Option<Instant>,
    damage_history: VecDeque<DamageRegion>,
    present_queue: VecDeque<RenderPass>,
}

impl CompositorState {
    fn new() -> Self {
        Self {
            pending_frame: false,
            submitted_frames: 0,
            last_frame_id: None,
            last_present: None,
            damage_history: VecDeque::with_capacity(DAMAGE_HISTORY_LEN),
            present_queue: VecDeque::with_capacity(PRESENT_QUEUE_LEN),
        }
    }

    fn push_damage(&mut self, damage: DamageRegion) {
        if self.damage_history.len() == DAMAGE_HISTORY_LEN {
            self.damage_history.pop_front();
        }
        self.damage_history.push_back(damage);
        self.pending_frame = true;
    }

    fn stats(&self) -> CompositorStats {
        CompositorStats {
            pending_frame: self.pending_frame,
            submitted_frames: self.submitted_frames,
            last_frame_id: self.last_frame_id,
            damage_history_len: self.damage_history.len(),
            present_queue_len: self.present_queue.len(),
            last_present_ms: self
                .last_present
                .map(|present| present.elapsed().as_millis()),
        }
    }

    fn push_present_pass(&mut self, render_pass: RenderPass) {
        if self.present_queue.len() == PRESENT_QUEUE_LEN {
            self.present_queue.pop_front();
        }
        self.present_queue.push_back(render_pass);
    }
}

/// GPU compositor for layer compositing
pub struct Compositor {
    state: Mutex<CompositorState>,
    frame_notify: Notify,
}

impl Compositor {
    pub async fn new(_config: &BrowserConfig) -> Result<Self, String> {
        info!("Initializing GPU compositor");
        Ok(Compositor {
            state: Mutex::new(CompositorState::new()),
            frame_notify: Notify::new(),
        })
    }

    /// Request a frame because visible content changed.
    pub async fn request_frame(&self) {
        {
            let mut state = self.state.lock().await;
            state.pending_frame = true;
        }
        self.frame_notify.notify_one();
    }

    /// Add damage and wake the compositor.
    pub async fn damage(&self, damage: DamageRegion) {
        {
            let mut state = self.state.lock().await;
            state.push_damage(damage);
        }
        self.frame_notify.notify_one();
    }

    /// Wait until a frame is requested, then pace to the target refresh interval.
    pub async fn wait_for_frame(&self) {
        loop {
            let should_render = {
                let mut state = self.state.lock().await;
                let should_render = state.pending_frame;
                if should_render {
                    state.pending_frame = false;
                }
                should_render
            };

            if should_render {
                tokio::time::sleep(FRAME_INTERVAL).await;
                return;
            }

            self.frame_notify.notified().await;
        }
    }

    pub async fn submit_frame(&self, frame: RenderFrame) {
        let mut state = self.state.lock().await;
        let damage = DamageRegion::full_frame(frame.width, frame.height);
        state.submitted_frames += 1;
        state.last_frame_id = Some(frame.id);
        state.last_present = Some(Instant::now());
        state.push_damage(damage);
        state.push_present_pass(RenderPass {
            frame_id: frame.id,
            damage,
        });
        state.pending_frame = false;
        debug!(
            "Submitted frame {} ({}x{})",
            frame.id, frame.width, frame.height
        );
    }

    pub async fn drain_present_queue(&self) -> Vec<RenderPass> {
        let mut state = self.state.lock().await;
        state.present_queue.drain(..).collect()
    }

    pub async fn stats(&self) -> CompositorStats {
        self.state.lock().await.stats()
    }
}

/// GPU process (runs in child process)
pub struct GpuProcess {
    channel_id: String,
}

impl GpuProcess {
    pub async fn new(channel_id: &str) -> Self {
        info!("GPU process initializing with channel: {}", channel_id);
        GpuProcess {
            channel_id: channel_id.to_string(),
        }
    }

    pub async fn run(&self) {
        info!("GPU process running on channel {}", self.channel_id);
        loop {
            tokio::time::sleep(FRAME_INTERVAL).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::BrowserConfig;

    #[tokio::test]
    async fn damage_should_request_one_frame() {
        let config = BrowserConfig::default();
        let compositor = Compositor::new(&config)
            .await
            .expect("compositor should initialize");

        compositor.damage(DamageRegion::full_frame(800, 600)).await;
        tokio::time::timeout(Duration::from_millis(100), compositor.wait_for_frame())
            .await
            .expect("damage should wake compositor");

        let stats = compositor.stats().await;
        assert!(!stats.pending_frame);
        assert_eq!(stats.damage_history_len, 1);
    }

    #[tokio::test]
    async fn submit_frame_should_record_present_state() {
        let config = BrowserConfig::default();
        let compositor = Compositor::new(&config)
            .await
            .expect("compositor should initialize");
        let mut frame = RenderFrame::new(320, 240);
        frame.id = 42;

        compositor.submit_frame(frame).await;

        let stats = compositor.stats().await;
        assert_eq!(stats.submitted_frames, 1);
        assert_eq!(stats.last_frame_id, Some(42));
        assert_eq!(stats.damage_history_len, 1);
        assert_eq!(stats.present_queue_len, 1);
    }

    #[tokio::test]
    async fn present_queue_should_be_bounded() {
        let config = BrowserConfig::default();
        let compositor = Compositor::new(&config)
            .await
            .expect("compositor should initialize");

        for frame_id in 0..5 {
            let mut frame = RenderFrame::new(10, 10);
            frame.id = frame_id;
            compositor.submit_frame(frame).await;
        }

        let stats = compositor.stats().await;
        assert_eq!(stats.present_queue_len, PRESENT_QUEUE_LEN);

        let passes = compositor.drain_present_queue().await;
        assert_eq!(passes.len(), PRESENT_QUEUE_LEN);
        assert_eq!(passes.first().map(|pass| pass.frame_id), Some(2));
    }
}
