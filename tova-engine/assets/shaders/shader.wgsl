struct CameraUniform {
    view_proj: mat4x4<f32>,
};

struct SunUniform {
    direction: vec3<f32>,
    _pad: f32,
    color: vec3<f32>,
    ambient: f32,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<uniform> sun: SunUniform;

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

    // Diffuse — half-lambert for softer Minecraft-style shading
    let raw_ndotl = dot(n, light_dir);
    let ndotl = raw_ndotl * 0.5 + 0.5; // wrap lighting
    let diffuse = sun.color * ndotl * 0.65;

    // Ambient — warm tint from sky
    let sky_ambient = vec3<f32>(0.6, 0.7, 0.9) * sun.ambient;
    let ground_ambient = vec3<f32>(0.4, 0.35, 0.3) * sun.ambient * 0.5;
    let ambient = mix(ground_ambient, sky_ambient, n.y * 0.5 + 0.5);

    let lit_color = in.color * (ambient + diffuse);

    return vec4<f32>(lit_color, 1.0);
}

// Sun disc shader — emissive, no lighting applied
@fragment
fn fs_sun(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}

