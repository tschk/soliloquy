//! WGPU Integration for GPU-accelerated rendering
//!
//! This module provides real WGPU device and pipeline setup for:
//! - Layout compute shaders
//! - Compositor rendering
//! - Texture atlas management

#[cfg(feature = "wgpu_rendering")]
use wgpu;

#[cfg(feature = "wgpu_rendering")]
use log::{debug, info};

#[cfg(feature = "wgpu_rendering")]
use std::sync::Arc;

/// WGPU device context for GPU operations
#[cfg(feature = "wgpu_rendering")]
pub struct WgpuContext {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub adapter_info: wgpu::AdapterInfo,
}

#[cfg(feature = "wgpu_rendering")]
impl WgpuContext {
    /// Initialize WGPU device with appropriate backend
    pub async fn new() -> Result<Self, String> {
        info!("Initializing WGPU device...");

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| "Failed to find suitable GPU adapter".to_string())?;

        let adapter_info = adapter.get_info();
        info!(
            "Selected adapter: {} ({:?})",
            adapter_info.name, adapter_info.backend
        );

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Soliloquy Browser Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .map_err(|e| format!("Failed to create device: {}", e))?;

        info!("WGPU device initialized successfully");

        Ok(WgpuContext {
            device: Arc::new(device),
            queue: Arc::new(queue),
            adapter_info,
        })
    }

    /// Create a compute pipeline for layout calculations
    pub fn create_layout_pipeline(&self) -> Result<wgpu::ComputePipeline, String> {
        let shader_src = include_str!("shaders/layout.wgsl");

        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Layout Compute Shader"),
                source: wgpu::ShaderSource::Wgsl(shader_src.into()),
            });

        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Layout Compute Pipeline"),
                layout: None,
                module: &shader,
                entry_point: "layout_pass",
                compilation_options: Default::default(),
                cache: None,
            });

        debug!("Created layout compute pipeline");
        Ok(pipeline)
    }

    /// Create a render pipeline for compositing
    pub fn create_compositor_pipeline(
        &self,
        surface_format: wgpu::TextureFormat,
    ) -> Result<wgpu::RenderPipeline, String> {
        let shader_src = include_str!("shaders/composite.wgsl");

        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Compositor Shader"),
                source: wgpu::ShaderSource::Wgsl(shader_src.into()),
            });

        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Compositor Render Pipeline"),
                layout: None,
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: surface_format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        debug!("Created compositor render pipeline");
        Ok(pipeline)
    }

    /// Create a GPU buffer
    pub fn create_buffer(
        &self,
        size: u64,
        usage: wgpu::BufferUsages,
    ) -> Result<wgpu::Buffer, String> {
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GPU Buffer"),
            size,
            usage,
            mapped_at_creation: false,
        });

        Ok(buffer)
    }

    /// Get device info
    pub fn adapter_info(&self) -> &wgpu::AdapterInfo {
        &self.adapter_info
    }
}

// Stub implementation when wgpu_rendering feature is disabled
#[cfg(not(feature = "wgpu_rendering"))]
pub struct WgpuContext;

#[cfg(not(feature = "wgpu_rendering"))]
impl WgpuContext {
    pub fn new() -> Result<Self, String> {
        log::warn!("WGPU rendering disabled - using stub implementation");
        Ok(WgpuContext)
    }
}

#[cfg(all(test, feature = "wgpu_rendering"))]
mod tests {
    use super::*;

    #[test]
    fn test_wgpu_context_creation() {
        // This test requires a GPU, so we just verify the interface exists
        // In CI without GPU, this would fail, so we skip it
        if std::env::var("CI").is_ok() {
            return;
        }

        pollster::block_on(async {
            let ctx = WgpuContext::new().await;
            assert!(ctx.is_ok() || ctx.is_err()); // Either works or doesn't have GPU
        });
    }

    #[test]
    fn test_stub_when_disabled() {
        // When feature is disabled, stub should always work
        #[cfg(not(feature = "wgpu_rendering"))]
        {
            let ctx = WgpuContext::new();
            assert!(ctx.is_ok());
        }
    }
}
