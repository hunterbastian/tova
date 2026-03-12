// ═══════════════════════════════════════════════════════════════
//  Bloom — brightness extraction + blur + composite
//  Two passes: extract bright pixels, then composite with blur
// ═══════════════════════════════════════════════════════════════

// ─── Extract pass ────────────────────────────────────────────
// Reads the scene color, outputs only pixels above brightness threshold

@group(0) @binding(0)
var scene_tex: texture_2d<f32>;

@group(0) @binding(1)
var scene_sampler: sampler;

const BLOOM_THRESHOLD: f32 = 0.32;      // lower — more glow from dim light
const BLOOM_SOFT_THRESHOLD: f32 = 0.22;  // wider knee
const BLOOM_INTENSITY: f32 = 0.25;       // stronger — dreamy, ethereal

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_bloom(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
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

@fragment
fn fs_bloom_extract(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(scene_tex, scene_sampler, in.uv).rgb;
    let brightness = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));

    // Soft knee — smooth transition around threshold
    let knee = BLOOM_THRESHOLD - BLOOM_SOFT_THRESHOLD;
    let soft = clamp(brightness - knee, 0.0, 2.0 * BLOOM_SOFT_THRESHOLD);
    let contribution = soft * soft / (4.0 * BLOOM_SOFT_THRESHOLD + 0.0001);
    let weight = max(contribution, brightness - BLOOM_THRESHOLD) / max(brightness, 0.0001);

    return vec4<f32>(color * weight, 1.0);
}

// ─── Blur pass ───────────────────────────────────────────────
// 9-tap Gaussian blur (horizontal or vertical, run twice)

@fragment
fn fs_bloom_blur(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_size = vec2<f32>(textureDimensions(scene_tex));
    let texel = 1.0 / tex_size;

    // 9-tap Gaussian weights
    let weights = array<f32, 5>(0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216);

    var result = textureSample(scene_tex, scene_sampler, in.uv).rgb * weights[0];

    // Horizontal blur (for vertical, swap x/y in a second pass)
    for (var i = 1; i < 5; i++) {
        let offset = vec2<f32>(f32(i) * texel.x, 0.0);
        result += textureSample(scene_tex, scene_sampler, in.uv + offset).rgb * weights[i];
        result += textureSample(scene_tex, scene_sampler, in.uv - offset).rgb * weights[i];
    }

    return vec4<f32>(result, 1.0);
}

@fragment
fn fs_bloom_blur_v(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_size = vec2<f32>(textureDimensions(scene_tex));
    let texel = 1.0 / tex_size;
    let weights = array<f32, 5>(0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216);
    var result = textureSample(scene_tex, scene_sampler, in.uv).rgb * weights[0];

    // Vertical blur
    for (var i = 1; i < 5; i++) {
        let offset = vec2<f32>(0.0, f32(i) * texel.y);
        result += textureSample(scene_tex, scene_sampler, in.uv + offset).rgb * weights[i];
        result += textureSample(scene_tex, scene_sampler, in.uv - offset).rgb * weights[i];
    }

    return vec4<f32>(result, 1.0);
}

// ─── Composite pass ─────────────────────────────────────────
// Additively blend blurred bloom over the scene

@group(0) @binding(2)
var bloom_tex: texture_2d<f32>;

@fragment
fn fs_bloom_composite(in: VertexOutput) -> @location(0) vec4<f32> {
    let scene = textureSample(scene_tex, scene_sampler, in.uv).rgb;
    let bloom = textureSample(bloom_tex, scene_sampler, in.uv).rgb;
    let color = scene + bloom * BLOOM_INTENSITY;
    return vec4<f32>(color, 1.0);
}
