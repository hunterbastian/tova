struct ShadowPassUniform {
    light_view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> shadow_pass: ShadowPassUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) normal: vec3<f32>,
};

@vertex
fn vs_shadow(in: VertexInput) -> @builtin(position) vec4<f32> {
    return shadow_pass.light_view_proj * vec4<f32>(in.position, 1.0);
}
