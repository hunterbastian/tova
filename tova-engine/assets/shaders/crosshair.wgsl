struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vs_crosshair(@location(0) position: vec2<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(position, 0.0, 1.0);
    return out;
}

@fragment
fn fs_crosshair() -> @location(0) vec4<f32> {
    // Semi-transparent white — Morrowind-style minimal crosshair
    return vec4<f32>(0.85, 0.82, 0.75, 0.6);
}
