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

    // Diffuse lighting from the sun
    let ndotl = max(dot(n, light_dir), 0.0);
    let diffuse = sun.color * ndotl;

    // Ambient light so shadows aren't pitch black
    let ambient = sun.color * sun.ambient;

    let lit_color = in.color * (ambient + diffuse);

    return vec4<f32>(lit_color, 1.0);
}

// Separate shader for the sword — rendered with its own view-proj (screen-space)
struct SwordCameraUniform {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> sword_camera: SwordCameraUniform;

@vertex
fn vs_sword(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = sword_camera.view_proj * vec4<f32>(in.position, 1.0);
    out.color = in.color;
    out.normal = in.normal;
    out.world_pos = in.position;
    return out;
}

@fragment
fn fs_sword(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);
    let light_dir = normalize(sun.direction);

    let ndotl = max(dot(n, light_dir), 0.0);
    let diffuse = sun.color * ndotl;
    let ambient = sun.color * sun.ambient;

    // Metallic specular highlight for the blade
    let view_dir = normalize(vec3<f32>(0.0, 0.0, 1.0));
    let half_dir = normalize(light_dir + view_dir);
    let spec = pow(max(dot(n, half_dir), 0.0), 32.0) * 0.5;

    let lit_color = in.color * (ambient + diffuse) + sun.color * spec;

    return vec4<f32>(lit_color, 1.0);
}
