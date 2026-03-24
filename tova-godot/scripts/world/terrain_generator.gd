extends MeshInstance3D

# ---------------------------------------------------------------------------
# Terrain palette (from world.js lines 11-18)
# ---------------------------------------------------------------------------
const PALETTE_GRASS    := Color("#4a6a3a")
const PALETTE_SPAWN    := Color("#5a7a48")
const PALETTE_FOREST   := Color("#2a4a22")
const PALETTE_HIGHLAND := Color("#5a5a4a")
const PALETTE_SLOPE    := Color("#4a5a3a")
const PALETTE_DRY      := Color("#3a5a2a")

# ---------------------------------------------------------------------------
# Member variables
# ---------------------------------------------------------------------------
var _noise_broad:    FastNoiseLite
var _noise_hills:    FastNoiseLite
var _noise_ridge:    FastNoiseLite
var _noise_moisture: FastNoiseLite

var _offset_x:       float
var _offset_z:       float
var _ridge_offset:   float
var _forest_center:  Vector3
var _castle_center:  Vector3
var _mountain_peak:  Vector3
var _rng:            RandomNumberGenerator


# ---------------------------------------------------------------------------
# _ready — initialize noise instances
# ---------------------------------------------------------------------------
func _ready() -> void:
	_noise_broad = FastNoiseLite.new()
	_noise_broad.noise_type = FastNoiseLite.TYPE_PERLIN
	_noise_broad.frequency = 0.0143  # ≈ 1/70

	_noise_hills = FastNoiseLite.new()
	_noise_hills.noise_type = FastNoiseLite.TYPE_PERLIN
	_noise_hills.frequency = 0.0417  # ≈ 1/24

	_noise_ridge = FastNoiseLite.new()
	_noise_ridge.noise_type = FastNoiseLite.TYPE_PERLIN
	_noise_ridge.frequency = 0.0909  # ≈ 1/11

	_noise_moisture = FastNoiseLite.new()
	_noise_moisture.noise_type = FastNoiseLite.TYPE_PERLIN
	_noise_moisture.frequency = 1.0  # coords divided manually by 16.0 at call site


# ---------------------------------------------------------------------------
# _build_terrain_context — port of buildTerrainContext() (world.js 138-206)
# ---------------------------------------------------------------------------
func _build_terrain_context(seed_val: int) -> void:
	_rng = RandomNumberGenerator.new()
	_rng.seed = seed_val

	_offset_x    = _rng.randf() * 1000.0
	_offset_z    = _rng.randf() * 1000.0
	_ridge_offset = _rng.randf() * 800.0 + 200.0

	_forest_center  = Vector3(40.0 + _rng.randf() * 16.0, 0.0, 12.0 + _rng.randf() * 16.0)
	_castle_center  = Vector3(-14.0 - _rng.randf() * 10.0, 0.0, -44.0 - _rng.randf() * 10.0)
	_mountain_peak  = Vector3(28.0 + _rng.randf() * 18.0, 0.0, -94.0 - _rng.randf() * 18.0)

	# Sample height at each zone center
	_forest_center.y = sample_height(_forest_center.x, _forest_center.z)
	_castle_center.y = sample_height(_castle_center.x, _castle_center.z)
	_mountain_peak.y = sample_height(_mountain_peak.x, _mountain_peak.z)


# ---------------------------------------------------------------------------
# sample_height — core height function (world.js 148-199)
# ---------------------------------------------------------------------------
func sample_height(x: float, z: float) -> float:
	var broad := _noise_broad.get_noise_3d(x + _offset_x, 0.15, z + _offset_z) * 28.0
	var hills := _noise_hills.get_noise_3d(x + _offset_x, 0.32, z + _offset_z) * 16.0
	var ridge := _noise_ridge.get_noise_3d(x - _ridge_offset, 0.52, z + _ridge_offset * 0.35) * 8.0

	# Mountain peak lift
	var peak_distance := Vector2(x - _mountain_peak.x, z - _mountain_peak.z).length()
	var peak_lift_raw := maxf(0.0, 1.0 - peak_distance / 90.0)
	var peak_lift := _smootherstep(peak_lift_raw) * 50.0

	# Forest lift
	var forest_dist := Vector2(x - _forest_center.x, z - _forest_center.z).length()
	var forest_lift := maxf(0.0, 1.0 - forest_dist / 22.0) * 1.15

	# Castle lift
	var castle_dist_lift := Vector2(x - _castle_center.x, z - _castle_center.z).length()
	var castle_lift := maxf(0.0, 1.0 - castle_dist_lift / 18.0) * 1.8

	var height := 8.0 + broad + hills + ridge + peak_lift + forest_lift + castle_lift

	# Spawn flatten
	# Secondary noise calls use _noise_broad (freq 0.0143 ≈ 1/70).
	# world.js passes coords divided by 18; to replicate with freq=1/70 we
	# multiply raw coords by 70/18, so effective divisor becomes 18.
	var spawn_distance := Vector2(x, z).length()
	if spawn_distance < GameState.SPAWN_BLEND_RADIUS:
		var target := 8.6 + _noise_broad.get_noise_3d(
			(x + _offset_x) * (70.0 / 18.0), 0.12, (z + _offset_z) * (70.0 / 18.0)
		) * 0.16
		var blend := _smootherstep(clampf(spawn_distance / float(GameState.SPAWN_BLEND_RADIUS), 0.0, 1.0))
		height = lerpf(target, height, blend)
		if spawn_distance < GameState.SPAWN_RADIUS:
			height = target

	# Castle plateau (world.js: noise(x/30, ...))
	var castle_distance := Vector2(x - _castle_center.x, z - _castle_center.z).length()
	if castle_distance < 16.0:
		var plateau := 12.0 + _noise_broad.get_noise_3d(
			(x + _offset_x) * (70.0 / 30.0), 0.1, (z + _offset_z) * (70.0 / 30.0)
		) * 0.32
		var blend := _smootherstep(castle_distance / 16.0)
		height = lerpf(plateau, height, blend)

	# View lane from spawn to castle (world.js: noise(x/20, ...))
	var castle_length_sq := _castle_center.x * _castle_center.x + _castle_center.z * _castle_center.z
	if castle_length_sq > 0.0:
		var projection := (x * _castle_center.x + z * _castle_center.z) / castle_length_sq
		var clamped_proj := clampf(projection, 0.0, 1.0)
		var nearest_x := _castle_center.x * clamped_proj
		var nearest_z := _castle_center.z * clamped_proj
		var view_distance := Vector2(x - nearest_x, z - nearest_z).length()
		var in_view_lane := clamped_proj > 0.12 and clamped_proj < 0.88 and view_distance < 12.0

		if in_view_lane:
			var lane_target := (
				8.4
				+ _noise_broad.get_noise_3d(
					(x + _offset_x) * (70.0 / 20.0), 0.12, (z + _offset_z) * (70.0 / 20.0)
				) * 0.12
				+ clamped_proj * 0.6
			)
			var lane_blend := 1.0 - _smootherstep(view_distance / 12.0)
			height = lerpf(height, lane_target, lane_blend * 0.82)

	return height


