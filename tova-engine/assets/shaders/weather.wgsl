// ═══════════════════════════════════════════════════════════════
//  Weather overlay — screen-space rain streaks and snowflakes
// ═══════════════════════════════════════════════════════════════

struct WeatherUniform {
    weather_type: f32,  // 0=clear, 1=rain, 2=snow
    intensity: f32,     // 0..1
    time: f32,          // elapsed seconds
    fog_mult: f32,
    sky_darken: f32,
    wind_x: f32,        // wind direction X
    wind_z: f32,        // wind direction Z (mapped to screen Y)
    wind_strength: f32, // 0..1
};

@group(0) @binding(0)
var<uniform> weather: WeatherUniform;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// Fullscreen triangle — 3 vertices, no vertex buffer needed
@vertex
fn vs_weather(@builtin(vertex_index) idx: u32) -> VertexOutput {
    let x = f32(i32(idx & 1u)) * 4.0 - 1.0;
    let y = f32(i32(idx >> 1u)) * 4.0 - 1.0;
    var out: VertexOutput;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>(x * 0.5 + 0.5, 0.5 - y * 0.5);
    return out;
}

// ─── Utility ─────────────────────────────────────────────────

fn hash2d(p: vec2<f32>) -> f32 {
    var n = dot(p, vec2<f32>(127.1, 311.7));
    n = sin(n) * 43758.5453;
    return fract(n);
}

fn hash2d_2(p: vec2<f32>) -> f32 {
    var n = dot(p, vec2<f32>(269.5, 183.3));
    n = sin(n) * 43758.5453;
    return fract(n);
}

// ─── Rain ────────────────────────────────────────────────────

fn rain(uv: vec2<f32>, time: f32, intensity: f32) -> vec4<f32> {
    var accum = 0.0;

    // Five layers — tiny droplets at different depths
    for (var i = 0; i < 5; i++) {
        let layer = f32(i);

        // Each layer: denser grid, faster fall, different opacity
        let scale = 30.0 + layer * 15.0;
        let speed = 2.2 + layer * 0.6;
        let alpha = (0.10 - layer * 0.012) * intensity;

        // Wind tilt — rain driven by wind direction + base angle
        let wind_tilt = weather.wind_x * weather.wind_strength * 0.3 + 0.08;
        let tilted = vec2<f32>(uv.x + uv.y * wind_tilt, uv.y);
        let p = tilted * vec2<f32>(scale, scale * 0.6);
        let grid = floor(p);
        let local = fract(p);

        let h = hash2d(grid + layer * 137.0);
        let h2 = hash2d_2(grid + layer * 137.0);

        // Random position within cell
        let cx = 0.2 + h * 0.6;

        // Horizontal distance — very thin drop
        let x_dist = abs(local.x - cx);
        let thin = smoothstep(0.018, 0.0, x_dist);

        // Falling drop — short teardrop shape with bright leading tip
        let drop_y = fract(local.y + time * speed + h * 10.0);
        let drop_len = 0.06 + h2 * 0.08; // variable length
        let drop_body = smoothstep(0.0, 0.015, drop_y) * smoothstep(drop_len, 0.0, drop_y);
        // Bright tip at the bottom of the drop
        let tip = smoothstep(0.02, 0.0, drop_y) * 1.8;

        accum += thin * (drop_body + tip) * alpha;
    }

    // Rain color: cool silver-blue, slightly brighter than before
    return vec4<f32>(0.72, 0.76, 0.84, accum);
}

// ─── Snow ────────────────────────────────────────────────────

fn snow(uv: vec2<f32>, time: f32, intensity: f32) -> vec4<f32> {
    var accum = 0.0;

    // Four layers — gentle parallax
    for (var i = 0; i < 4; i++) {
        let layer = f32(i);
        let scale = 10.0 + layer * 6.0;
        let alpha = (0.18 - layer * 0.03) * intensity;

        let p = uv * scale;
        let grid = floor(p);
        let local = fract(p);

        let h = hash2d(grid + layer * 200.0);
        let h2 = hash2d_2(grid + layer * 200.0);

        // Wandering snowflake — drifts with wind + sinusoidal wander
        let wind_drift = weather.wind_x * weather.wind_strength * 0.4;
        let drift = sin(time * (0.4 + h * 0.3) + h * 6.28) * 0.25 + wind_drift;
        let cx = 0.5 + drift;
        let cy = fract(0.5 - time * (0.06 + h * 0.05) + h2 * 10.0);

        let d = length(local - vec2<f32>(cx, cy));
        let size = 0.025 + h * 0.035;

        // Soft circle
        let flake = smoothstep(size, size * 0.15, d);
        // Subtle twinkle
        let twinkle = 0.8 + 0.2 * sin(time * 2.0 + h * 20.0);
        accum += flake * alpha * twinkle;
    }

    // Snow color: bright, warm white
    return vec4<f32>(0.88, 0.90, 0.93, accum);
}

// ─── Fragment entry ──────────────────────────────────────────

@fragment
fn fs_weather(in: VertexOutput) -> @location(0) vec4<f32> {
    // Skip if no weather
    if (weather.intensity < 0.001) {
        discard;
    }

    if (weather.weather_type < 0.5) {
        discard; // clear
    } else if (weather.weather_type < 1.5) {
        return rain(in.uv, weather.time, weather.intensity);
    } else {
        return snow(in.uv, weather.time, weather.intensity);
    }
}
