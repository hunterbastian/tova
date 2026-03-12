// ═══════════════════════════════════════════════════════════════
//  Tova — atmospheric lighting shader
//  Aesthetic: Dark fantasy — brooding, ancient, twilight world
//  Cold fog, deep shadows, muted light piercing heavy overcast
// ═══════════════════════════════════════════════════════════════

struct CameraUniform {
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _pad: f32,
};

struct SunUniform {
    direction: vec3<f32>,
    _pad: f32,
    color: vec3<f32>,
    ambient: f32,
};

struct WeatherUniform {
    weather_type: f32,
    intensity: f32,
    time: f32,
    fog_mult: f32,
    sky_darken: f32,
    wind_x: f32,
    wind_z: f32,
    wind_strength: f32,
    sky_zenith: vec4<f32>,
    sky_horizon: vec4<f32>,
    sky_horizon_sun: vec4<f32>,
    sky_nadir: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<uniform> sun: SunUniform;

@group(2) @binding(0)
var<uniform> weather: WeatherUniform;

// ─── Shadow mapping ─────────────────────────────────────────
struct ShadowUniform {
    light_vp: mat4x4<f32>,
};

@group(3) @binding(0)
var<uniform> shadow_uni: ShadowUniform;

@group(3) @binding(1)
var shadow_map: texture_depth_2d;

@group(3) @binding(2)
var shadow_sampler: sampler_comparison;

// ─── Ocean depth texture (terrain depth copy for transparency) ───────
@group(4) @binding(0)
var ocean_depth_tex: texture_depth_2d;
@group(4) @binding(1)
var ocean_depth_sampler: sampler;

// ─── Atmosphere colors (driven by time-of-day via weather uniform) ───

// ─── Fog ──────────────────────────────────────────────────────
const FOG_START: f32 = 6.0;
const FOG_END: f32 = 85.0;       // closer — world feels enclosed, mysterious
const FOG_HEIGHT_FALLOFF: f32 = 0.04;
const SEA_LEVEL: f32 = 48.0;

// ─── Lighting ─────────────────────────────────────────────────
const SHADOW_COLOR: vec3<f32> = vec3<f32>(0.18, 0.16, 0.22);  // cold blue-purple shadows
const BOUNCE_COLOR: vec3<f32> = vec3<f32>(0.14, 0.13, 0.18);  // dim cold bounce
const HORIZON_FILL: vec3<f32> = vec3<f32>(0.24, 0.22, 0.28);  // muted purple-grey

// ─── Ocean ────────────────────────────────────────────────────
const OCEAN_HALF_DIST: f32 = 220.0;
const OCEAN_SHALLOW_COLOR: vec3<f32> = vec3<f32>(0.15, 0.20, 0.24);  // murky teal
const OCEAN_DEEP_COLOR: vec3<f32> = vec3<f32>(0.06, 0.08, 0.14);     // ink-black deep
const FOAM_COLOR: vec3<f32> = vec3<f32>(0.50, 0.48, 0.52);           // pale grey foam
const SHORE_DEPTH: f32 = 2.5;
const FOAM_SHORE_DEPTH: f32 = 1.2;

// ─── Vertex I/O ───────────────────────────────────────────────

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

// ═══════════════════════════════════════════════════════════════
//  Utility functions
// ═══════════════════════════════════════════════════════════════

// GPU-friendly hash — returns 0..1 from a 2D position
fn hash2d(p: vec2<f32>) -> f32 {
    var n = dot(p, vec2<f32>(127.1, 311.7));
    n = sin(n) * 43758.5453;
    return fract(n);
}

// Smooth 2D value noise — used for cloud shadows and detail
fn value_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    // Smooth interpolation curve
    let u = f * f * (3.0 - 2.0 * f);

