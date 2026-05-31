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

fn explicit_or(value: f32, fallback: f32) -> f32 {
    if (value > 0.0) {
        return value;
    }
    return fallback;
}

fn available_inline_size(node: LayoutNode, containing_width: f32) -> f32 {
    return max(
        containing_width
            - node.margin_left
            - node.margin_right
            - node.padding_left
            - node.padding_right,
        0.0
    );
}

fn available_block_size(node: LayoutNode, containing_height: f32) -> f32 {
    return max(
        containing_height
            - node.margin_top
            - node.margin_bottom
            - node.padding_top
            - node.padding_bottom,
        0.0
    );
}

@compute @workgroup_size(64)
fn layout_pass(@builtin(global_invocation_id) id: vec3<u32>) {
    let node_idx = id.x;
    
    // Bounds check
    if (node_idx >= params.node_count) {
        return;
    }
    
    var node = nodes[node_idx];
    
    // Get parent position if not root
    var parent_x = 0.0;
    var parent_y = 0.0;
    var parent_width = params.viewport_width;
    var parent_height = params.viewport_height;
    
    if (node.parent_idx != 0xFFFFFFFFu) {
        let parent = nodes[node.parent_idx];
        parent_x = parent.computed_x;
        parent_y = parent.computed_y;
        parent_width = parent.computed_width;
        parent_height = parent.computed_height;
    }

    let fixed = (node.style_flags & STYLE_POSITION_FIXED) != 0u;
    let positioned = fixed || (node.style_flags & STYLE_POSITION_ABSOLUTE) != 0u;
    let block = (node.style_flags & STYLE_DISPLAY_BLOCK) != 0u;
    let inline_display = (node.style_flags & STYLE_DISPLAY_INLINE) != 0u;

    var containing_x = parent_x;
    var containing_y = parent_y;
    var containing_width = parent_width;
    var containing_height = parent_height;

    if (fixed) {
        containing_x = 0.0;
        containing_y = 0.0;
        containing_width = params.viewport_width;
        containing_height = params.viewport_height;
    }
    
    let available_width = available_inline_size(node, containing_width);
    let available_height = available_block_size(node, containing_height);

    node.computed_x = containing_x + node.margin_left + node.padding_left;
    node.computed_y = containing_y + node.margin_top + node.padding_top;

    if (inline_display) {
        node.computed_width = explicit_or(node.computed_width, node.padding_left + node.padding_right);
        node.computed_height = explicit_or(node.computed_height, node.padding_top + node.padding_bottom);
    } else if (positioned || block) {
        node.computed_width = explicit_or(node.computed_width, available_width);
        node.computed_height = explicit_or(node.computed_height, available_height);
    } else {
        node.computed_width = explicit_or(node.computed_width, available_width);
        node.computed_height = explicit_or(node.computed_height, node.padding_top + node.padding_bottom);
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
