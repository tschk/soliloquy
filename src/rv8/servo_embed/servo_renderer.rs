//! In-process Servo WebView + software GL readback for host embedders.

use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use dpi::PhysicalSize;
use servo::{
    DeviceIntPoint, DeviceIntRect, DeviceIntSize, EventLoopWaker, LoadStatus, RenderingContext,
    Servo, ServoBuilder, SoftwareRenderingContext, WebView, WebViewBuilder, WebViewDelegate,
};
use url::Url;

use crate::renderer::RenderFrame;

struct EmbedderDelegate {
    frame_ready: Arc<AtomicBool>,
    load_complete: Arc<AtomicBool>,
}

impl WebViewDelegate for EmbedderDelegate {
    fn notify_new_frame_ready(&self, webview: WebView) {
        self.frame_ready.store(true, Ordering::Relaxed);
        webview.paint();
    }

    fn notify_load_status_changed(&self, _webview: WebView, status: LoadStatus) {
        if status == LoadStatus::Complete {
            self.load_complete.store(true, Ordering::Relaxed);
        }
    }
}

struct EventLoopWakerImpl(Arc<AtomicBool>);

impl EventLoopWaker for EventLoopWakerImpl {
    fn clone_box(&self) -> Box<dyn EventLoopWaker> {
        Box::new(EventLoopWakerImpl(self.0.clone()))
    }

    fn wake(&self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

pub struct ServoRenderer {
    servo: Servo,
    rendering_context: Rc<dyn RenderingContext>,
    webview: WebView,
    width: u32,
    height: u32,
    frame_ready: Arc<AtomicBool>,
    load_complete: Arc<AtomicBool>,
}

impl ServoRenderer {
    pub fn new(width: u32, height: u32) -> Result<Self, String> {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

        let width = width.max(1);
        let height = height.max(1);
        let physical = PhysicalSize::new(width, height);
        let rendering_context = Rc::new(
            SoftwareRenderingContext::new(physical)
                .map_err(|e| format!("SoftwareRenderingContext: {e:?}"))?,
        );
        rendering_context
            .make_current()
            .map_err(|e| format!("make_current: {e:?}"))?;

        let woken = Arc::new(AtomicBool::new(false));
        let mut preferences = servo::Preferences::default();
        preferences.network_http_proxy_uri.clear();
        preferences.network_https_proxy_uri.clear();

        let servo = ServoBuilder::default()
            .preferences(preferences)
            .event_loop_waker(Box::new(EventLoopWakerImpl(woken)))
            .build();

        let frame_ready = Arc::new(AtomicBool::new(false));
        let load_complete = Arc::new(AtomicBool::new(false));
        let delegate = Rc::new(EmbedderDelegate {
            frame_ready: frame_ready.clone(),
            load_complete: load_complete.clone(),
        });

        let webview = WebViewBuilder::new(&servo, rendering_context.clone())
            .delegate(delegate)
            .build();

        Ok(ServoRenderer {
            servo,
            rendering_context,
            webview,
            width,
            height,
            frame_ready,
            load_complete,
        })
    }

    pub fn navigate(&mut self, url: &str) -> Result<(), String> {
        let parsed = Url::parse(url).map_err(|e| format!("invalid URL: {e}"))?;
        self.load_complete.store(false, Ordering::Relaxed);
        self.frame_ready.store(false, Ordering::Relaxed);
        self.webview.load(parsed);
        self.pump_until(
            || self.load_complete.load(Ordering::Relaxed),
            Duration::from_secs(120),
        )?;
        self.pump_until(
            || self.frame_ready.load(Ordering::Relaxed),
            Duration::from_secs(30),
        )?;
        Ok(())
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width.max(1);
        self.height = height.max(1);
        self.webview.resize(PhysicalSize::new(self.width, self.height));
        self.frame_ready.store(false, Ordering::Relaxed);
        self.pump_for(Duration::from_millis(50));
    }

    pub fn title(&self) -> String {
        self.webview
            .page_title()
            .unwrap_or_else(|| String::new())
    }

    pub fn capture_frame(&mut self, generation: u64) -> Option<RenderFrame> {
        self.pump_for(Duration::from_millis(16));
        if !self.frame_ready.load(Ordering::Relaxed) {
            self.webview.paint();
        }
        let rect = DeviceIntRect::from_origin_and_size(
            DeviceIntPoint::zero(),
            DeviceIntSize::new(self.width as i32, self.height as i32),
        );
        let image = self.rendering_context.read_to_image(rect)?;
        let mut frame = RenderFrame::new(self.width, self.height);
        frame.id = generation;
        let img_w = image.width();
        let img_h = image.height();
        let rgba = image.into_raw();
        if img_w == self.width && img_h == self.height && rgba.len() == frame.pixels.len() {
            frame.pixels = rgba;
        } else {
            let copy_len = frame.pixels.len().min(rgba.len());
            frame.pixels[..copy_len].copy_from_slice(&rgba[..copy_len]);
        }
        Some(frame)
    }

    fn pump_until(&self, done: impl Fn() -> bool, timeout: Duration) -> Result<(), String> {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if done() {
                return Ok(());
            }
            self.servo.spin_event_loop();
            thread::sleep(Duration::from_millis(1));
        }
        Err("Servo navigation timed out".to_string())
    }

    fn pump_for(&self, duration: Duration) {
        let deadline = Instant::now() + duration;
        while Instant::now() < deadline {
            self.servo.spin_event_loop();
            thread::sleep(Duration::from_millis(1));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_com_renders_with_layout() {
        let mut renderer = ServoRenderer::new(800, 600).expect("servo renderer");
        renderer
            .navigate("https://example.com")
            .expect("navigate example.com");
        let frame = renderer.capture_frame(1).expect("frame");
        assert_eq!(frame.width, 800);
        assert_eq!(frame.height, 600);
        let non_blank = frame
            .pixels
            .chunks_exact(4)
            .any(|px| px[0] > 32 || px[1] > 32 || px[2] > 32);
        assert!(non_blank, "expected non-blank Servo pixels for example.com");
        if !renderer.title().is_empty() {
            assert!(
                renderer.title().to_ascii_lowercase().contains("example"),
                "unexpected title: {}",
                renderer.title()
            );
        }
    }
}
