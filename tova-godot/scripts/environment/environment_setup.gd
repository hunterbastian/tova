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

# ---------------------------------------------------------------------------
# WorldEnvironment
# ---------------------------------------------------------------------------
func _setup_world_environment() -> void:
	var sky_material := ProceduralSkyMaterial.new()
	sky_material.sky_top_color = Color("#5a7fa0")
	sky_material.sky_horizon_color = Color("#e8c090")
	sky_material.ground_bottom_color = Color("#8a7060")
	sky_material.ground_horizon_color = Color("#d8c8b8")

	var sky := Sky.new()
	sky.sky_material = sky_material

	var env := Environment.new()
	env.background_mode = Environment.BG_SKY
	env.sky = sky
	env.fog_enabled = true
	env.fog_mode = Environment.FOG_MODE_EXPONENTIAL
	env.fog_density = 0.012
	env.fog_light_color = Color("#d8c8b8")
	env.tonemap_exposure = 1.35
	env.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	env.ambient_light_color = Color("#c8a878")
	env.ambient_light_energy = 1.1

	_world_env = WorldEnvironment.new()
	_world_env.environment = env
	add_child(_world_env)

# ---------------------------------------------------------------------------
# Sun (DirectionalLight3D)
# ---------------------------------------------------------------------------
func _setup_sun() -> void:
	_sun = DirectionalLight3D.new()
	_sun.light_color = Color("#f0c890")
	_sun.light_energy = 2.0
	_sun.shadow_enabled = true
	_sun.directional_shadow_max_distance = 280.0
	_sun.directional_shadow_mode = DirectionalLight3D.SHADOW_ORTHOGONAL
	add_child(_sun)
	_sun.look_at_from_position(Vector3(88.0, 132.0, -24.0), Vector3.ZERO)

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
# Public API
# ---------------------------------------------------------------------------
func update_spawn_light(ground_height: float) -> void:
	_spawn_fill_light.position = Vector3(4.0, ground_height + 7.5, 8.0)
