struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _pad: f32,
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

// ─── Morrowind atmosphere ───────────────────────────────────
// NOTE: SKY_ZENITH is also the clear color in state.rs — keep in sync
const SKY_ZENITH: vec3<f32> = vec3<f32>(0.52, 0.50, 0.47);    // warm grey overcast
const SKY_HORIZON: vec3<f32> = vec3<f32>(0.58, 0.55, 0.48);   // dusty haze
const SKY_HORIZON_SUN: vec3<f32> = vec3<f32>(0.62, 0.56, 0.45); // faint amber glow near sun

// ─── Fog — dense, close, oppressive ────────────────────────
const FOG_START: f32 = 10.0;
const FOG_END: f32 = 100.0;
const FOG_HEIGHT_FALLOFF: f32 = 0.025; // thick in valleys
const SEA_LEVEL: f32 = 48.0; // must match chunk.rs SEA_LEVEL

// ─── Lighting — soft, diffuse, low contrast ────────────────
const SHADOW_COLOR: vec3<f32> = vec3<f32>(0.35, 0.33, 0.30);  // warm grey shadows (not blue)
const BOUNCE_COLOR: vec3<f32> = vec3<f32>(0.28, 0.27, 0.22);  // muted earth bounce

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

// ─── Sky: overcast haze with faint sun glow ─────────────────
fn compute_sky(view_dir: vec3<f32>, light_dir: vec3<f32>) -> vec3<f32> {
    let up_factor = clamp(view_dir.y, 0.0, 1.0);
    // Morrowind sky barely changes — mostly uniform overcast
    let horizon_blend = 1.0 - pow(up_factor, 0.6);

    // Very faint sun presence — diffused through thick cloud/ash
    let sun_dot = clamp(dot(view_dir, light_dir), 0.0, 1.0);
    let sun_glow = pow(sun_dot, 12.0) * 0.25; // subtle, high power = tight
    let horizon_color = mix(SKY_HORIZON, SKY_HORIZON_SUN, sun_glow);

    return mix(SKY_ZENITH, horizon_color, horizon_blend);
}

// ─── Filmic tone mapping (softer than ACES) ─────────────────
fn tonemap(x: vec3<f32>) -> vec3<f32> {
    // Hable/Uncharted 2 — gives a moodier, less contrasty look than ACES
    let a = x * 0.15 + 0.05;
    let b = x * 0.15 + 0.50;
    let c = x * 0.002 + 0.01;
    let d = x * 0.02 + 0.30;
    return (a / b) - (c / d);
}

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
    let light_dir = sun.direction; // pre-normalized on CPU
    let view_vec = in.world_pos - camera.camera_pos;
    let dist = length(view_vec);
    let view_dir = view_vec / dist;

    // ─── Diffuse: half-lambert, soft wrap ────────────────
    let raw_ndotl = dot(n, light_dir);
    let ndotl = raw_ndotl * 0.5 + 0.5;

    // Muted warm light — overcast, diffused through clouds
    let warm_light = sun.color * vec3<f32>(0.95, 0.90, 0.80);
    let light_color = mix(SHADOW_COLOR, warm_light, ndotl);
    let diffuse = light_color * 0.50; // low intensity — overcast

    // ─── Ambient: heavy, uniform — overcast sky ─────────
    let sky_factor = n.y * 0.5 + 0.5;
    let sky_ambient = vec3<f32>(0.42, 0.40, 0.36) * sun.ambient;
    let ground_bounce = BOUNCE_COLOR * sun.ambient * 0.5;
    let ambient = mix(ground_bounce, sky_ambient, sky_factor);

    // ─── Combine — no rim light, no subsurface (too modern) ──
    var lit_color = in.color * (ambient + diffuse);

    // ─── Dense fog ──────────────────────────────────────
    let frag_height = in.world_pos.y;
    let cam_height = camera.camera_pos.y;

    // Distance fog — close and thick
    let dist_fog = clamp((dist - FOG_START) / (FOG_END - FOG_START), 0.0, 1.0);
    // Smooth curve — things fade gradually then vanish
    let dist_fog_smooth = dist_fog * dist_fog;

    // Height fog — much thicker in low areas
    let height_diff = SEA_LEVEL - min(frag_height, cam_height);
    let height_fog = clamp(1.0 - exp(-max(height_diff, 0.0) * FOG_HEIGHT_FALLOFF), 0.0, 0.7);

    let total_fog = clamp(dist_fog_smooth + height_fog * dist_fog_smooth, 0.0, 1.0);

    // Fog fades to the sky color in that direction
    let fog_sky = compute_sky(view_dir, light_dir);
    lit_color = mix(lit_color, fog_sky, total_fog);

    // ─── Tone mapping + color grading ───────────────────
    lit_color = tonemap(lit_color * 0.95); // low exposure

    // Desaturate slightly — washed out, ashy
    let luma = dot(lit_color, vec3<f32>(0.2126, 0.7152, 0.0722));
    lit_color = mix(vec3<f32>(luma), lit_color, 0.85);

    // Slight warm tint — everything shifted toward brown/amber
    lit_color = lit_color * vec3<f32>(1.04, 1.0, 0.93);

    return vec4<f32>(lit_color, 1.0);
}

// ─── Sun disc: barely visible through the haze ──────────────
@fragment
fn fs_sun(in: VertexOutput) -> @location(0) vec4<f32> {
    let view_vec = in.world_pos - camera.camera_pos;
    let view_dir = normalize(view_vec);
    let light_dir = sun.direction;

    let sky = compute_sky(view_dir, light_dir);

    // Sun barely pierces through — pale smudge in the overcast
    let sun_bright = vec3<f32>(0.72, 0.68, 0.58);

    // SUN_DISTANCE and SUN_SIZE must match state.rs constants
    let sun_pos = sun.direction * 800.0;
    let to_center = length(in.world_pos - sun_pos) / 22.0;
    let disc = clamp(1.0 - to_center, 0.0, 1.0);
    let soft_disc = pow(disc, 3.0); // tighter falloff — small bright core

    var color = mix(sky, sun_bright, soft_disc * 0.6); // only 60% blend — hazy
    color = tonemap(color * 0.95);

    // Same desaturation as world
    let luma = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    color = mix(vec3<f32>(luma), color, 0.85);
    color = color * vec3<f32>(1.04, 1.0, 0.93);

    return vec4<f32>(color, 1.0);
}
