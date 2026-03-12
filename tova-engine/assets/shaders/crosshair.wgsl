// ═══════════════════════════════════════════════════════════════
//  Crosshair — fullscreen SDF, perfectly anti-aliased
//  Drawn as a single fullscreen triangle, procedural in fragment.
// ═══════════════════════════════════════════════════════════════

struct CrosshairUniform {
    aspect: f32,
};

@group(0) @binding(0)
var<uniform> ch: CrosshairUniform;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_crosshair(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Fullscreen triangle
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let pos = positions[vertex_index];
    var out: VertexOutput;
    out.clip_position = vec4<f32>(pos, 0.0, 1.0);
    // UV centered at screen center, aspect-corrected
    out.uv = vec2<f32>(pos.x * ch.aspect, pos.y);
    return out;
}

// ─── SDF helpers ────────────────────────────────────────────

/// Rounded box SDF — distance to a rounded rectangle.
fn sd_box(p: vec2<f32>, half: vec2<f32>, r: f32) -> f32 {
    let d = abs(p) - half + r;
    return length(max(d, vec2<f32>(0.0))) + min(max(d.x, d.y), 0.0) - r;
}

@fragment
fn fs_crosshair(in: VertexOutput) -> @location(0) vec4<f32> {
    let p = in.uv;

    // ─── Parameters (in NDC-ish space) ─────────────────────
    let thickness = 0.0028;       // bar half-thickness
    let bar_len = 0.018;          // bar half-length
    let gap = 0.006;              // gap from center to bar start
    let dot_r = 0.0025;           // center dot radius
    let round = 0.0012;           // corner rounding on bars
    let aa = 0.0015;              // anti-aliasing width

    // ─── Center dot — small circle ─────────────────────────
    let dot_dist = length(p) - dot_r;

    // ─── Four bars — computed as rounded rects offset from center ───
    // Right bar: center at (gap + length/2, 0)
    let bar_half = vec2<f32>(bar_len * 0.5, thickness);
    let bar_offset = gap + bar_len * 0.5;

    let d_right = sd_box(p - vec2<f32>(bar_offset, 0.0), bar_half, round);
    let d_left  = sd_box(p + vec2<f32>(bar_offset, 0.0), bar_half, round);
    let d_top   = sd_box(p - vec2<f32>(0.0, bar_offset), bar_half.yx, round);
    let d_bot   = sd_box(p + vec2<f32>(0.0, bar_offset), bar_half.yx, round);

    // Union of all bar SDFs + dot
    let d_bars = min(min(d_right, d_left), min(d_top, d_bot));
    let d_shape = min(d_bars, dot_dist);

    // ─── Soft shadow / glow underneath ─────────────────────
    let shadow_alpha = smoothstep(0.008, 0.0, d_shape) * 0.25;
    let shadow = vec4<f32>(0.0, 0.0, 0.0, shadow_alpha);

    // ─── Foreground — warm white with smooth edges ─────────
    let fg_alpha = smoothstep(aa, -aa * 0.5, d_shape) * 0.65;
    let fg = vec4<f32>(0.92, 0.89, 0.82, fg_alpha);

    // ─── Thin bright edge highlight (1px inner stroke) ─────
    let edge_dist = abs(d_shape + 0.0004);
    let edge_alpha = smoothstep(0.001, 0.0002, edge_dist) * fg_alpha * 0.3;
    let edge = vec4<f32>(1.0, 0.97, 0.90, edge_alpha);

    // Composite: shadow → foreground → edge highlight
    var color = shadow;
    color = mix(color, fg, fg_alpha);
    color = vec4<f32>(
        mix(color.rgb, edge.rgb, edge_alpha),
        max(color.a, edge.a)
    );

    // Discard fully transparent pixels for performance
    if color.a < 0.005 {
        discard;
    }

    return color;
}
