struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _pad: f32,
};

struct SunUniform {
    direction: vec3<f32>,
    fog_density: f32,
    color: vec3<f32>,
    ambient: f32,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<uniform> sun: SunUniform;

// Fog settings — Morrowind-style dense atmospheric fog
const FOG_COLOR: vec3<f32> = vec3<f32>(0.62, 0.60, 0.56); // warm overcast grey
const FOG_START: f32 = 20.0;   // fog begins here
const FOG_END: f32 = 120.0;    // fully fogged at this distance

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) world_pos: vec3<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(in.position, 1.0);
    out.color = in.color;
    out.normal = in.normal;
    out.world_pos = in.position;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);
    let light_dir = normalize(sun.direction);

    // Diffuse — half-lambert for soft wrap lighting
    let raw_ndotl = dot(n, light_dir);
    let ndotl = raw_ndotl * 0.5 + 0.5;
    let diffuse = sun.color * ndotl * 0.55;

    // Ambient — muted, slightly warm from above, cooler from below
    let sky_ambient = vec3<f32>(0.50, 0.50, 0.48) * sun.ambient;
    let ground_ambient = vec3<f32>(0.30, 0.28, 0.25) * sun.ambient * 0.5;
    let ambient = mix(ground_ambient, sky_ambient, n.y * 0.5 + 0.5);

    let lit_color = in.color * (ambient + diffuse);

    // Distance fog — controlled by fog_density uniform
    let dist = distance(in.world_pos, camera.camera_pos);
    let fog_factor = clamp((dist - FOG_START) / (FOG_END - FOG_START), 0.0, 1.0);
    let fog = fog_factor * fog_factor * sun.fog_density;

    let final_color = mix(lit_color, FOG_COLOR, fog);

    return vec4<f32>(final_color, 1.0);
}

// Sun disc shader — emissive, fades into fog at distance
@fragment
fn fs_sun(in: VertexOutput) -> @location(0) vec4<f32> {
    let haze = mix(in.color, FOG_COLOR, 0.25 * sun.fog_density);
    return vec4<f32>(haze, 1.0);
}

// Overlay shaders — 2D UI rendered directly in clip space
@vertex
fn vs_overlay(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position.xy, 0.0, 1.0);
    out.color = in.color;
    out.normal = in.normal;
    out.world_pos = vec3<f32>(0.0, 0.0, 0.0);
    return out;
}

@fragment
fn fs_overlay(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, in.normal.x);
}