    let a = hash2d(i + vec2<f32>(0.0, 0.0));
    let b = hash2d(i + vec2<f32>(1.0, 0.0));
    let c = hash2d(i + vec2<f32>(0.0, 1.0));
    let d = hash2d(i + vec2<f32>(1.0, 1.0));

    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// FBM (fractal Brownian motion) — layered noise for clouds
fn fbm(p: vec2<f32>, octaves: i32) -> f32 {
    var sum = 0.0;
    var amp = 0.5;
    var freq = 1.0;
    var pos = p;

    for (var i = 0; i < octaves; i = i + 1) {
        sum += value_noise(pos * freq) * amp;
        freq *= 2.0;
        amp *= 0.5;
        // Rotate each octave slightly to break axis alignment
        pos = vec2<f32>(pos.x * 0.8 - pos.y * 0.6, pos.x * 0.6 + pos.y * 0.8);
    }
    return sum;
}

// ─── Cloud constants ─────────────────────────────────────────
const CLOUD_ALTITUDE: f32 = 240.0;   // lower cloud ceiling — oppressive
const CLOUD_SCALE: f32 = 0.0010;     // bigger, heavier cloud masses
const CLOUD_COVERAGE: f32 = 0.55;    // heavier overcast — darker world
const CLOUD_BRIGHTNESS: f32 = 0.38;  // dark brooding clouds
const CLOUD_EDGE_BRIGHTNESS: f32 = 0.48; // thin edges still catch faint light

// ═══════════════════════════════════════════════════════════════
//  Clouds — flat FBM noise on a high-altitude plane
// ═══════════════════════════════════════════════════════════════

fn sample_clouds(ray_origin: vec3<f32>, view_dir: vec3<f32>, light_dir: vec3<f32>) -> vec4<f32> {
    // Ray-plane intersection: find where view ray hits cloud plane
    if view_dir.y <= 0.01 {
        // Looking below or at horizon — no clouds
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    let t = (CLOUD_ALTITUDE - ray_origin.y) / view_dir.y;
    if t < 0.0 || t > 2000.0 {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    let hit = ray_origin + view_dir * t;
    let cloud_uv = vec2<f32>(hit.x, hit.z) * CLOUD_SCALE;

    // Multi-octave cloud noise
    let n1 = fbm(cloud_uv, 4);
    let n2 = fbm(cloud_uv * 2.2 + vec2<f32>(3.7, 1.2), 3) * 0.4;
    let raw_density = n1 + n2;

    // Coverage threshold — shapes clouds from noise
    let density = smoothstep(CLOUD_COVERAGE, CLOUD_COVERAGE + 0.28, raw_density);

    if density < 0.001 {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    // Cloud lighting: brighter edges (thin), darker cores (thick)
    let edge_factor = 1.0 - smoothstep(0.0, 0.6, density);
    let base_bright = mix(CLOUD_BRIGHTNESS, CLOUD_EDGE_BRIGHTNESS, edge_factor);

    // Sun-side brightening — faint cold light piercing the overcast
    let sun_dot = clamp(dot(view_dir, light_dir), 0.0, 1.0);
    let sun_glow = pow(sun_dot, 4.0) * 0.08;  // subtler, tighter

    let cloud_color = vec3<f32>(
        base_bright + sun_glow * 0.6,
        base_bright + sun_glow * 0.7,
        base_bright + sun_glow * 1.0,   // cool-tinted light bleed
    );

    // Fade out near horizon — clouds flatten and dissolve into haze
    let horizon_fade = smoothstep(0.02, 0.15, view_dir.y);
    // Also fade at extreme distance
    let dist_fade = 1.0 - smoothstep(800.0, 1800.0, t);

    let alpha = density * horizon_fade * dist_fade * 0.85;

    return vec4<f32>(cloud_color, alpha);
}

// ═══════════════════════════════════════════════════════════════
//  Sky — overcast with clouds and subtle sun presence
// ═══════════════════════════════════════════════════════════════

fn compute_sky_base(view_dir: vec3<f32>, light_dir: vec3<f32>) -> vec3<f32> {
    let up_factor = clamp(view_dir.y, -1.0, 1.0);

    let horizon_blend = 1.0 - pow(max(up_factor, 0.0), 0.5);
    let nadir_blend = pow(max(-up_factor, 0.0), 0.8);

    let sun_dot = clamp(dot(view_dir, light_dir), 0.0, 1.0);
    let sun_core = pow(sun_dot, 16.0) * 0.20;
    let sun_halo = pow(sun_dot, 3.0) * 0.12;
    let sun_glow = sun_core + sun_halo;

    let horizon_color = mix(weather.sky_horizon.xyz, weather.sky_horizon_sun.xyz, sun_glow);

    var sky = mix(weather.sky_zenith.xyz, horizon_color, horizon_blend);
    sky = mix(sky, weather.sky_nadir.xyz, nadir_blend);

    return sky;
}

fn compute_sky(view_dir: vec3<f32>, light_dir: vec3<f32>) -> vec3<f32> {
    let base_sky = compute_sky_base(view_dir, light_dir);

    // Sample clouds — blend over the base sky
    let clouds = sample_clouds(camera.camera_pos, view_dir, light_dir);
    var sky = mix(base_sky, clouds.xyz, clouds.w);

    // Weather: darken and desaturate sky
    sky = sky * (1.0 - weather.sky_darken);
    return sky;
}

// ═══════════════════════════════════════════════════════════════
//  Cloud shadows — large-scale light/dark patches on terrain
// ═══════════════════════════════════════════════════════════════

fn cloud_shadow(world_pos: vec3<f32>) -> f32 {
    // Large-scale noise sampled in world XZ — simulates cloud cover
    let cloud_uv = vec2<f32>(world_pos.x, world_pos.z) * 0.006;
    let cloud = fbm(cloud_uv, 3);

    // Map to shadow: 0.7 (shadowed) to 1.0 (lit)
    // Biased bright — overcast means most areas are similar,
    // but patches of relative brightness break through
    return mix(0.72, 1.0, cloud);
}

// ═══════════════════════════════════════════════════════════════
//  Fog — dense, height-aware, sun-tinted
// ═══════════════════════════════════════════════════════════════

fn compute_fog(
    dist: f32,
    frag_height: f32,
    cam_height: f32,
    view_dir: vec3<f32>,
    light_dir: vec3<f32>,
) -> vec4<f32> {
    // Distance fog — exponential-squared, thickened by weather
    let fog_density = 0.012 * weather.fog_mult;
    let dist_factor = max(dist - FOG_START, 0.0) * fog_density;
    let dist_fog = 1.0 - exp(-dist_factor * dist_factor);

    // Height fog — exponentially thicker below sea level (valley mist)
    let min_height = min(frag_height, cam_height);
    let height_diff = SEA_LEVEL + 5.0 - min_height;
    let height_fog = clamp(1.0 - exp(-max(height_diff, 0.0) * FOG_HEIGHT_FALLOFF), 0.0, 0.6);

    let total_fog = clamp(dist_fog + height_fog * dist_fog, 0.0, 1.0);

    // Fog color: base sky — cold, muted scatter
    let sky_fog = compute_sky_base(view_dir, light_dir);
    let sun_dot = clamp(dot(view_dir, light_dir), 0.0, 1.0);
    let sun_scatter = pow(sun_dot, 5.0) * 0.08;  // dimmer, tighter
    let cold_scatter = vec3<f32>(0.35, 0.38, 0.48);  // blue-grey scatter
    let fog_color = sky_fog + cold_scatter * sun_scatter;

    return vec4<f32>(fog_color, total_fog);
}

// ═══════════════════════════════════════════════════════════════
//  Ground mist — wispy fog pooling in valleys and low areas
// ═══════════════════════════════════════════════════════════════

fn ground_mist(world_pos: vec3<f32>, cam_pos: vec3<f32>, time: f32) -> f32 {
    // Only active below a certain altitude (sea_level + ~15)
    let mist_ceiling = SEA_LEVEL + 15.0;
    let height = world_pos.y;
    if (height > mist_ceiling) { return 0.0; }

    // Height falloff — thickest near sea level, fades upward
    let height_factor_raw = 1.0 - clamp((height - SEA_LEVEL) / (mist_ceiling - SEA_LEVEL), 0.0, 1.0);
    let height_factor = height_factor_raw * height_factor_raw;  // quadratic falloff

    // Animated wispy noise — slowly drifting patches
    let drift = vec2<f32>(time * 0.3, time * 0.15);
    let mist_uv = vec2<f32>(world_pos.x, world_pos.z) * 0.015 + drift;
    let noise1 = fbm(mist_uv, 3);
    let noise2 = value_noise(mist_uv * 3.0 - drift * 0.5) * 0.3;
    let mist_density = max(noise1 + noise2 - 0.3, 0.0);  // threshold to create patches

    // Distance fade — mist is more visible at medium distance, not right at camera
    let mist_dist = length(world_pos.xz - cam_pos.xz);
    let dist_fade = smoothstep(3.0, 15.0, mist_dist) * smoothstep(80.0, 40.0, mist_dist);

    return mist_density * height_factor * dist_fade * 0.6;
}

// ═══════════════════════════════════════════════════════════════
//  Aerial perspective — distant objects shift color
// ═══════════════════════════════════════════════════════════════

fn aerial_perspective(color: vec3<f32>, dist: f32, frag_height: f32) -> vec3<f32> {
    // Distant objects lose saturation and shift toward the atmosphere color
    let t = clamp(dist / FOG_END, 0.0, 1.0);
    let t_sq = t * t;

    // Desaturate with distance
    let luma = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    let desat = mix(color, vec3<f32>(luma), t_sq * 0.4);

    // Tint toward sky — cold atmospheric haze swallows distant terrain
    let altitude_factor = clamp((frag_height - SEA_LEVEL) / 40.0, 0.0, 1.0);
    let tint_strength = t_sq * (1.0 - altitude_factor * 0.4) * 0.30;
    let atmo_tint = vec3<f32>(0.30, 0.28, 0.35);  // cold purple-grey

    return mix(desat, atmo_tint, tint_strength);
}

// ═══════════════════════════════════════════════════════════════
//  Shadow sampling — PCF 3x3 from directional light
// ═══════════════════════════════════════════════════════════════

const SHADOW_MAP_TEXEL: f32 = 1.0 / 2048.0;
const SHADOW_BIAS: f32 = 0.003;
const SHADOW_CASCADE_HALF: f32 = 80.0;

fn sample_shadow(world_pos: vec3<f32>) -> f32 {
    let light_clip = shadow_uni.light_vp * vec4<f32>(world_pos, 1.0);
    let light_ndc = light_clip.xyz / light_clip.w;

    // NDC → UV (flip Y for texture coordinates)
    let uv = vec2<f32>(
        light_ndc.x * 0.5 + 0.5,
        light_ndc.y * -0.5 + 0.5,
    );
    let depth = light_ndc.z;

    // Outside shadow map — treat as fully lit
    if uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 || depth > 1.0 {
        return 1.0;
    }

    // Poisson disc offsets for organic soft shadows (12 taps)
    let poisson = array<vec2<f32>, 12>(
        vec2<f32>(-0.326, -0.406),
        vec2<f32>(-0.840, -0.074),
        vec2<f32>(-0.696,  0.457),
        vec2<f32>(-0.203,  0.621),
        vec2<f32>( 0.962, -0.195),
        vec2<f32>( 0.473, -0.480),
        vec2<f32>( 0.519,  0.767),
        vec2<f32>( 0.185, -0.893),
        vec2<f32>( 0.507,  0.064),
        vec2<f32>(-0.321,  0.932),
        vec2<f32>(-0.698, -0.680),
        vec2<f32>( 0.053,  0.326),
    );

    // Normal-offset bias: steeper slopes get more bias
    let slope_bias = SHADOW_BIAS;
    let spread = SHADOW_MAP_TEXEL * 1.8;

    var result = 0.0;
    for (var i = 0; i < 12; i++) {
        let offset = poisson[i] * spread;
        result += textureSampleCompare(
            shadow_map, shadow_sampler,
            uv + offset,
            depth - slope_bias
        );
    }
    result = result / 12.0;

    // Fade shadows at cascade edges to avoid hard cutoff
    let edge_dist = max(
        max(abs(light_ndc.x), abs(light_ndc.y)),
        0.0
    );
    let edge_fade = smoothstep(0.85, 1.0, edge_dist);
    return mix(result, 1.0, edge_fade);
}

// ═══════════════════════════════════════════════════════════════
//  Tone mapping — filmic with better shoulder/toe
// ═══════════════════════════════════════════════════════════════

fn tonemap_aces(x: vec3<f32>) -> vec3<f32> {
    // Simplified ACES fit by Krzysztof Narkowicz, tuned for low-key look
    let a = x * (x * 2.51 + 0.03);
    let b = x * (x * 2.43 + 0.59) + 0.14;
    return clamp(a / b, vec3<f32>(0.0), vec3<f32>(1.0));
}

// ═══════════════════════════════════════════════════════════════
//  Color grading — dark fantasy: cold shadows, silver highlights
// ═══════════════════════════════════════════════════════════════

fn color_grade(color: vec3<f32>) -> vec3<f32> {
    var c = color;

    // Lift (shadows): deep blue-purple — ominous, ancient
    let luma = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
    let shadow_weight = 1.0 - smoothstep(0.0, 0.35, luma);
    let shadow_tint = vec3<f32>(0.88, 0.86, 1.08);  // strong blue-purple push
    c = c * mix(vec3<f32>(1.0), shadow_tint, shadow_weight * 0.6);

    // Gain (highlights): cold silver — no warmth, pale moonlit
    let highlight_weight = smoothstep(0.35, 0.75, luma);
    let highlight_tint = vec3<f32>(0.95, 0.97, 1.06);  // blue-silver
    c = c * mix(vec3<f32>(1.0), highlight_tint, highlight_weight * 0.45);

    // Heavy desaturation — drained, bleak world
    let final_luma = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
    c = mix(vec3<f32>(final_luma), c, 0.68);

    // Crush blacks slightly — deeper shadows, more contrast
    c = max(c - 0.02, vec3<f32>(0.0));

    return c;
}

// ═══════════════════════════════════════════════════════════════
//  Terrain lighting — the main fragment shader
// ═══════════════════════════════════════════════════════════════

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var pos = in.position;

    // Wind sway for vegetation (grass, leaves, flowers)
    // Detect vegetation: green-dominant color + non-upward normal (blade geometry)
    let is_green = in.color.y > in.color.x && in.color.y > in.color.z;
    let not_flat = abs(in.normal.y) < 0.85;
    if (is_green && not_flat && weather.wind_strength > 0.0) {
        // Sway amount based on height above ground (higher = more sway)
        // Use normal.y as a proxy: blade tips have lower normal.y
        let sway_factor = (1.0 - abs(in.normal.y)) * weather.wind_strength;

        // Multi-frequency wind: gusty, organic motion
        let t = weather.time;
        let p = pos.xz * 0.15;
        let gust1 = sin(p.x * 1.3 + t * 1.8) * cos(p.y * 0.9 + t * 1.2);
        let gust2 = sin(p.x * 2.7 + t * 3.1 + 1.5) * 0.3;
        let gust = (gust1 + gust2) * 0.5 + 0.5; // 0..1

        let wind_dir = vec2<f32>(weather.wind_x, weather.wind_z);
        let displacement = wind_dir * sway_factor * gust * 0.35;
        pos.x += displacement.x;
        pos.z += displacement.y;
    }

    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(pos, 1.0);
    out.color = in.color;
    out.normal = in.normal;
    out.world_pos = pos;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);
    let light_dir = sun.direction;
    let view_vec = in.world_pos - camera.camera_pos;
    let dist = length(view_vec);
    let view_dir = view_vec / max(dist, 0.001);

    // ─── Diffuse: wrapped lambert with warm/cool shift ─────
    let raw_ndotl = dot(n, light_dir);
    let ndotl = raw_ndotl * 0.5 + 0.5; // half-lambert

    // Dim filtered light — cold when lit, deep purple in shadow
    let filtered_light = sun.color * vec3<f32>(0.82, 0.80, 0.85);
    let deep_shadow = SHADOW_COLOR * vec3<f32>(0.90, 0.88, 1.05);
    let light_color = mix(deep_shadow, filtered_light, ndotl);
    let diffuse = light_color * 0.45;  // less overall light

    // ─── Tri-directional ambient ───────────────────────────
    // Sky hemisphere (top), ground bounce (bottom), horizon fill (sides)
    let sky_weight = clamp(n.y * 0.5 + 0.5, 0.0, 1.0);
    let horizon_weight = 1.0 - abs(n.y);

    let sky_ambient = vec3<f32>(0.32, 0.30, 0.38) * sun.ambient;  // cold sky light
    let ground_ambient = BOUNCE_COLOR * sun.ambient * 0.40;
    let horiz_ambient = HORIZON_FILL * sun.ambient * 0.50;

    let ambient = mix(ground_ambient, sky_ambient, sky_weight)
                + horiz_ambient * horizon_weight;

    // ─── Shadows (real + cloud) ───────────────────────────
    let real_shadow = sample_shadow(in.world_pos);
    // Soften shadow — never go fully black, keep 30% diffuse in shadow
    let shadow = mix(0.45, 1.0, real_shadow) * cloud_shadow(in.world_pos);

    // ─── Combine base lighting ─────────────────────────────
    var lit_color = in.color * (ambient + diffuse * shadow);

    // ─── Subtle top-face brightness boost ──────────────────
    // Upward-facing surfaces catch more overcast sky light
    let top_boost = max(n.y, 0.0) * 0.06;
    lit_color = lit_color + lit_color * top_boost;

    // ─── Aerial perspective (before fog) ───────────────────
    lit_color = aerial_perspective(lit_color, dist, in.world_pos.y);

    // ─── Fog ───────────────────────────────────────────────
    let fog = compute_fog(dist, in.world_pos.y, camera.camera_pos.y, view_dir, light_dir);
    lit_color = mix(lit_color, fog.xyz, fog.w);

    // ─── Ground mist — wispy fog pooling in valleys ────────
    let mist = ground_mist(in.world_pos, camera.camera_pos, weather.time);
    let mist_color = vec3<f32>(0.22, 0.21, 0.26);  // cold purple-grey mist
    lit_color = mix(lit_color, mist_color, mist);

    // ─── Tone mapping + color grading ──────────────────────
    lit_color = tonemap_aces(lit_color * 0.90);
    lit_color = color_grade(lit_color);

    return vec4<f32>(lit_color, 1.0);
}

// ═══════════════════════════════════════════════════════════════
//  Ocean fragment shader
// ═══════════════════════════════════════════════════════════════

// ─── Wave displacement (shared between vertex + fragment) ────
fn wave_height(xz: vec2<f32>, time: f32) -> f32 {
    let t = time * 0.4;
    // Large slow swell
    let w1 = sin(xz.x * 0.018 + t * 0.7) * cos(xz.y * 0.012 + t * 0.5) * 0.45;
    // Medium chop
    let w2 = sin(xz.x * 0.07 + t * 1.3 + xz.y * 0.03) * 0.18;
    let w3 = cos(xz.y * 0.065 + t * 1.1 + xz.x * 0.02) * 0.15;
    // Fine ripple
    let w4 = sin(xz.x * 0.2 + t * 2.5 + xz.y * 0.12) * 0.06;
    return w1 + w2 + w3 + w4;
}

fn wave_normal(pos: vec3<f32>, time: f32) -> vec3<f32> {
    let eps = 0.5;
    let h  = wave_height(pos.xz, time);
    let hx = wave_height(pos.xz + vec2<f32>(eps, 0.0), time);
    let hz = wave_height(pos.xz + vec2<f32>(0.0, eps), time);
    let dx = (hx - h) / eps;
    let dz = (hz - h) / eps;
    return normalize(vec3<f32>(-dx, 1.0, -dz));
}

// ─── Ocean vertex shader — displaces Y with waves ───────────
@vertex
fn vs_ocean(in: VertexInput) -> VertexOutput {
    var pos = in.position;
    pos.y += wave_height(pos.xz, weather.time);

    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(pos, 1.0);
    out.color = in.color;
    out.normal = in.normal; // will be recalculated in fragment
    out.world_pos = pos;
    return out;
}

// ─── Linearize depth (wgpu perspective_rh, depth 0..1) ──────
fn linearize_depth(d: f32) -> f32 {
    let near = 0.1;
    let far = 1000.0;
    return near * far / (far - d * (far - near));
}

@fragment
fn fs_ocean(in: VertexOutput) -> @location(0) vec4<f32> {
    // ─── Animated wave normals (from displaced surface) ─────
    let n = wave_normal(in.world_pos, weather.time);
    let light_dir = sun.direction;
    let view_vec = in.world_pos - camera.camera_pos;
    let dist = length(view_vec);
    let view_dir = view_vec / max(dist, 0.001);

    // ─── Water depth from terrain depth buffer ──────────────
    let screen_uv = in.clip_position.xy / vec2<f32>(textureDimensions(ocean_depth_tex));
    let terrain_depth_raw = textureSampleLevel(ocean_depth_tex, ocean_depth_sampler, screen_uv, 0);
    let terrain_depth = linearize_depth(terrain_depth_raw);
    let water_depth_linear = linearize_depth(in.clip_position.z);
    let depth_diff = terrain_depth - water_depth_linear; // world-space depth under water

    // ─── Shoreline edge blend — soft alpha near terrain ─────
    let shore_alpha = clamp(depth_diff / SHORE_DEPTH, 0.0, 1.0);
    // Smooth step for softer transition
    let alpha = shore_alpha * shore_alpha * (3.0 - 2.0 * shore_alpha);

    // ─── Fresnel — more reflective at glancing angles ───────
    let ndotv = clamp(dot(n, -view_dir), 0.0, 1.0);
    let fresnel = 0.04 + (1.0 - 0.04) * pow(1.0 - ndotv, 5.0);

    // ─── Base water color — depth-based shallow→deep ────────
    let color_depth_t = clamp(depth_diff / 8.0, 0.0, 1.0);
    let water_color = mix(OCEAN_SHALLOW_COLOR, OCEAN_DEEP_COLOR, color_depth_t);

    // ─── Diffuse — dim, the water absorbs light ─────────────
    let ndotl = dot(n, light_dir) * 0.5 + 0.5;
    let cold_light = sun.color * vec3<f32>(0.70, 0.72, 0.80);
    let diffuse = mix(SHADOW_COLOR * 0.7, cold_light, ndotl) * 0.28;

    let sky_ambient = vec3<f32>(0.22, 0.22, 0.28) * sun.ambient;
    let ambient = sky_ambient * 0.6;

    var lit_color = water_color * (ambient + diffuse);

    // ─── Sky reflection — blended by fresnel ────────────────
    let reflect_sky = compute_sky(reflect(view_dir, n), light_dir);
    lit_color = mix(lit_color, reflect_sky * 0.6, fresnel * 0.7);

    // ─── Sun specular — scattered, broad sun path on water ──
    let reflect_dir = reflect(view_dir, n);
    let spec_dot = clamp(dot(reflect_dir, light_dir), 0.0, 1.0);
    let spec_tight = pow(spec_dot, 48.0) * 0.35;
    let spec_broad = pow(spec_dot, 6.0) * 0.10;
    let specular = spec_tight + spec_broad;
    let spec_color = vec3<f32>(0.45, 0.48, 0.55);  // cold silver specular
    lit_color = lit_color + spec_color * specular;

    // ─── Foam — wave crests + shoreline ─────────────────────
    let wave_h = wave_height(in.world_pos.xz, weather.time);
    // Crest foam: foam appears on wave peaks
    let crest_foam = clamp((wave_h - 0.25) * 2.5, 0.0, 1.0);
    // Shoreline foam: band of foam near terrain
    let shore_foam_raw = clamp(1.0 - depth_diff / FOAM_SHORE_DEPTH, 0.0, 1.0);
    // Animated foam pattern — noisy, organic
    let foam_noise = value_noise(in.world_pos.xz * 0.8 + vec2<f32>(weather.time * 0.3, weather.time * 0.2));
    let foam_noise2 = value_noise(in.world_pos.xz * 2.5 - vec2<f32>(weather.time * 0.5));
    let shore_foam = shore_foam_raw * shore_foam_raw * step(0.35, foam_noise);
    let total_foam = clamp(crest_foam * foam_noise2 + shore_foam, 0.0, 1.0);
    lit_color = mix(lit_color, FOAM_COLOR * (ambient + diffuse * 1.5), total_foam * 0.75);

    // ─── Shadows on water too (real + cloud) ────────────────
    let real_shadow_w = sample_shadow(in.world_pos);
    let shadow = mix(0.45, 1.0, real_shadow_w) * cloud_shadow(in.world_pos);
    lit_color = lit_color * mix(0.85, 1.0, shadow);

    // ─── Fog ────────────────────────────────────────────────
    let fog = compute_fog(dist, in.world_pos.y, camera.camera_pos.y, view_dir, light_dir);
    lit_color = mix(lit_color, fog.xyz, fog.w);

    // ─── Ground mist — wispy fog pooling over water ────────
    let mist = ground_mist(in.world_pos, camera.camera_pos, weather.time);
    let mist_color = vec3<f32>(0.18, 0.19, 0.26);  // cold blue-grey mist
    lit_color = mix(lit_color, mist_color, mist);

    // ─── Tone mapping + grading — dark, desaturated ocean ───
    lit_color = tonemap_aces(lit_color * 0.78);
    let luma = dot(lit_color, vec3<f32>(0.2126, 0.7152, 0.0722));
    lit_color = mix(vec3<f32>(luma), lit_color, 0.62);
    lit_color = lit_color * vec3<f32>(0.92, 0.95, 1.08);  // push cold blue

    return vec4<f32>(lit_color, alpha);
}

// ═══════════════════════════════════════════════════════════════
//  Sun disc — pale smudge through overcast
// ═══════════════════════════════════════════════════════════════

@fragment
fn fs_sun(in: VertexOutput) -> @location(0) vec4<f32> {
    let view_vec = in.world_pos - camera.camera_pos;
    let view_dir = normalize(view_vec);
    let light_dir = sun.direction;

    // Sky with clouds behind the sun
    let sky = compute_sky(view_dir, light_dir);

    // Sun disc — pale, sickly glow barely visible through thick clouds
    let sun_bright = vec3<f32>(0.65, 0.62, 0.60);  // desaturated, cold
    let sun_pos = sun.direction * 800.0;
    let to_center = length(in.world_pos - sun_pos) / 22.0;
    let disc = clamp(1.0 - to_center, 0.0, 1.0);

    // Dimmer core, wider diffuse glow — obscured sun
    let core = pow(disc, 5.0) * 0.45;
    let glow = pow(disc, 1.8) * 0.15;
    let sun_blend = core + glow;

    // Clouds partially occlude the sun — thicker clouds dim it more
    let clouds = sample_clouds(camera.camera_pos, view_dir, light_dir);
    let cloud_occlusion = 1.0 - clouds.w * 0.6;

    var color = mix(sky, sun_bright, sun_blend * cloud_occlusion);
    color = tonemap_aces(color * 0.95);
    color = color_grade(color);

    return vec4<f32>(color, 1.0);
}

// ═══════════════════════════════════════════════════════════════
//  Sky dome — fullscreen sky with clouds
//  Renders at depth=1.0 (behind everything)
// ═══════════════════════════════════════════════════════════════

struct SkyVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_dir: vec3<f32>,
};

@vertex
fn vs_sky(@builtin(vertex_index) vertex_index: u32) -> SkyVertexOutput {
    // Fullscreen triangle (3 verts covering the screen)
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let pos = positions[vertex_index];

    var out: SkyVertexOutput;
    // Place at max depth (z=1.0 in NDC, w=1.0)
    out.clip_position = vec4<f32>(pos, 1.0, 1.0);

    // Reconstruct world-space view direction from clip position
    // Inverse of view_proj * world_pos, but we only need direction
    let inv_vp = camera.inv_view_proj;
    let world_far = inv_vp * vec4<f32>(pos, 1.0, 1.0);
    let world_near = inv_vp * vec4<f32>(pos, -1.0, 1.0);
    let far_pos = world_far.xyz / world_far.w;
    let near_pos = world_near.xyz / world_near.w;
    out.world_dir = normalize(far_pos - near_pos);

    return out;
}

@fragment
fn fs_sky(in: SkyVertexOutput) -> @location(0) vec4<f32> {
    let view_dir = normalize(in.world_dir);
    let light_dir = sun.direction;

    var color = compute_sky(view_dir, light_dir);

    // Tone mapping + color grading (same as world)
    color = tonemap_aces(color * 0.95);
    color = color_grade(color);

    return vec4<f32>(color, 1.0);
}
