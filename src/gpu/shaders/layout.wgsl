// GPU-accelerated layout computation shader
// 
// Computes box model layout in parallel on the GPU.
// Each workgroup processes a batch of layout nodes.

struct LayoutNode {
    parent_idx: u32,
    child_count: u32,
    style_flags: u32,
    padding: u32,  // Alignment padding
    
    computed_x: f32,
    computed_y: f32,
    computed_width: f32,
    computed_height: f32,
    
    margin_left: f32,
    margin_top: f32,
    margin_right: f32,
    margin_bottom: f32,
    
    padding_left: f32,
    padding_top: f32,
    padding_right: f32,
    padding_bottom: f32,
}

// Style flags for layout behavior
const STYLE_DISPLAY_BLOCK: u32 = 0x01u;
const STYLE_DISPLAY_INLINE: u32 = 0x02u;
const STYLE_DISPLAY_FLEX: u32 = 0x04u;
const STYLE_POSITION_ABSOLUTE: u32 = 0x08u;
const STYLE_POSITION_FIXED: u32 = 0x10u;

// Layout computation parameters
struct LayoutParams {
    viewport_width: f32,
    viewport_height: f32,
    node_count: u32,
    pass_index: u32,  // Multi-pass layout for dependency resolution
}

@group(0) @binding(0) var<storage, read_write> nodes: array<LayoutNode>;
@group(0) @binding(1) var<uniform> params: LayoutParams;

@compute @workgroup_size(64)
fn layout_pass(@builtin(global_invocation_id) id: vec3<u32>) {
    let node_idx = id.x;
    
    // Bounds check
    if (node_idx >= params.node_count) {
        return;
    }
    
    var node = nodes[node_idx];
    
    // Skip fixed position elements on first pass
    if (params.pass_index == 0u && (node.style_flags & STYLE_POSITION_FIXED) != 0u) {
        return;
    }
    
    // Get parent position if not root
    var parent_x = 0.0;
    var parent_y = 0.0;
    var parent_width = params.viewport_width;
    
    if (node.parent_idx != 0xFFFFFFFFu) {
        let parent = nodes[node.parent_idx];
        parent_x = parent.computed_x;
        parent_y = parent.computed_y;
        parent_width = parent.computed_width;
    }
    
    // Compute position based on display type
    if ((node.style_flags & STYLE_DISPLAY_BLOCK) != 0u) {
        // Block layout: full width, stack vertically
        node.computed_x = parent_x + node.margin_left + node.padding_left;
        node.computed_y = parent_y + node.margin_top + node.padding_top;
        node.computed_width = parent_width - node.margin_left - node.margin_right 
                             - node.padding_left - node.padding_right;
        
        // Height computed based on content (placeholder)
        if (node.computed_height == 0.0) {
            node.computed_height = 100.0; // Default height
        }
    }
    else if ((node.style_flags & STYLE_DISPLAY_INLINE) != 0u) {
        // Inline layout: flow horizontally
        node.computed_x = parent_x + node.margin_left;
        node.computed_y = parent_y + node.margin_top;
        
        // Width/height based on content (placeholder)
        if (node.computed_width == 0.0) {
            node.computed_width = 50.0;
        }
        if (node.computed_height == 0.0) {
            node.computed_height = 20.0;
        }
    }
    else if ((node.style_flags & STYLE_POSITION_ABSOLUTE) != 0u) {
        // Absolute positioning: positioned relative to parent
        // Position already set by style resolution
    }
    
    // Apply fixed positioning
    if ((node.style_flags & STYLE_POSITION_FIXED) != 0u) {
        // Fixed to viewport, ignore parent
        node.computed_x = node.margin_left;
        node.computed_y = node.margin_top;
    }
    
    // Write back computed values
    nodes[node_idx] = node;
}

// Compute content height by summing children
@compute @workgroup_size(64)
fn compute_heights(@builtin(global_invocation_id) id: vec3<u32>) {
    let node_idx = id.x;
    
    if (node_idx >= params.node_count) {
        return;
    }
    
    var node = nodes[node_idx];
    
    // If no children, keep current height
    if (node.child_count == 0u) {
        return;
    }
    
    // Sum child heights (simplified - assumes children are stacked)
    var total_height = 0.0;
    var child_idx = node_idx + 1u; // Simplified child lookup
    
    for (var i = 0u; i < node.child_count; i = i + 1u) {
        if (child_idx < params.node_count) {
            let child = nodes[child_idx];
            total_height = total_height + child.computed_height 
                         + child.margin_top + child.margin_bottom;
            child_idx = child_idx + 1u;
        }
    }
    
    // Update parent height
    node.computed_height = total_height + node.padding_top + node.padding_bottom;
    nodes[node_idx] = node;
}
