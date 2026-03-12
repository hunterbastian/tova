// ═══════════════════════════════════════════════════════════════
//  Volumetric lighting — screen-space god rays via shadow map
//  Marches from each pixel toward the sun, accumulating
//  in-scattering where the shadow map says "lit".
// ═══════════════════════════════════════════════════════════════

struct CameraUniform {
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _pad: f32,
};

struct SunUniform {
    direction: vec3<f32>,
    _pad: f32,
    color: vec3<f32>,
    ambient: f32,
};

struct ShadowUniform {
    light_vp: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(0) @binding(1)
var<uniform> sun: SunUniform;

@group(0) @binding(2)
var<uniform> shadow_uni: ShadowUniform;

@group(0) @binding(3)
var shadow_map: texture_depth_2d;

@group(0) @binding(4)
var shadow_sampler: sampler_comparison;

@group(0) @binding(5)
var depth_tex: texture_depth_2d;

// ─── Constants ───────────────────────────────────────────────
const NUM_STEPS: i32 = 16;
const MAX_RAY_DIST: f32 = 60.0;
const SCATTERING: f32 = 0.12;     // intensity of light shafts
const SHADOW_BIAS: f32 = 0.004;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_volumetric(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
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

/// Mie scattering phase function — forward scattering lobe.
fn henyey_greenstein(cos_theta: f32, g: f32) -> f32 {
    let g2 = g * g;
    let denom = 1.0 + g2 - 2.0 * g * cos_theta;
    return (1.0 - g2) / (4.0 * 3.14159 * pow(denom, 1.5));
}

/// Reconstruct world position from depth buffer + UV.
fn world_from_depth(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    let ndc = vec4<f32>(
        uv.x * 2.0 - 1.0,
        (1.0 - uv.y) * 2.0 - 1.0,
        depth,
        1.0,
    );
    let world_h = camera.inv_view_proj * ndc;
    return world_h.xyz / world_h.w;
}

/// Check if a world position is in shadow (single tap, no PCF for perf).
fn shadow_at(world_pos: vec3<f32>) -> f32 {
    let light_clip = shadow_uni.light_vp * vec4<f32>(world_pos, 1.0);
    let light_ndc = light_clip.xyz / light_clip.w;
    let uv = vec2<f32>(
        light_ndc.x * 0.5 + 0.5,
        light_ndc.y * -0.5 + 0.5,
    );
    let depth = light_ndc.z;

    if uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 || depth > 1.0 {
        return 1.0;
    }

    return textureSampleCompare(
        shadow_map, shadow_sampler,
        uv, depth - SHADOW_BIAS
    );
}

@fragment
fn fs_volumetric(in: VertexOutput) -> @location(0) vec4<f32> {
    // Read scene depth at this pixel
    let pixel = vec2<i32>(in.clip_position.xy);
    let scene_depth = textureLoad(depth_tex, pixel, 0);

    // Reconstruct world position of the scene pixel
    let world_pos = world_from_depth(in.uv, scene_depth);
    let to_pixel = world_pos - camera.camera_pos;
    let ray_dist = min(length(to_pixel), MAX_RAY_DIST);
    let ray_dir = normalize(to_pixel);

    // Phase function — stronger scattering when looking toward the sun
    let cos_theta = dot(ray_dir, sun.direction);
    let phase = henyey_greenstein(cos_theta, 0.6);

    // March along the ray, accumulating light
    let step_size = ray_dist / f32(NUM_STEPS);
    var accumulated = 0.0;

    for (var i = 1i; i <= NUM_STEPS; i++) {
        let t = f32(i) * step_size;
        let sample_pos = camera.camera_pos + ray_dir * t;

        // Is this point in light?
        let lit = shadow_at(sample_pos);
        accumulated += lit * step_size;
    }

    // Normalize and apply scattering
    let fog_amount = accumulated / MAX_RAY_DIST;
    let light_color = sun.color * SCATTERING * phase;
    let scatter = light_color * fog_amount;

    return vec4<f32>(scatter, fog_amount * SCATTERING);
}
