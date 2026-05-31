//! GPU-accelerated layout computation
//!
//! Offloads box model calculations to compute shaders for parallel processing.

use crate::gpu::wgpu_integration::WgpuContext;
use bytemuck::{Pod, Zeroable};
use log::{debug, info, warn};
use wgpu::util::DeviceExt;

/// Layout node for GPU computation
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
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

/// GPU layout compute pipeline
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

        for i in 0..nodes.len() {
            layout_node(i, nodes, self.viewport_width, self.viewport_height);
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

fn layout_node(
    node_idx: usize,
    nodes: &mut [LayoutNode],
    viewport_width: f32,
    viewport_height: f32,
) {
    let node = nodes[node_idx];
    let (parent_x, parent_y, parent_width, parent_height) =
        parent_box(&node, nodes, viewport_width, viewport_height);
    let containing_x = if is_fixed(node) { 0.0 } else { parent_x };
    let containing_y = if is_fixed(node) { 0.0 } else { parent_y };
    let containing_width = if is_fixed(node) {
        viewport_width
    } else {
        parent_width
    };
    let containing_height = if is_fixed(node) {
        viewport_height
    } else {
        parent_height
    };
    let available_width = available_inline_size(node, containing_width);
    let available_height = available_block_size(node, containing_height);
    let mut laid_out = node;

    laid_out.computed_x = containing_x + node.margin_left + node.padding_left;
    laid_out.computed_y = containing_y + node.margin_top + node.padding_top;

    if is_inline(node) {
        laid_out.computed_width =
            explicit_or(node.computed_width, node.padding_left + node.padding_right);
        laid_out.computed_height =
            explicit_or(node.computed_height, node.padding_top + node.padding_bottom);
    } else if is_positioned(node) || is_block(node) {
        laid_out.computed_width = explicit_or(node.computed_width, available_width);
        laid_out.computed_height = explicit_or(node.computed_height, available_height);
    } else {
        laid_out.computed_width = explicit_or(node.computed_width, available_width);
        laid_out.computed_height =
            explicit_or(node.computed_height, node.padding_top + node.padding_bottom);
    }

    nodes[node_idx] = laid_out;
}

fn parent_box(
    node: &LayoutNode,
    nodes: &[LayoutNode],
    viewport_width: f32,
    viewport_height: f32,
) -> (f32, f32, f32, f32) {
    if node.parent_idx == u32::MAX {
        return (0.0, 0.0, viewport_width, viewport_height);
    }

    nodes
        .get(node.parent_idx as usize)
        .map(|parent| {
            (
                parent.computed_x,
                parent.computed_y,
                parent.computed_width,
                parent.computed_height,
            )
        })
        .unwrap_or((0.0, 0.0, viewport_width, viewport_height))
}

fn available_inline_size(node: LayoutNode, containing_width: f32) -> f32 {
    (containing_width
        - node.margin_left
        - node.margin_right
        - node.padding_left
        - node.padding_right)
        .max(0.0)
}

fn available_block_size(node: LayoutNode, containing_height: f32) -> f32 {
    (containing_height
        - node.margin_top
        - node.margin_bottom
        - node.padding_top
        - node.padding_bottom)
        .max(0.0)
}

fn explicit_or(value: f32, fallback: f32) -> f32 {
    if value > 0.0 {
        value
    } else {
        fallback
    }
}

fn is_block(node: LayoutNode) -> bool {
    (node.style_flags & style_flags::DISPLAY_BLOCK) != 0
}

fn is_inline(node: LayoutNode) -> bool {
    (node.style_flags & style_flags::DISPLAY_INLINE) != 0
}

fn is_positioned(node: LayoutNode) -> bool {
    (node.style_flags & (style_flags::POSITION_ABSOLUTE | style_flags::POSITION_FIXED)) != 0
}

fn is_fixed(node: LayoutNode) -> bool {
    (node.style_flags & style_flags::POSITION_FIXED) != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cpu_compute(width: f32, height: f32) -> GpuLayoutCompute {
        GpuLayoutCompute {
            viewport_width: width,
            viewport_height: height,
            gpu_available: false,
            wgpu_ctx: None,
            pipeline: None,
        }
    }

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
        let compute = cpu_compute(800.0, 600.0);

        let mut nodes = vec![
            LayoutNode {
                parent_idx: u32::MAX,
                child_count: 1,
                style_flags: style_flags::DISPLAY_BLOCK,
                computed_height: 120.0,
                ..Default::default()
            },
            LayoutNode {
                parent_idx: 0,
                child_count: 0,
                style_flags: style_flags::DISPLAY_BLOCK,
                computed_height: 60.0,
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
    fn block_layout_uses_containing_width_and_style_height() {
        let compute = cpu_compute(800.0, 600.0);
        let mut nodes = vec![LayoutNode {
            parent_idx: u32::MAX,
            style_flags: style_flags::DISPLAY_BLOCK,
            computed_height: 48.0,
            margin_left: 10.0,
            margin_right: 30.0,
            padding_left: 5.0,
            padding_right: 7.0,
            ..Default::default()
        }];

        compute.compute_layout(&mut nodes).unwrap();

        assert_eq!(nodes[0].computed_width, 748.0);
        assert_eq!(nodes[0].computed_height, 48.0);
        assert_eq!(nodes[0].computed_x, 15.0);
    }

    #[test]
    fn inline_layout_uses_explicit_size_or_padding_size() {
        let compute = cpu_compute(800.0, 600.0);
        let mut nodes = vec![LayoutNode {
            parent_idx: u32::MAX,
            style_flags: style_flags::DISPLAY_INLINE,
            padding_left: 6.0,
            padding_top: 4.0,
            padding_right: 8.0,
            padding_bottom: 10.0,
            ..Default::default()
        }];

        compute.compute_layout(&mut nodes).unwrap();

        assert_eq!(nodes[0].computed_width, 14.0);
        assert_eq!(nodes[0].computed_height, 14.0);
    }

    #[test]
    fn fixed_layout_uses_viewport_as_containing_box() {
        let compute = cpu_compute(800.0, 600.0);
        let mut nodes = vec![
            LayoutNode {
                parent_idx: u32::MAX,
                style_flags: style_flags::DISPLAY_BLOCK,
                computed_x: 50.0,
                computed_y: 40.0,
                computed_width: 300.0,
                computed_height: 200.0,
                ..Default::default()
            },
            LayoutNode {
                parent_idx: 0,
                style_flags: style_flags::POSITION_FIXED,
                margin_left: 20.0,
                margin_top: 10.0,
                margin_right: 60.0,
                margin_bottom: 90.0,
                padding_left: 5.0,
                padding_top: 7.0,
                padding_right: 15.0,
                padding_bottom: 3.0,
                ..Default::default()
            },
        ];

        compute.compute_layout(&mut nodes).unwrap();

        assert_eq!(nodes[1].computed_x, 25.0);
        assert_eq!(nodes[1].computed_y, 17.0);
        assert_eq!(nodes[1].computed_width, 700.0);
        assert_eq!(nodes[1].computed_height, 490.0);
    }

    #[test]
    fn absolute_layout_uses_parent_as_containing_box() {
        let compute = cpu_compute(800.0, 600.0);
        let mut nodes = vec![
            LayoutNode {
                parent_idx: u32::MAX,
                style_flags: style_flags::DISPLAY_BLOCK,
                computed_width: 300.0,
                computed_height: 200.0,
                margin_left: 50.0,
                margin_top: 40.0,
                ..Default::default()
            },
            LayoutNode {
                parent_idx: 0,
                style_flags: style_flags::POSITION_ABSOLUTE,
                computed_width: 90.0,
                margin_left: 20.0,
                margin_top: 10.0,
                padding_left: 5.0,
                padding_top: 7.0,
                padding_bottom: 3.0,
                ..Default::default()
            },
        ];

        compute.compute_layout(&mut nodes).unwrap();

        assert_eq!(nodes[1].computed_x, 75.0);
        assert_eq!(nodes[1].computed_y, 57.0);
        assert_eq!(nodes[1].computed_width, 90.0);
        assert_eq!(nodes[1].computed_height, 180.0);
    }

    #[test]
    fn layout_node_matches_cpu_pass_for_representative_tree() {
        let compute = cpu_compute(1024.0, 768.0);
        let mut cpu_nodes = representative_nodes();
        let mut direct_nodes = representative_nodes();

        compute.compute_layout(&mut cpu_nodes).unwrap();
        for i in 0..direct_nodes.len() {
            layout_node(i, &mut direct_nodes, 1024.0, 768.0);
        }

        assert_eq!(cpu_nodes, direct_nodes);
    }

    #[test]
    fn gpu_layout_matches_cpu_layout_when_gpu_is_available() {
        let gpu_compute = GpuLayoutCompute::new(640.0, 480.0);
        if !gpu_compute.gpu_available {
            return;
        }

        let cpu_compute = cpu_compute(640.0, 480.0);
        let mut gpu_nodes = gpu_parity_nodes();
        let mut cpu_nodes = gpu_parity_nodes();

        gpu_compute.compute_layout(&mut gpu_nodes).unwrap();
        cpu_compute.compute_layout(&mut cpu_nodes).unwrap();

        assert_eq!(gpu_nodes, cpu_nodes);
    }

    fn representative_nodes() -> Vec<LayoutNode> {
        vec![
            LayoutNode {
                parent_idx: u32::MAX,
                style_flags: style_flags::DISPLAY_BLOCK,
                computed_height: 400.0,
                padding_left: 8.0,
                padding_top: 6.0,
                padding_right: 8.0,
                padding_bottom: 6.0,
                ..Default::default()
            },
            LayoutNode {
                parent_idx: 0,
                style_flags: style_flags::DISPLAY_INLINE,
                padding_left: 2.0,
                padding_top: 3.0,
                padding_right: 4.0,
                padding_bottom: 5.0,
                ..Default::default()
            },
            LayoutNode {
                parent_idx: 0,
                style_flags: style_flags::POSITION_ABSOLUTE,
                computed_width: 128.0,
                computed_height: 64.0,
                margin_left: 20.0,
                margin_top: 30.0,
                ..Default::default()
            },
            LayoutNode {
                parent_idx: 0,
                style_flags: style_flags::POSITION_FIXED,
                margin_left: 12.0,
                margin_top: 14.0,
                margin_right: 16.0,
                margin_bottom: 18.0,
                ..Default::default()
            },
        ]
    }

    fn gpu_parity_nodes() -> Vec<LayoutNode> {
        vec![
            LayoutNode {
                parent_idx: u32::MAX,
                style_flags: style_flags::DISPLAY_BLOCK,
                computed_width: 640.0,
                computed_height: 480.0,
                ..Default::default()
            },
            LayoutNode {
                parent_idx: 0,
                style_flags: style_flags::DISPLAY_BLOCK,
                computed_height: 40.0,
                margin_left: 10.0,
                margin_right: 20.0,
                padding_left: 4.0,
                padding_right: 6.0,
                ..Default::default()
            },
            LayoutNode {
                parent_idx: 0,
                style_flags: style_flags::DISPLAY_INLINE,
                padding_left: 3.0,
                padding_top: 5.0,
                padding_right: 7.0,
                padding_bottom: 11.0,
                ..Default::default()
            },
            LayoutNode {
                parent_idx: 0,
                style_flags: style_flags::POSITION_ABSOLUTE,
                computed_width: 80.0,
                computed_height: 30.0,
                margin_left: 12.0,
                margin_top: 14.0,
                padding_left: 2.0,
                padding_top: 4.0,
                ..Default::default()
            },
            LayoutNode {
                parent_idx: 0,
                style_flags: style_flags::POSITION_FIXED,
                margin_left: 8.0,
                margin_top: 9.0,
                margin_right: 10.0,
                margin_bottom: 11.0,
                padding_left: 1.0,
                padding_top: 2.0,
                padding_right: 3.0,
                padding_bottom: 4.0,
                ..Default::default()
            },
        ]
    }

    #[test]
    fn test_viewport_resize() {
        let mut compute = GpuLayoutCompute::new(1920.0, 1080.0);
        compute.set_viewport_size(1280.0, 720.0);

        assert_eq!(compute.viewport_width, 1280.0);
        assert_eq!(compute.viewport_height, 720.0);
    }
}
