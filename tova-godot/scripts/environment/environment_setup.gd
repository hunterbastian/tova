extends Node3D

# ---------------------------------------------------------------------------
# Private references
# ---------------------------------------------------------------------------
var _world_env: WorldEnvironment
var _sun: DirectionalLight3D
var _spawn_fill_light: OmniLight3D
var _moon: MeshInstance3D

# ---------------------------------------------------------------------------
# _ready
# ---------------------------------------------------------------------------
func _ready() -> void:
	_setup_world_environment()
	_setup_sun()
	_setup_spawn_fill_light()
	_setup_moon()
	_setup_cloud_volumes()

# ---------------------------------------------------------------------------
# WorldEnvironment
# ---------------------------------------------------------------------------
func _setup_world_environment() -> void:
	var sky_material := ProceduralSkyMaterial.new()
	sky_material.sky_top_color = Color("#1a2a3a")
	sky_material.sky_horizon_color = Color("#4a3a30")
	sky_material.ground_bottom_color = Color("#0a0a08")
	sky_material.ground_horizon_color = Color("#2a2018")
	sky_material.sky_curve = 0.1
	sky_material.ground_curve = 0.1

	var sky := Sky.new()
	sky.sky_material = sky_material

	var env := Environment.new()
	env.background_mode = Environment.BG_SKY
	env.sky = sky
	env.fog_enabled = true
	env.fog_mode = Environment.FOG_MODE_EXPONENTIAL
	env.fog_density = 0.012
	env.fog_light_color = Color("#3a3228")
	env.volumetric_fog_enabled = true
	env.volumetric_fog_density = 0.005
	env.volumetric_fog_albedo = Color("#2a2a28")
	env.volumetric_fog_emission = Color("#1a1a18")
	env.volumetric_fog_length = 200.0
	env.volumetric_fog_sky_affect = 0.05
	env.volumetric_fog_gi_inject = 0.0
	env.tonemap_exposure = 0.8
	env.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	env.ambient_light_color = Color("#4a4038")
	env.ambient_light_energy = 0.6

	_world_env = WorldEnvironment.new()
	_world_env.environment = env
	add_child(_world_env)

# ---------------------------------------------------------------------------
# Sun (DirectionalLight3D)
# ---------------------------------------------------------------------------
func _setup_sun() -> void:
	_sun = DirectionalLight3D.new()
	_sun.light_color = Color("#a08060")
	_sun.light_energy = 1.2
	_sun.shadow_enabled = true
	_sun.directional_shadow_max_distance = 280.0
	_sun.directional_shadow_mode = DirectionalLight3D.SHADOW_ORTHOGONAL
	add_child(_sun)
	_sun.look_at_from_position(Vector3(88.0, 200.0, -24.0), Vector3.ZERO)

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
# Moon (MeshInstance3D)
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
# Cloud volumes — FogVolume nodes placed high for cloud layer
# ---------------------------------------------------------------------------
func _setup_cloud_volumes() -> void:
	var cloud_mat := FogMaterial.new()
	cloud_mat.density = 0.15
	cloud_mat.albedo = Color("#4a4a50")

	# Several cloud patches at high altitude
	var rng := RandomNumberGenerator.new()
	rng.seed = 42
	for i in range(12):
		var fog_vol := FogVolume.new()
		fog_vol.shape = RenderingServer.FOG_VOLUME_SHAPE_ELLIPSOID
		fog_vol.size = Vector3(
			15.0 + rng.randf() * 25.0,
			1.5 + rng.randf() * 2.5,
			12.0 + rng.randf() * 20.0
		)
		fog_vol.position = Vector3(
			-120.0 + rng.randf() * 240.0,
			180.0 + rng.randf() * 80.0,
			-120.0 + rng.randf() * 240.0
		)
		fog_vol.material = cloud_mat
		add_child(fog_vol)

# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------
func update_spawn_light(ground_height: float) -> void:
	_spawn_fill_light.position = Vector3(4.0, ground_height + 7.5, 8.0)
