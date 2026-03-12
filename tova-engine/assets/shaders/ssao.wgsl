// ═══════════════════════════════════════════════════════════════
//  SSAO — Screen-Space Ambient Occlusion
//  Samples depth in a hemisphere around each pixel.
//  Output: single-channel occlusion factor (0=occluded, 1=open)
//  Applied as a multiply on the scene in a composite pass.
// ═══════════════════════════════════════════════════════════════

struct CameraUniform {
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _pad: f32,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(0) @binding(1)
var depth_tex: texture_depth_2d;

const NUM_SAMPLES: i32 = 8;
const RADIUS: f32 = 0.8;
const BIAS: f32 = 0.05;
const INTENSITY: f32 = 0.20;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_ssao(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
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

/// Reconstruct view-space position from UV + depth.
fn view_pos_from_depth(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    let ndc = vec4<f32>(
        uv.x * 2.0 - 1.0,
        (1.0 - uv.y) * 2.0 - 1.0,
        depth,
        1.0,
    );
    let world_h = camera.inv_view_proj * ndc;
    return world_h.xyz / world_h.w;
}

/// Hash for pseudo-random sample directions (deterministic per-pixel).
fn hash3(p: vec2<f32>) -> vec3<f32> {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * vec3<f32>(443.897, 441.423, 437.195));
    p3 = p3 + dot(p3, p3.yzx + 19.19);
    return fract(vec3<f32>(
        (p3.x + p3.y) * p3.z,
        (p3.x + p3.z) * p3.y,
        (p3.y + p3.z) * p3.x
    )) * 2.0 - 1.0;
}

@fragment
fn fs_ssao(in: VertexOutput) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(in.clip_position.xy);
    let center_depth = textureLoad(depth_tex, pixel, 0);

    // Skip sky pixels (depth = 1.0)
    if center_depth >= 0.9999 {
        return vec4<f32>(1.0, 1.0, 1.0, 1.0);
    }

    let center_pos = view_pos_from_depth(in.uv, center_depth);

    // Estimate normal from depth neighbors
    let tex_size = vec2<f32>(textureDimensions(depth_tex));
    let texel = 1.0 / tex_size;

    let depth_r = textureLoad(depth_tex, pixel + vec2<i32>(1, 0), 0);
    let depth_u = textureLoad(depth_tex, pixel + vec2<i32>(0, 1), 0);
    let pos_r = view_pos_from_depth(in.uv + vec2<f32>(texel.x, 0.0), depth_r);
    let pos_u = view_pos_from_depth(in.uv + vec2<f32>(0.0, texel.y), depth_u);
    let normal = normalize(cross(pos_r - center_pos, pos_u - center_pos));

    // Sample hemisphere
    var occlusion = 0.0;
    let noise_seed = in.clip_position.xy;

    for (var i = 0i; i < NUM_SAMPLES; i++) {
        // Pseudo-random sample direction
        var sample_dir = hash3(noise_seed + vec2<f32>(f32(i) * 7.13, f32(i) * 3.77));
        // Flip to hemisphere oriented with normal
        if dot(sample_dir, normal) < 0.0 {
            sample_dir = -sample_dir;
        }

        // Scale by radius, weight closer samples more
        let scale = f32(i + 1) / f32(NUM_SAMPLES);
        let sample_pos = center_pos + sample_dir * RADIUS * mix(0.1, 1.0, scale * scale);

        // Project sample to screen space
        let proj = camera.view_proj * vec4<f32>(sample_pos, 1.0);
        let proj_uv = vec2<f32>(
            proj.x / proj.w * 0.5 + 0.5,
            1.0 - (proj.y / proj.w * 0.5 + 0.5),
        );

        // Read depth at projected position
        let sample_pixel = vec2<i32>(proj_uv * tex_size);
        let sample_depth = textureLoad(depth_tex, clamp(sample_pixel, vec2<i32>(0), vec2<i32>(tex_size) - 1), 0);
        let sample_world = view_pos_from_depth(proj_uv, sample_depth);

        // Check if the sample is occluded
        let range_check = smoothstep(0.0, 1.0, RADIUS / abs(length(center_pos - sample_world)));
        let is_occluded = select(0.0, 1.0, length(sample_world - camera.camera_pos) < length(sample_pos - camera.camera_pos) - BIAS);
        occlusion += is_occluded * range_check;
    }

    let ao = 1.0 - (occlusion / f32(NUM_SAMPLES)) * INTENSITY;
    let result = clamp(ao, 0.0, 1.0);

    return vec4<f32>(result, result, result, 1.0);
}
