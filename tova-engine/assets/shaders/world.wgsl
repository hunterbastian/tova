struct FrameUniform {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    time: f32,
    exposure: f32,
    render_scale: f32,
    near_plane: f32,
    far_plane: f32,
};

struct LightingUniform {
    sun_direction: vec3<f32>,
    sun_intensity: f32,
    sun_color: vec3<f32>,
    ambient: f32,
    fog_color: vec3<f32>,
    fog_base_density: f32,
    fog_height_falloff: f32,
    fog_enabled: f32,
    volumetric_strength: f32,
    shader_pack_enabled: f32,
    water_wave_strength: f32,
    water_specular: f32,
    day_phase: f32,
    _pad0: f32,
};

struct ShadowUniform {
    light_view_proj: array<mat4x4<f32>, 2>,
    cascade_splits: vec4<f32>,
    shadow_params: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> frame: FrameUniform;

@group(1) @binding(0)
var<uniform> lighting: LightingUniform;

@group(2) @binding(0)
var shadow_map: texture_depth_2d_array;

@group(2) @binding(1)
var shadow_sampler: sampler_comparison;

@group(2) @binding(2)
var<uniform> shadow_data: ShadowUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) base_color: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) world_pos: vec3<f32>,
};

fn select_cascade(camera_dist: f32) -> u32 {
    if camera_dist < shadow_data.cascade_splits.x {
        return 0u;
    }
    return 1u;
}

fn sample_shadow(world_pos: vec3<f32>, normal: vec3<f32>, camera_dist: f32) -> f32 {
    if shadow_data.shadow_params.w < 0.5 {
        return 1.0;
    }

    let cascade = select_cascade(camera_dist);
    let light_clip = shadow_data.light_view_proj[cascade] * vec4<f32>(world_pos, 1.0);
    if light_clip.w <= 0.0 {
        return 1.0;
    }

    let ndc = light_clip.xyz / light_clip.w;
    let uv = ndc.xy * 0.5 + vec2<f32>(0.5, 0.5);
    if uv.x <= 0.0 || uv.x >= 1.0 || uv.y <= 0.0 || uv.y >= 1.0 || ndc.z <= 0.0 || ndc.z >= 1.0 {
        return 1.0;
    }

    let map_size = max(shadow_data.shadow_params.x, 1.0);
    let texel = 1.0 / map_size;
    let radius = shadow_data.shadow_params.y;
    let bias = shadow_data.shadow_params.z + (1.0 - max(dot(normal, normalize(lighting.sun_direction)), 0.0)) * 0.0015;
    let layer = i32(cascade);

    if radius < 0.5 {
        return textureSampleCompare(shadow_map, shadow_sampler, uv, layer, ndc.z - bias);
    }

    var visibility = 0.0;
    var taps = 0.0;

    if radius < 1.25 {
        for (var y = 0; y < 2; y = y + 1) {
            for (var x = 0; x < 2; x = x + 1) {
                let offset = (vec2<f32>(f32(x), f32(y)) - vec2<f32>(0.5, 0.5)) * texel;
                visibility += textureSampleCompare(shadow_map, shadow_sampler, uv + offset, layer, ndc.z - bias);
                taps += 1.0;
            }
        }
    } else {
        for (var y = -1; y <= 1; y = y + 1) {
            for (var x = -1; x <= 1; x = x + 1) {
                let offset = vec2<f32>(f32(x), f32(y)) * texel;
                visibility += textureSampleCompare(shadow_map, shadow_sampler, uv + offset, layer, ndc.z - bias);
                taps += 1.0;
            }
        }
    }

    return visibility / max(taps, 1.0);
}

fn apply_height_fog(color: vec3<f32>, world_pos: vec3<f32>, camera_dist: f32, sun_height: f32) -> vec3<f32> {
    let fog_density = lighting.fog_base_density * lighting.fog_enabled;
    if fog_density <= 0.0 {
        return color;
    }

    let dist_term = 1.0 - exp(-camera_dist * fog_density);
    let height_term = exp(-max(world_pos.y, 0.0) * lighting.fog_height_falloff);
    let sky_warmth = clamp(sun_height * 0.5 + 0.5, 0.0, 1.0);
    let fog_tint = mix(
        lighting.fog_color * vec3<f32>(0.82, 0.90, 0.98),
        lighting.fog_color * vec3<f32>(0.96, 0.94, 0.92),
        sky_warmth,
    );
    let fog = clamp(dist_term * (0.80 + 0.40 * height_term), 0.0, 1.0);
    return mix(color, fog_tint, fog);
}

@vertex
fn vs_world(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = frame.view_proj * vec4<f32>(in.position, 1.0);
    out.base_color = in.color;
    out.normal = normalize(in.normal);
    out.world_pos = in.position;
    return out;
}

@fragment
fn fs_world(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);
    let light_dir = normalize(lighting.sun_direction);
    let view_dir = normalize(frame.camera_pos - in.world_pos);

    let ndotl = max(dot(n, light_dir), 0.0);
    let half_lambert = ndotl * 0.5 + 0.5;
    let diffuse = lighting.sun_color * half_lambert * lighting.sun_intensity;

    let sky_ambient = vec3<f32>(0.30, 0.33, 0.36) * lighting.ambient;
    let ground_ambient = vec3<f32>(0.11, 0.10, 0.09) * lighting.ambient;
    let ambient = mix(ground_ambient, sky_ambient, n.y * 0.5 + 0.5);

    let camera_dist = distance(in.world_pos, frame.camera_pos);
    let shadow_vis = sample_shadow(in.world_pos, n, camera_dist);

    var lit = in.base_color * (ambient + diffuse * shadow_vis);

    let looks_like_water = in.base_color.b > in.base_color.r + 0.025 && in.base_color.b > in.base_color.g;
    if looks_like_water && n.y > 0.8 {
        let wave = sin((in.world_pos.x + frame.time * 2.0) * 0.18)
            * cos((in.world_pos.z - frame.time * 1.6) * 0.15)
            * lighting.water_wave_strength;
        let fresnel = pow(1.0 - max(dot(view_dir, vec3<f32>(0.0, 1.0, 0.0)), 0.0), 3.0);
        let reflected = reflect(-light_dir, n);
        let sparkle = pow(max(dot(reflected, view_dir), 0.0), 36.0) * lighting.water_specular;
        let water_tint = vec3<f32>(0.08, 0.11, 0.12) + vec3<f32>(wave * 0.16);
        lit = mix(lit, water_tint, 0.68) + fresnel * 0.08 + sparkle * 0.45;
    }

    if lighting.shader_pack_enabled < 0.5 {
        let simple = in.base_color * (ambient + diffuse * 0.55);
        return vec4<f32>(simple, 1.0);
    }

    let fogged = apply_height_fog(lit, in.world_pos, camera_dist, light_dir.y);
    return vec4<f32>(fogged, 1.0);
}

@fragment
fn fs_sun(in: VertexOutput) -> @location(0) vec4<f32> {
    let haze = mix(in.base_color, lighting.fog_color, lighting.fog_enabled * 0.4 + 0.15);
    let glow = 0.08 + 0.20 * clamp(lighting.sun_direction.y * 0.8 + 0.2, 0.0, 1.0);
    return vec4<f32>(haze + glow, 1.0);
}

@vertex
fn vs_overlay(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position.xy, 0.0, 1.0);
    out.base_color = in.color;
    out.normal = in.normal;
    out.world_pos = vec3<f32>(0.0, 0.0, 0.0);
    return out;
}

@fragment
fn fs_overlay(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.base_color, in.normal.x);
}
