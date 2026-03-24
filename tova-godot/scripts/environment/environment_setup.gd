extends Node3D

var _world_env: WorldEnvironment
var _sun: DirectionalLight3D
var _fill_light: DirectionalLight3D
var _spawn_fill_light: OmniLight3D
var _moon: MeshInstance3D

func _ready() -> void:
	_setup_world_environment()
	_setup_sun()
	_setup_fill_light()
	_setup_spawn_fill_light()
	_setup_moon()

# ---------------------------------------------------------------------------
# Environment — sky, GI, fog, post-processing
# ---------------------------------------------------------------------------
func _setup_world_environment() -> void:
	# Sky — warm late-afternoon palette
	var sky_material := ProceduralSkyMaterial.new()
	sky_material.sky_top_color = Color("#4a7aaa")
	sky_material.sky_horizon_color = Color("#c0a070")
	sky_material.ground_bottom_color = Color("#3a3530")
	sky_material.ground_horizon_color = Color("#7a6a58")
	sky_material.sky_curve = 0.08
	sky_material.ground_curve = 0.08
	sky_material.sky_energy_multiplier = 1.0
	sky_material.ground_energy_multiplier = 0.5

	var sky := Sky.new()
	sky.sky_material = sky_material
	sky.radiance_size = Sky.RADIANCE_SIZE_256

	var env := Environment.new()
	env.background_mode = Environment.BG_SKY
	env.sky = sky

	# ── Ambient light from sky (not flat color) ───────────────────────────
	env.ambient_light_source = Environment.AMBIENT_SOURCE_SKY
	env.ambient_light_energy = 0.8

	# ── Reflected light from sky ──────────────────────────────────────────
	env.reflected_light_source = Environment.REFLECTION_SOURCE_SKY

	# ── SDFGI — signed distance field global illumination ─────────────────
	# Best for open-world: indirect light bounces off terrain, trees, structures
	env.sdfgi_enabled = true
	env.sdfgi_use_occlusion = true
	env.sdfgi_cascade0_distance = 6.4
	env.sdfgi_max_distance = 200.0
	env.sdfgi_energy = 0.8
	env.sdfgi_bounce_feedback = 0.3
	env.sdfgi_normal_bias = 1.1

	# ── Fog — warm atmospheric depth ──────────────────────────────────────
	env.fog_enabled = true
	env.fog_mode = Environment.FOG_MODE_EXPONENTIAL
	env.fog_density = 0.008
	env.fog_light_color = Color("#9a8a70")
	env.fog_light_energy = 0.6
	env.fog_sun_scatter = 0.3
	env.fog_sky_affect = 0.4

	# ── Volumetric fog — atmospheric depth between trees ──────────────────
	env.volumetric_fog_enabled = true
	env.volumetric_fog_density = 0.005
	env.volumetric_fog_albedo = Color("#8a8070")
	env.volumetric_fog_emission = Color("#3a3530")
	env.volumetric_fog_emission_energy = 0.3
	env.volumetric_fog_length = 150.0
	env.volumetric_fog_sky_affect = 0.0

	# ── Tonemap ───────────────────────────────────────────────────────────
	# AgX not available in this Godot version — use ACES Fitted instead
	env.tonemap_mode = 3  # TONE_MAP_ACES
	env.tonemap_exposure = 1.1
	env.tonemap_white = 6.0

	# ── SSAO — contact shadows in crevices ────────────────────────────────
	env.ssao_enabled = true
	env.ssao_radius = 1.5
	env.ssao_intensity = 2.0
	env.ssao_power = 1.8
	env.ssao_light_affect = 0.3

	# ── SSIL — screen-space indirect light ────────────────────────────────
	env.ssil_enabled = true
	env.ssil_radius = 4.0
	env.ssil_intensity = 1.0
	env.ssil_normal_rejection = 1.0

	# ── Glow — soft bloom ─────────────────────────────────────────────────
	env.glow_enabled = true
	env.glow_intensity = 0.3
	env.glow_strength = 0.6
	env.glow_bloom = 0.05
	env.glow_blend_mode = Environment.GLOW_BLEND_MODE_SOFTLIGHT
	env.glow_hdr_threshold = 1.2

	# ── Adjustments — slight color grading ────────────────────────────────
	env.adjustment_enabled = true
	env.adjustment_brightness = 1.05
	env.adjustment_contrast = 1.15
	env.adjustment_saturation = 1.15

	_world_env = WorldEnvironment.new()
	_world_env.environment = env
	add_child(_world_env)

# ---------------------------------------------------------------------------
# Sun — primary directional light with cascaded shadows
# ---------------------------------------------------------------------------
func _setup_sun() -> void:
	_sun = DirectionalLight3D.new()
	_sun.light_color = Color("#f0c888")
	_sun.light_energy = 2.0
	_sun.light_indirect_energy = 1.5
	# Shadow — 4 cascades for quality at all distances
	_sun.shadow_enabled = true
	_sun.directional_shadow_mode = DirectionalLight3D.SHADOW_PARALLEL_4_SPLITS
	_sun.directional_shadow_max_distance = 200.0
	_sun.directional_shadow_split_1 = 0.05
	_sun.directional_shadow_split_2 = 0.15
	_sun.directional_shadow_split_3 = 0.4
	_sun.directional_shadow_blend_splits = true
	_sun.shadow_bias = 0.03
	_sun.shadow_normal_bias = 2.0
	_sun.shadow_blur = 2.0

	add_child(_sun)
	# Late afternoon angle — sun low in the sky for long shadows
	_sun.look_at_from_position(Vector3(100.0, 160.0, -40.0), Vector3.ZERO)

# ---------------------------------------------------------------------------
# Fill light — secondary directional from opposite side (sky bounce)
# ---------------------------------------------------------------------------
func _setup_fill_light() -> void:
	_fill_light = DirectionalLight3D.new()
	_fill_light.light_color = Color("#7090b0")
	_fill_light.light_energy = 0.4
	_fill_light.light_indirect_energy = 0.5
	_fill_light.shadow_enabled = false
	add_child(_fill_light)
	# Opposite of sun — fills shadow side with cool sky bounce
	_fill_light.look_at_from_position(Vector3(-80.0, 100.0, 60.0), Vector3.ZERO)

# ---------------------------------------------------------------------------
# Spawn fill light (OmniLight3D)
# ---------------------------------------------------------------------------
func _setup_spawn_fill_light() -> void:
	_spawn_fill_light = OmniLight3D.new()
	_spawn_fill_light.light_color = Color("#e0c898")
	_spawn_fill_light.light_energy = 1.6
	_spawn_fill_light.omni_range = 52.0
	_spawn_fill_light.omni_attenuation = 2.0
	_spawn_fill_light.position = Vector3(4.0, 16.0, 8.0)
	add_child(_spawn_fill_light)

# ---------------------------------------------------------------------------
# Moon
# ---------------------------------------------------------------------------
func _setup_moon() -> void:
	var sphere := SphereMesh.new()
	sphere.radius = 7.0
	sphere.height = 14.0
	sphere.rings = 24
	sphere.radial_segments = 24

	var mat := StandardMaterial3D.new()
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.albedo_color = Color("#e8d8c0")

	_moon = MeshInstance3D.new()
	_moon.mesh = sphere
	_moon.material_override = mat
	_moon.position = Vector3(-110.0, 92.0, -210.0)
	add_child(_moon)

# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------
func update_spawn_light(ground_height: float) -> void:
	_spawn_fill_light.position = Vector3(4.0, ground_height + 7.5, 8.0)
