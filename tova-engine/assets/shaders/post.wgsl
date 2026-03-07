struct PostUniform {
    bloom_threshold: f32,
    bloom_intensity: f32,
    bloom_enabled: f32,
    volumetric_enabled: f32,
    color_grade_amount: f32,
    vignette_strength: f32,
    dither_strength: f32,
    rain_intensity: f32,
    sun_screen_pos: vec2<f32>,
    sun_glare_strength: f32,
    near_plane: f32,
    far_plane: f32,
    time: f32,
    volumetric_decay: f32,
    volumetric_weight: f32,
    volumetric_density: f32,
    volumetric_steps: f32,
    rain_speed: f32,
    rain_slant: f32,
    rain_scale: f32,
    _pad0: vec3<f32>,
};

struct BlurUniform {
    direction: vec2<f32>,
    texel_size: vec2<f32>,
};

@group(0) @binding(0)
var extract_source: texture_2d<f32>;

@group(0) @binding(1)
var extract_sampler: sampler;

@group(0) @binding(2)
var<uniform> extract_post: PostUniform;

@group(1) @binding(0)
var blur_source: texture_2d<f32>;

@group(1) @binding(1)
var blur_sampler: sampler;

@group(1) @binding(2)
var<uniform> blur_params: BlurUniform;

@group(2) @binding(0)
var composite_scene: texture_2d<f32>;

@group(2) @binding(1)
var composite_bloom_half: texture_2d<f32>;

@group(2) @binding(2)
var composite_bloom_quarter: texture_2d<f32>;

@group(2) @binding(3)
var composite_sampler: sampler;

@group(2) @binding(4)
var composite_depth: texture_depth_2d;

@group(2) @binding(5)
var<uniform> composite_post: PostUniform;

struct PostVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

