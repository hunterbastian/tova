// ═══════════════════════════════════════════════════════════════
//  HUD — Vignette, Compass, Stamina Bar, God Mode Indicator
//  Single fullscreen pass, all procedural from uniforms.
// ═══════════════════════════════════════════════════════════════

struct HudUniform {
    yaw: f32,
    stamina: f32,
    god_mode: f32,
    aspect: f32,
    time: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

@group(0) @binding(0)
var<uniform> hud: HudUniform;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_hud(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
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

// ─── Helpers ────────────────────────────────────────────────

/// Smooth box SDF for rounded rectangles.
fn box_sdf(p: vec2<f32>, half_size: vec2<f32>, radius: f32) -> f32 {
    let d = abs(p) - half_size + radius;
    return length(max(d, vec2<f32>(0.0))) + min(max(d.x, d.y), 0.0) - radius;
}

/// Smooth pulse — 1 when x is near center, fades with width.
fn pulse(x: f32, center: f32, width: f32) -> f32 {
    let d = abs(x - center) / width;
    return smoothstep(1.0, 0.0, d);
}

/// Wrap angle difference to [-PI, PI].
fn angle_diff(a: f32, b: f32) -> f32 {
    let PI = 3.14159265;
    var d = a - b;
    d = ((d + PI) % (2.0 * PI)) - PI;
    // Handle negative modulo
    if d < -PI { d += 2.0 * PI; }
    return d;
}

@fragment
fn fs_hud(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    var color = vec4<f32>(0.0, 0.0, 0.0, 0.0);

    // ─── Vignette ────────────────────────────────────────
    let center = vec2<f32>(0.5, 0.5);
    let dist = length((uv - center) * vec2<f32>(1.0, 1.0 / hud.aspect));
    let vignette = smoothstep(0.28, 0.78, dist) * 0.55;  // heavier, closes in more
    // Cold dark tint — oppressive, not cozy
    color = vec4<f32>(0.02, 0.01, 0.04, vignette);

    // ─── Compass bar ─────────────────────────────────────
    // Thin bar at the very top of the screen
    let compass_y = uv.y;
    let compass_height = 0.025;
    let compass_fade = smoothstep(0.0, compass_height * 0.5, compass_y)
                     * smoothstep(compass_height, compass_height * 0.5, compass_y);

    if compass_y < compass_height {
        // Background bar — darker
        let bar_alpha = compass_fade * 0.45;
        color = mix(color, vec4<f32>(0.03, 0.02, 0.05, bar_alpha), bar_alpha);

        // Map yaw to compass position
        // The compass shows ~120° of the full 360° circle
        let PI = 3.14159265;
        let fov_span = PI * 0.667; // ~120 degrees visible
        let compass_center = hud.yaw;

        // Cardinal directions: N=0, E=PI/2, S=PI, W=-PI/2
        let cardinals = array<f32, 8>(
            0.0,          // N
            PI * 0.25,    // NE
            PI * 0.5,     // E
            PI * 0.75,    // SE
            PI,           // S
            -PI * 0.75,   // SW
            -PI * 0.5,    // W
            -PI * 0.25,   // NW
        );

        for (var i = 0; i < 8; i++) {
            let diff = angle_diff(cardinals[i], compass_center);
            let screen_x = 0.5 + diff / fov_span;

            if screen_x > 0.08 && screen_x < 0.92 {
                let dx = abs(uv.x - screen_x);

                // Edge fade — ticks smoothly disappear near screen edges
                let edge_fade = smoothstep(0.08, 0.16, screen_x) * smoothstep(0.92, 0.84, screen_x);

                // Cardinal (N/E/S/W) = thicker, brighter tick
                let is_cardinal = (i % 2) == 0;

                if is_cardinal {
                    // Thick tick — pale silver
                    let tick = smoothstep(0.003, 0.001, dx) * compass_fade * edge_fade;
                    let tick_color = vec4<f32>(0.58, 0.56, 0.65, tick * 0.80);
                    color = mix(color, tick_color, tick * 0.80);
                } else {
                    // Thin tick — dim grey
                    let tick = smoothstep(0.002, 0.0008, dx) * compass_fade * edge_fade;
                    let tick_color = vec4<f32>(0.40, 0.38, 0.45, tick * 0.45);
                    color = mix(color, tick_color, tick * 0.45);
                }
            }
        }

        // Center mark — small downward notch
        let center_dx = abs(uv.x - 0.5);
        let center_tick = smoothstep(0.0015, 0.0005, center_dx) * compass_fade;
        let center_color = vec4<f32>(0.65, 0.63, 0.72, center_tick * 0.85);
        color = mix(color, center_color, center_tick * 0.9);
    }

    // ─── Stamina bar ─────────────────────────────────────
    // Thin bar at bottom center, only visible when stamina < 1
    if hud.stamina < 0.99 {
        let bar_width = 0.22;
        let bar_height = 0.006;
        let bar_y = 0.955; // near bottom
        let bar_center = vec2<f32>(0.5, bar_y);

        let dx = abs(uv.x - bar_center.x);
        let dy = abs(uv.y - bar_center.y);

        // Soft glow underneath the bar (wider, fainter)
        let glow_radius = 0.018;
        let glow_dist = length(vec2<f32>(dx / (bar_width * 1.2), dy / glow_radius));
        if glow_dist < 1.2 {
            var glow_color: vec3<f32>;
            if hud.stamina > 0.5 {
                glow_color = vec3<f32>(0.35, 0.45, 0.25);
            } else if hud.stamina > 0.2 {
                glow_color = vec3<f32>(0.55, 0.45, 0.20);
            } else {
                glow_color = vec3<f32>(0.55, 0.22, 0.15);
            }
            let glow_a = smoothstep(1.2, 0.3, glow_dist) * 0.12;
            color = mix(color, vec4<f32>(glow_color, glow_a), glow_a);
        }

        if dx < bar_width + 0.005 && dy < bar_height + 0.005 {
            // Background track
            let track_alpha = smoothstep(bar_height + 0.003, bar_height, dy)
                            * smoothstep(bar_width + 0.003, bar_width, dx);
            color = mix(color, vec4<f32>(0.08, 0.07, 0.06, track_alpha * 0.6), track_alpha * 0.6);

            // Filled portion
            let fill_edge = 0.5 - bar_width + bar_width * 2.0 * hud.stamina;
            if uv.x < fill_edge && dx < bar_width && dy < bar_height {
                // Color shifts from green → amber → red as stamina drains
                var bar_color: vec3<f32>;
                if hud.stamina > 0.5 {
                    bar_color = mix(vec3<f32>(0.72, 0.65, 0.35), vec3<f32>(0.45, 0.58, 0.32), (hud.stamina - 0.5) * 2.0);
                } else {
                    bar_color = mix(vec3<f32>(0.65, 0.30, 0.20), vec3<f32>(0.72, 0.65, 0.35), hud.stamina * 2.0);
                }
                let fill_alpha = smoothstep(bar_height + 0.001, bar_height - 0.001, dy);
                color = mix(color, vec4<f32>(bar_color, fill_alpha * 0.8), fill_alpha * 0.8);
            }

            // Rounded cap ends — soft fade at the left and right edges of the track
            let cap_fade = smoothstep(bar_width, bar_width - 0.005, dx);
            // Thin border
            let border_x = smoothstep(bar_width - 0.002, bar_width + 0.001, dx) * smoothstep(bar_width + 0.003, bar_width + 0.001, dx);
            let border_y = smoothstep(bar_height - 0.001, bar_height + 0.001, dy) * smoothstep(bar_height + 0.003, bar_height + 0.001, dy);
            let border_a = max(border_x, border_y) * 0.3 * cap_fade;
            let border_color = vec4<f32>(0.40, 0.35, 0.25, border_a);
            color = mix(color, border_color, border_a);
        }
    }

    // ─── God mode indicator ──────────────────────────────
    // Small glowing symbol in upper-right when god mode is on
    if hud.god_mode > 0.5 {
        let icon_center = vec2<f32>(0.94, 0.05);
        let icon_uv = (uv - icon_center) * vec2<f32>(hud.aspect, 1.0);

        // Gentle breathing pulse
        let pulse_t = sin(hud.time * 2.0) * 0.5 + 0.5; // 0..1, slow cycle
        let pulse_scale = 0.85 + pulse_t * 0.15; // 0.85..1.0

        // Diamond shape
        let diamond = abs(icon_uv.x) + abs(icon_uv.y);
        let glow = smoothstep(0.025 * pulse_scale, 0.008, diamond);
        let inner = smoothstep(0.012, 0.006, diamond);

        // Eerie pale glow — spectral, otherworldly
        let glow_alpha = glow * (0.35 + pulse_t * 0.25);
        let glow_color = vec4<f32>(0.55, 0.60, 0.82, glow_alpha);
        let inner_color = vec4<f32>(0.72, 0.78, 0.95, inner * 0.85);

        color = mix(color, glow_color, glow_alpha);
        color = mix(color, inner_color, inner * 0.85);

        // Small wing-like lines extending from diamond
        let wing_l = smoothstep(0.001, 0.0005, abs(icon_uv.y + icon_uv.x * 0.3))
                   * smoothstep(-0.035, -0.012, icon_uv.x)
                   * smoothstep(icon_uv.x, -0.035, icon_uv.x - 0.001);
        let wing_r = smoothstep(0.001, 0.0005, abs(icon_uv.y - icon_uv.x * 0.3))
                   * smoothstep(0.035, 0.012, icon_uv.x)
                   * smoothstep(-icon_uv.x, -0.035, -icon_uv.x - 0.001);
        let wing_alpha = (wing_l + wing_r) * 0.6;
        let wing_color = vec4<f32>(0.50, 0.55, 0.75, wing_alpha);
        color = mix(color, wing_color, wing_alpha);
    }

    return color;
}
