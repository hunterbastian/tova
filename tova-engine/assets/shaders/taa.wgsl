// ═══════════════════════════════════════════════════════════════
//  TAA — Temporal Anti-Aliasing resolve pass
//  Blends current frame with reprojected history.
//  Uses neighborhood clamping to prevent ghosting.
// ═══════════════════════════════════════════════════════════════

struct TaaUniform {
    prev_view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    jitter: vec2<f32>,
    feedback: f32,  // blend factor (0.9 = heavy temporal, 0.5 = responsive)
    _pad: f32,
};

@group(0) @binding(0)
var current_tex: texture_2d<f32>;

@group(0) @binding(1)
var history_tex: texture_2d<f32>;

@group(0) @binding(2)
var depth_tex: texture_depth_2d;

@group(0) @binding(3)
var tex_sampler: sampler;

@group(0) @binding(4)
var<uniform> taa: TaaUniform;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_taa(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let pos = positions[vertex_index];
    var out: VertexOutput;
    out.clip_position = vec4<f32>(pos, 0.0, 1.0);
    out.uv = vec2<f32>(pos.x * 0.5 + 0.5, 1.0 - (pos.y * 0.5 + 0.5));
    return out;
}

/// Reconstruct world position from UV + depth for reprojection.
fn world_from_depth(uv: vec2<f32>, depth: f32) -> vec4<f32> {
    let ndc = vec4<f32>(
        uv.x * 2.0 - 1.0,
        (1.0 - uv.y) * 2.0 - 1.0,
        depth,
        1.0,
    );
    let world_h = taa.inv_view_proj * ndc;
    return vec4<f32>(world_h.xyz / world_h.w, 1.0);
}

/// Clamp color to the neighborhood of the current frame.
/// Prevents ghosting by restricting the history to plausible values.
fn neighborhood_clamp(current: vec3<f32>, history: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    let tex_size = vec2<f32>(textureDimensions(current_tex));
    let texel = 1.0 / tex_size;

    // Sample 3x3 neighborhood to find min/max bounds
    var min_col = current;
    var max_col = current;

    for (var y = -1i; y <= 1i; y++) {
        for (var x = -1i; x <= 1i; x++) {
            if x == 0 && y == 0 { continue; }
            let offset = vec2<f32>(f32(x), f32(y)) * texel;
            let sample = textureSample(current_tex, tex_sampler, uv + offset).rgb;
            min_col = min(min_col, sample);
            max_col = max(max_col, sample);
        }
    }

    return clamp(history, min_col, max_col);
}

@fragment
fn fs_taa(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_size = vec2<f32>(textureDimensions(current_tex));

    // Remove jitter from UV to get unjittered current sample
    let unjittered_uv = in.uv - taa.jitter * 0.5;
    let current = textureSample(current_tex, tex_sampler, unjittered_uv).rgb;

    // Read depth and reproject to previous frame's UV
    let pixel = vec2<i32>(in.clip_position.xy);
    let depth = textureLoad(depth_tex, pixel, 0);
    let world_pos = world_from_depth(in.uv, depth);
    let prev_clip = taa.prev_view_proj * world_pos;
    let prev_ndc = prev_clip.xyz / prev_clip.w;
    let prev_uv = vec2<f32>(
        prev_ndc.x * 0.5 + 0.5,
        1.0 - (prev_ndc.y * 0.5 + 0.5),
    );

    // Check if reprojected UV is valid (on screen)
    if prev_uv.x < 0.0 || prev_uv.x > 1.0 || prev_uv.y < 0.0 || prev_uv.y > 1.0 {
        // No history available — use current frame only
        return vec4<f32>(current, 1.0);
    }

    // Sample and clamp history
    let raw_history = textureSample(history_tex, tex_sampler, prev_uv).rgb;
    let clamped_history = neighborhood_clamp(current, raw_history, in.uv);

    // Blend
    let result = mix(current, clamped_history, taa.feedback);
    return vec4<f32>(result, 1.0);
}