fn aces_fitted(color: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((color * (a * color + b)) / (color * (c * color + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}

fn hash_noise(p: vec2<f32>) -> f32 {
    let h = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(h) * 43758.5453);
}

fn pixelate_uv(uv: vec2<f32>, dimensions: vec2<u32>) -> vec2<f32> {
    let size = vec2<f32>(dimensions);
    return (floor(uv * size) + vec2<f32>(0.5, 0.5)) / size;
}

fn posterize(color: vec3<f32>, levels: f32) -> vec3<f32> {
    return floor(color * levels + 0.5) / levels;
}

fn color_grade(color: vec3<f32>, amount: f32) -> vec3<f32> {
    let luma = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    var graded = mix(vec3<f32>(luma), color, 1.0 + amount * 0.18);
    graded *= mix(vec3<f32>(1.0), vec3<f32>(0.94, 0.98, 1.05), amount);
    return graded;
}

fn pixel_rain_layer(
    uv: vec2<f32>,
    time: f32,
    scale: f32,
    speed: f32,
    slant: f32,
    density: f32,
    min_length: f32,
    max_length: f32,
) -> f32 {
    let warped = vec2<f32>(uv.x + uv.y * slant, uv.y - time * speed);
    let grid = vec2<f32>(scale, scale * 0.42);
    let cell = floor(warped * grid);
    let local = fract(warped * grid);

    let activation = step(1.0 - density, hash_noise(cell + vec2<f32>(13.0, 5.0)));
    let width = 1.0 - smoothstep(0.10, 0.34, abs(local.x - 0.5));
    let streak_length = mix(min_length, max_length, hash_noise(cell + vec2<f32>(31.0, 17.0)));
    let streak = smoothstep(0.0, 0.06, local.y)
        * (1.0 - smoothstep(streak_length, streak_length + 0.08, local.y));
    let taper = 1.0 - smoothstep(0.0, streak_length + 0.08, local.y) * 0.35;

    return activation * width * streak * taper;
}

fn pixel_rain(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    let far_layer = pixel_rain_layer(
        uv,
        composite_post.time,
        composite_post.rain_scale,
        composite_post.rain_speed,
        composite_post.rain_slant,
        composite_post.rain_intensity * 0.72,
        0.28,
        0.62,
    );
    let near_layer = pixel_rain_layer(
        uv + vec2<f32>(0.17, 0.0),
        composite_post.time,
        composite_post.rain_scale * 0.72,
        composite_post.rain_speed * 1.28,
        composite_post.rain_slant * 1.45,
        composite_post.rain_intensity * 0.52,
        0.22,
        0.54,
    );

    let sky_visibility = smoothstep(0.72, 1.0, depth);
    let horizon_boost = 0.75 + (1.0 - uv.y) * 0.45;
    let rain = (far_layer * 0.65 + near_layer) * composite_post.rain_intensity;
    let sparkle = hash_noise(floor(uv * composite_post.rain_scale * 0.5) + vec2<f32>(composite_post.time * 3.0, 7.0));
    let glint = smoothstep(0.86, 1.0, sparkle) * rain * 0.16;
    let visibility = mix(0.55, 1.0, sky_visibility) * horizon_boost;

    return vec3<f32>(0.56, 0.60, 0.64) * (rain * visibility + glint);
}

@vertex
fn vs_fullscreen(@builtin(vertex_index) vertex_index: u32) -> PostVertexOutput {
    var out: PostVertexOutput;
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(3.0, 1.0),
    );

    let pos = positions[vertex_index];
    out.clip_position = vec4<f32>(pos, 0.0, 1.0);
    out.uv = pos * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    return out;
}

@fragment
fn fs_extract_bloom(in: PostVertexOutput) -> @location(0) vec4<f32> {
    let src = textureSample(extract_source, extract_sampler, in.uv).rgb;
    if extract_post.bloom_enabled < 0.5 {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    let lum = dot(src, vec3<f32>(0.2126, 0.7152, 0.0722));
    let threshold = extract_post.bloom_threshold;
    let knee = 0.25;
    let soft = clamp((lum - threshold + knee) / (2.0 * knee + 1e-4), 0.0, 1.0);
    let contribution = max(lum - threshold, 0.0) + soft * soft * knee;
    let scale = contribution / max(lum, 1e-4);
    return vec4<f32>(src * scale, 1.0);
}

@fragment
fn fs_blur(in: PostVertexOutput) -> @location(0) vec4<f32> {
    let dir = blur_params.direction * blur_params.texel_size;

    let w0 = 0.227027;
    let w1 = 0.1945946;
    let w2 = 0.1216216;
    let w3 = 0.054054;

    var color = textureSample(blur_source, blur_sampler, in.uv).rgb * w0;
    color += textureSample(blur_source, blur_sampler, in.uv + dir * 1.0).rgb * w1;
    color += textureSample(blur_source, blur_sampler, in.uv - dir * 1.0).rgb * w1;
    color += textureSample(blur_source, blur_sampler, in.uv + dir * 2.0).rgb * w2;
    color += textureSample(blur_source, blur_sampler, in.uv - dir * 2.0).rgb * w2;
    color += textureSample(blur_source, blur_sampler, in.uv + dir * 3.0).rgb * w3;
    color += textureSample(blur_source, blur_sampler, in.uv - dir * 3.0).rgb * w3;

    return vec4<f32>(color, 1.0);
}

@fragment
fn fs_composite(in: PostVertexOutput) -> @location(0) vec4<f32> {
    let scene_uv = pixelate_uv(in.uv, textureDimensions(composite_scene));
    let bloom_half_uv = pixelate_uv(in.uv, textureDimensions(composite_bloom_half));
    let bloom_quarter_uv = pixelate_uv(in.uv, textureDimensions(composite_bloom_quarter));
    var hdr = textureSample(composite_scene, composite_sampler, scene_uv).rgb;

    let bloom_half = textureSample(composite_bloom_half, composite_sampler, bloom_half_uv).rgb;
    let bloom_quarter = textureSample(composite_bloom_quarter, composite_sampler, bloom_quarter_uv).rgb;

    if composite_post.bloom_enabled > 0.5 {
        hdr += (bloom_half + bloom_quarter * 0.75) * composite_post.bloom_intensity;
    }

    let depth_size = vec2<f32>(textureDimensions(composite_depth));
    let depth_coord = clamp(vec2<i32>(scene_uv * depth_size), vec2<i32>(0), vec2<i32>(depth_size) - vec2<i32>(1));
    let depth = textureLoad(composite_depth, depth_coord, 0);

    if depth >= 0.9999 {
        let horizon = vec3<f32>(0.20, 0.19, 0.18);
        let zenith = vec3<f32>(0.10, 0.12, 0.15);
        let gradient_t = clamp(pow(1.0 - in.uv.y, 1.35), 0.0, 1.0);
        let sky = mix(horizon, zenith, gradient_t);
        hdr = mix(hdr, sky, 0.9);
    }

    let to_sun = composite_post.sun_screen_pos - in.uv;
    let sun_dist = length(to_sun);
    let glare = exp(-sun_dist * 14.0) * composite_post.sun_glare_strength;

    var shafts = 0.0;
    if composite_post.volumetric_enabled > 0.5 {
        let steps = i32(clamp(composite_post.volumetric_steps, 1.0, 24.0));
        let delta = to_sun * composite_post.volumetric_density / max(f32(steps), 1.0);
        var sample_uv = in.uv;
        var illumination = 1.0;

        for (var i = 0; i < 24; i = i + 1) {
            if i >= steps {
                break;
            }

            sample_uv += delta;
            if sample_uv.x < 0.0 || sample_uv.x > 1.0 || sample_uv.y < 0.0 || sample_uv.y > 1.0 {
                continue;
            }

            let coord = clamp(vec2<i32>(sample_uv * depth_size), vec2<i32>(0), vec2<i32>(depth_size) - vec2<i32>(1));
            let depth_sample = textureLoad(composite_depth, coord, 0);
            let visible = select(0.0, 1.0, depth_sample >= 0.9995);
            shafts += visible * illumination;
            illumination *= composite_post.volumetric_decay;
        }

        shafts = shafts * composite_post.volumetric_weight / max(f32(steps), 1.0);
    }

    hdr += vec3<f32>(1.0, 0.95, 0.85) * shafts;
    hdr += vec3<f32>(1.0, 0.92, 0.76) * glare;

    var color = aces_fitted(hdr);
    color = color_grade(color, composite_post.color_grade_amount);
    color = posterize(color, 28.0);
    color += pixel_rain(in.uv, depth);

    let vignette_dist = distance(in.uv, vec2<f32>(0.5, 0.5));
    let vignette = smoothstep(0.72, 0.34, vignette_dist);
    color *= mix(1.0, vignette, composite_post.vignette_strength);

    let scanline = 0.985 + sin(scene_uv.y * depth_size.y * 3.14159) * 0.015;
    color *= scanline;

    let noise = hash_noise(scene_uv * 1337.0 + vec2<f32>(composite_post.time, composite_post.time * 0.37));
    color += (noise - 0.5) * composite_post.dither_strength * 1.35;

    return vec4<f32>(clamp(color, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
