// Compositor shader for efficient rendering with damage tracking
//
// Implements tile-based rendering with damage regions and occlusion culling.

struct CompositorParams {
    viewport_width: f32,
    viewport_height: f32,
    tile_size: f32,
    layer_count: u32,
}

struct Tile {
    x: u32,
    y: u32,
    is_damaged: u32,
    is_occluded: u32,
}

struct Layer {
    texture_id: u32,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    opacity: f32,
    z_index: f32,
    clip_x: f32,
    clip_y: f32,
    clip_width: f32,
    clip_height: f32,
}

@group(0) @binding(0) var<uniform> params: CompositorParams;
@group(0) @binding(1) var<storage, read_write> tiles: array<Tile>;
@group(0) @binding(2) var<storage, read> layers: array<Layer>;
@group(0) @binding(3) var output_texture: texture_storage_2d<rgba8unorm, write>;

// Mark tiles that need redrawing based on damage regions
@compute @workgroup_size(16, 16)
fn mark_damage(@builtin(global_invocation_id) id: vec3<u32>) {
    let tile_x = id.x;
    let tile_y = id.y;
    
    let tiles_wide = u32(ceil(params.viewport_width / params.tile_size));
    let tiles_high = u32(ceil(params.viewport_height / params.tile_size));
    
    if (tile_x >= tiles_wide || tile_y >= tiles_high) {
        return;
    }
    
    let tile_idx = tile_y * tiles_wide + tile_x;
    
    // Check if any layer intersects this tile
    let tile_bounds_x = f32(tile_x) * params.tile_size;
    let tile_bounds_y = f32(tile_y) * params.tile_size;
    let tile_bounds_w = params.tile_size;
    let tile_bounds_h = params.tile_size;
    
    var is_damaged = false;
    
    for (var i = 0u; i < params.layer_count; i = i + 1u) {
        let layer = layers[i];
        
        // Check intersection
        if (layer.x < tile_bounds_x + tile_bounds_w &&
            layer.x + layer.width > tile_bounds_x &&
            layer.y < tile_bounds_y + tile_bounds_h &&
            layer.y + layer.height > tile_bounds_y) {
            is_damaged = true;
            break;
        }
    }
    
    tiles[tile_idx].is_damaged = u32(is_damaged);
}

// Occlusion culling - mark fully occluded tiles
@compute @workgroup_size(16, 16)
fn cull_occluded(@builtin(global_invocation_id) id: vec3<u32>) {
    let tile_x = id.x;
    let tile_y = id.y;
    
    let tiles_wide = u32(ceil(params.viewport_width / params.tile_size));
    let tiles_high = u32(ceil(params.viewport_height / params.tile_size));
    
    if (tile_x >= tiles_wide || tile_y >= tiles_high) {
        return;
    }
    
    let tile_idx = tile_y * tiles_wide + tile_x;
    
    let tile_bounds_x = f32(tile_x) * params.tile_size;
    let tile_bounds_y = f32(tile_y) * params.tile_size;
    let tile_bounds_w = params.tile_size;
    let tile_bounds_h = params.tile_size;
    
    // Check if any opaque layer fully covers this tile
    var is_occluded = false;
    
    for (var i = 0u; i < params.layer_count; i = i + 1u) {
        let layer = layers[i];
        
        // Check if layer is opaque
        if (layer.opacity >= 1.0) {
            // Check if layer fully covers tile
            if (layer.x <= tile_bounds_x &&
                layer.y <= tile_bounds_y &&
                layer.x + layer.width >= tile_bounds_x + tile_bounds_w &&
                layer.y + layer.height >= tile_bounds_y + tile_bounds_h) {
                is_occluded = true;
                break;
            }
        }
    }
    
    tiles[tile_idx].is_occluded = u32(is_occluded);
}

// Vertex shader for compositing
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coord: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(input.position, 0.0, 1.0);
    output.tex_coord = input.tex_coord;
    return output;
}

// Fragment shader for compositing
@group(1) @binding(0) var layer_texture: texture_2d<f32>;
@group(1) @binding(1) var layer_sampler: sampler;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(layer_texture, layer_sampler, input.tex_coord);
    return color;
}