# ---------------------------------------------------------------------------
# generate_terrain — build mesh, material, and collision
# ---------------------------------------------------------------------------
func generate_terrain(seed_val: int) -> void:
	_build_terrain_context(seed_val)

	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)

	var half := float(GameState.WORLD_SIZE) / 2.0
	var seg  := GameState.WORLD_SEGMENTS
	var step := float(GameState.WORLD_SIZE) / float(seg)

	# Pre-sample all vertex heights and colors
	var heights: Array[float] = []
	var colors:  Array[Color] = []
	heights.resize((seg + 1) * (seg + 1))
	colors.resize((seg + 1) * (seg + 1))

	for iz in range(seg + 1):
		for ix in range(seg + 1):
			var x := -half + ix * step
			var z := -half + iz * step
			var y := sample_height(x, z)
			var idx := iz * (seg + 1) + ix
			heights[idx] = y
			colors[idx]  = _get_vertex_color(x, z, y, seed_val)

	# Build triangles (two per quad, unindexed for flat shading)
	for iz in range(seg):
		for ix in range(seg):
			var i00 := iz * (seg + 1) + ix
			var i10 := i00 + 1
			var i01 := i00 + (seg + 1)
			var i11 := i01 + 1

			var x00 := -half + ix * step
			var z00 := -half + iz * step

			# Triangle 1
			_add_vertex(st, x00,         heights[i00], z00,         colors[i00])
			_add_vertex(st, x00 + step,  heights[i10], z00,         colors[i10])
			_add_vertex(st, x00,         heights[i01], z00 + step,  colors[i01])

			# Triangle 2
			_add_vertex(st, x00 + step,  heights[i10], z00,         colors[i10])
			_add_vertex(st, x00 + step,  heights[i11], z00 + step,  colors[i11])
			_add_vertex(st, x00,         heights[i01], z00 + step,  colors[i01])

	st.generate_normals()
	var arr_mesh := st.commit()
	self.mesh = arr_mesh

	# Material
	var mat := StandardMaterial3D.new()
	mat.vertex_color_use_as_albedo = true
	mat.roughness  = 0.96
	mat.metallic   = 0.02
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_PER_PIXEL
	self.material_override = mat

	# Shadow — terrain receives shadows, doesn't cast
	self.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF

	# Collision
	var static_body     := StaticBody3D.new()
	add_child(static_body)
	var collision_shape := CollisionShape3D.new()
	var concave_shape   := ConcavePolygonShape3D.new()
	concave_shape.set_faces(arr_mesh.get_faces())
	collision_shape.shape = concave_shape
	static_body.add_child(collision_shape)

	# Publish zone centers to GameState
	GameState.forest_center = _forest_center
	GameState.castle_center = _castle_center


# ---------------------------------------------------------------------------
# clear_terrain
# ---------------------------------------------------------------------------
func clear_terrain() -> void:
	mesh = null
	for child in get_children():
		child.queue_free()


# ---------------------------------------------------------------------------
# _get_vertex_color — port of buildTerrain() color logic (world.js 248-263)
# ---------------------------------------------------------------------------
func _get_vertex_color(x: float, z: float, y: float, seed_val: int) -> Color:
	var forest_dist := Vector2(x - _forest_center.x, z - _forest_center.z).length()
	var spawn_dist  := Vector2(x, z).length()
	var moisture    := _noise_moisture.get_noise_3d(
		(x + seed_val) / 16.0,
		1.4,
		(z - seed_val) / 16.0
	) * 0.5 + 0.5

	if spawn_dist < GameState.SPAWN_BLEND_RADIUS + 8:
		return PALETTE_SPAWN
	elif forest_dist < 26.0:
		return PALETTE_FOREST
	elif y > 20.0:
		return PALETTE_HIGHLAND
	elif y > 15.0:
		return PALETTE_SLOPE
	elif moisture < 0.32:
		return PALETTE_DRY
	else:
		return PALETTE_GRASS


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
func _add_vertex(st: SurfaceTool, x: float, y: float, z: float, color: Color) -> void:
	st.set_color(color)
	st.add_vertex(Vector3(x, y, z))


func _smootherstep(t: float) -> float:
	return t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
