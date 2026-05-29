//! GPU-accelerated layout computation
//!
//! Offloads box model calculations to compute shaders for parallel processing.

use crate::gpu::wgpu_integration::WgpuContext;
use bytemuck::{Pod, Zeroable};
use log::{debug, info, warn};
use wgpu::util::DeviceExt;

/// Layout node for GPU computation
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct LayoutNode {
    pub parent_idx: u32,
    pub child_count: u32,
    pub style_flags: u32,
    _padding: u32,

    pub computed_x: f32,
    pub computed_y: f32,
    pub computed_width: f32,
    pub computed_height: f32,

    pub margin_left: f32,
    pub margin_top: f32,
    pub margin_right: f32,
    pub margin_bottom: f32,

    pub padding_left: f32,
    pub padding_top: f32,
    pub padding_right: f32,
    pub padding_bottom: f32,
}

impl Default for LayoutNode {
    fn default() -> Self {
        LayoutNode {
            parent_idx: u32::MAX, // No parent
            child_count: 0,
            style_flags: 0,
            _padding: 0,
            computed_x: 0.0,
            computed_y: 0.0,
            computed_width: 0.0,
            computed_height: 0.0,
            margin_left: 0.0,
            margin_top: 0.0,
            margin_right: 0.0,
            margin_bottom: 0.0,
            padding_left: 0.0,
            padding_top: 0.0,
            padding_right: 0.0,
            padding_bottom: 0.0,
        }
    }
}

/// Style flags
pub mod style_flags {
    pub const DISPLAY_BLOCK: u32 = 0x01;
    pub const DISPLAY_INLINE: u32 = 0x02;
    pub const DISPLAY_FLEX: u32 = 0x04;
    pub const POSITION_ABSOLUTE: u32 = 0x08;
    pub const POSITION_FIXED: u32 = 0x10;
}

/// Layout computation parameters
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct LayoutParams {
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub node_count: u32,
    pub pass_index: u32,
}

/// GPU layout compute pipeline (placeholder)
///
/// In production, this would:
/// 1. Create WGPU compute pipeline from layout.wgsl
/// 2. Upload layout nodes to GPU buffer
/// 3. Execute compute shader
/// 4. Read back computed layout
pub struct GpuLayoutCompute {
    /// Viewport dimensions
    viewport_width: f32,
    viewport_height: f32,
    /// Whether GPU compute is available
    gpu_available: bool,

    // WGPU context
    wgpu_ctx: Option<WgpuContext>,
    pipeline: Option<wgpu::ComputePipeline>,
}

impl GpuLayoutCompute {
    /// Create a new GPU layout compute instance
    pub fn new(viewport_width: f32, viewport_height: f32) -> Self {
        info!(
            "Initializing GPU layout compute ({}x{})",
            viewport_width, viewport_height
        );

        // Try to initialize WGPU
        let wgpu_ctx = pollster::block_on(WgpuContext::new()).ok();
        let mut pipeline = None;
        let mut gpu_available = false;

        if let Some(ref ctx) = wgpu_ctx {
            if let Ok(p) = ctx.create_layout_pipeline() {
                pipeline = Some(p);
                gpu_available = true;
                debug!("WGPU compute pipeline initialized");
            } else {
                warn!("Failed to create layout pipeline");
            }
        } else {
            warn!("Failed to initialize WGPU context");
        }

        GpuLayoutCompute {
            viewport_width,
            viewport_height,
            gpu_available,
            wgpu_ctx,
            pipeline,
        }
    }

    /// Compute layout for a tree of nodes
    pub fn compute_layout(&self, nodes: &mut [LayoutNode]) -> Result<(), String> {
        if !self.gpu_available {
            // Fallback to CPU layout
            return self.cpu_layout_fallback(nodes);
        }

        let ctx = self.wgpu_ctx.as_ref().ok_or("WGPU context missing")?;
        let pipeline = self.pipeline.as_ref().ok_or("Pipeline missing")?;

        debug!("Computing layout for {} nodes on GPU", nodes.len());

        let node_count = nodes.len() as u32;
        let params = LayoutParams {
            viewport_width: self.viewport_width,
            viewport_height: self.viewport_height,
            node_count,
            pass_index: 0,
        };

        // Create buffers
        let nodes_buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Nodes Buffer"),
                contents: bytemuck::cast_slice(nodes),
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST,
            });

        let params_buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Params Buffer"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create bind group
        // We rely on implicit bind group layout from pipeline
        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Layout Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: nodes_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Encode commands
        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Layout Compute Encoder"),
            });

        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Layout Compute Pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
