struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _pad: f32,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) world_position: vec3<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = camera.view_proj * vec4<f32>(input.position, 1.0);
    output.color = input.color;
    output.normal = normalize(input.normal);
    output.world_position = input.position;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let sun_direction = normalize(vec3<f32>(0.22, 0.78, -0.42));
    let sun_amount = max(dot(input.normal, sun_direction), 0.0);
    let sky_fill = clamp(input.normal.y * 0.5 + 0.5, 0.0, 1.0);
    let ambient_color = mix(vec3<f32>(0.15, 0.16, 0.19), vec3<f32>(0.29, 0.28, 0.24), sky_fill);
    let light_color = mix(vec3<f32>(0.24, 0.26, 0.31), vec3<f32>(0.84, 0.74, 0.58), sun_amount);
    let lit = input.color * (ambient_color + light_color * 0.56);

    let view_distance = distance(input.world_position, camera.camera_pos);
    let distance_fog = smoothstep(16.0, 110.0, view_distance);
    let height_fog = 1.0 - smoothstep(20.0, 82.0, input.world_position.y);
    let fog = clamp(distance_fog * 0.86 + height_fog * 0.24, 0.0, 1.0);
    let fog_color = mix(vec3<f32>(0.09, 0.10, 0.12), vec3<f32>(0.16, 0.16, 0.15), height_fog);

    return vec4<f32>(mix(lit, fog_color, fog), 1.0);
}

struct HudVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) alpha: f32,
};

@vertex
fn vs_hud(input: VertexInput) -> HudVertexOutput {
    var output: HudVertexOutput;
    output.clip_position = vec4<f32>(input.position, 1.0);
    output.color = input.color;
    output.alpha = input.normal.x;
    return output;
}

@fragment
fn fs_hud(input: HudVertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color, input.alpha);
}