// Dispatch: workgroup size is 64
         let workgroup_count = node_count.div_ceil(64);
            cpass.dispatch_workgroups(workgroup_count, 1, 1);
        }

// Read back
         let buffer_size = std::mem::size_of_val(nodes) as u64;
        let staging_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        encoder.copy_buffer_to_buffer(&nodes_buffer, 0, &staging_buffer, 0, buffer_size);

        ctx.queue.submit(Some(encoder.finish()));

        // Map and read
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = futures::channel::oneshot::channel();
        buffer_slice.map_async(
            wgpu::MapMode::Read,
            move |v: Result<(), wgpu::BufferAsyncError>| {
                sender.send(v).ok();
            },
        );

        ctx.device.poll(wgpu::Maintain::Wait);

        if let Ok(Ok(())) = pollster::block_on(receiver) {
            let data = buffer_slice.get_mapped_range();
            let result: &[LayoutNode] = bytemuck::cast_slice(&data);
            nodes.copy_from_slice(result);
            drop(data);
            staging_buffer.unmap();
            Ok(())
        } else {
            Err("Failed to read back from GPU".to_string())
        }
    }

    /// CPU fallback layout computation
    fn cpu_layout_fallback(&self, nodes: &mut [LayoutNode]) -> Result<(), String> {
        debug!(
            "Computing layout for {} nodes on CPU (fallback)",
            nodes.len()
        );

        // Simple top-down layout pass
        for i in 0..nodes.len() {
            // Get parent info before borrowing current node
            let (parent_x, parent_y, parent_width) = {
                let node = &nodes[i];
                if node.parent_idx != u32::MAX {
                    let parent_idx = node.parent_idx as usize;
                    if parent_idx < i {
                        let parent = &nodes[parent_idx];
                        (parent.computed_x, parent.computed_y, parent.computed_width)
                    } else {
                        (0.0, 0.0, self.viewport_width)
                    }
                } else {
                    (0.0, 0.0, self.viewport_width)
                }
            };

            // Now mutably borrow current node
            let node = &mut nodes[i];

            // Block layout
            if (node.style_flags & style_flags::DISPLAY_BLOCK) != 0 {
                node.computed_x = parent_x + node.margin_left + node.padding_left;
                node.computed_y = parent_y + node.margin_top + node.padding_top;
                node.computed_width = parent_width
                    - node.margin_left
                    - node.margin_right
                    - node.padding_left
                    - node.padding_right;

                if node.computed_height == 0.0 {
                    node.computed_height = 100.0; // Default
                }
            }
        }

        Ok(())
    }

    /// Update viewport size
    pub fn set_viewport_size(&mut self, width: f32, height: f32) {
        self.viewport_width = width;
        self.viewport_height = height;
        info!("Updated viewport size to {}x{}", width, height);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_node_default() {
        let node = LayoutNode::default();
        assert_eq!(node.parent_idx, u32::MAX);
        assert_eq!(node.child_count, 0);
        assert_eq!(node.computed_width, 0.0);
    }

    #[test]
    fn test_gpu_layout_compute_creation() {
        let compute = GpuLayoutCompute::new(1920.0, 1080.0);
        assert_eq!(compute.viewport_width, 1920.0);
        assert_eq!(compute.viewport_height, 1080.0);
    }

    #[test]
    fn test_simple_layout() {
        let compute = GpuLayoutCompute::new(800.0, 600.0);

        let mut nodes = vec![
            LayoutNode {
                parent_idx: u32::MAX,
                child_count: 1,
                style_flags: style_flags::DISPLAY_BLOCK,
                ..Default::default()
            },
            LayoutNode {
                parent_idx: 0,
                child_count: 0,
                style_flags: style_flags::DISPLAY_BLOCK,
                ..Default::default()
            },
        ];

        let result = compute.compute_layout(&mut nodes);
        assert!(result.is_ok());

        // Root should be at 0,0 with full width
        assert_eq!(nodes[0].computed_x, 0.0);
        assert_eq!(nodes[0].computed_y, 0.0);
    }

    #[test]
    fn test_viewport_resize() {
        let mut compute = GpuLayoutCompute::new(1920.0, 1080.0);
        compute.set_viewport_size(1280.0, 720.0);

        assert_eq!(compute.viewport_width, 1280.0);
        assert_eq!(compute.viewport_height, 720.0);
    }
}
