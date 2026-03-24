extends MeshInstance3D

# ---------------------------------------------------------------------------
# Terrain palette (from world.js lines 11-18)
# ---------------------------------------------------------------------------
const PALETTE_GRASS    := Color("#5a8a48")
const PALETTE_SPAWN    := Color("#6a9a56")
const PALETTE_FOREST   := Color("#3a6a2a")
const PALETTE_HIGHLAND := Color("#6a6a5a")
const PALETTE_SLOPE    := Color("#4a7a3a")
const PALETTE_DRY      := Color("#4a6a32")
const PALETTE_ROCK     := Color("#7a7a72")
const PALETTE_SNOW     := Color("#e8e8f0")

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
var _mountain_peaks: Array[Dictionary]  # [{pos: Vector3, height: float, radius: float}]
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

	_forest_center  = Vector3(240.0 + _rng.randf() * 96.0, 0.0, 72.0 + _rng.randf() * 96.0)
	_castle_center  = Vector3(-84.0 - _rng.randf() * 60.0, 0.0, -264.0 - _rng.randf() * 60.0)

	# Generate 8-12 distinct mountain peaks around the map
	var half := float(GameState.WORLD_SIZE) / 2.0
	var peak_count := 8 + _rng.randi_range(0, 4)
	_mountain_peaks = []
	var attempts := 0
	while _mountain_peaks.size() < peak_count and attempts < 50:
		attempts += 1
		var px := _rng.randf_range(-half * 0.85, half * 0.85)
		var pz := _rng.randf_range(-half * 0.85, half * 0.85)
		# Keep peaks away from spawn
		if Vector2(px, pz).length() < 150.0:
			continue
		# Keep peaks separated from each other (min 200 units apart)
		var too_close := false
		for existing in _mountain_peaks:
			if Vector2(px - existing["pos"].x, pz - existing["pos"].z).length() < 200.0:
				too_close = true
				break
		if too_close:
			continue
		var peak_height := 100.0 + _rng.randf() * 200.0  # 100-300 units tall
		var peak_radius := 120.0 + _rng.randf() * 180.0  # 120-300 unit falloff (tighter = more distinct)
		_mountain_peaks.append({
			"pos": Vector3(px, 0.0, pz),
			"height": peak_height,
			"radius": peak_radius,
		})

	# Sample height at zone centers (peaks not included yet since they reference this)
	_forest_center.y = sample_height(_forest_center.x, _forest_center.z)
	_castle_center.y = sample_height(_castle_center.x, _castle_center.z)


# ---------------------------------------------------------------------------
# sample_height — core height function (world.js 148-199)
# ---------------------------------------------------------------------------
func sample_height(x: float, z: float) -> float:
	var broad := _noise_broad.get_noise_3d(x + _offset_x, 0.15, z + _offset_z) * 20.0
	var hills := _noise_hills.get_noise_3d(x + _offset_x, 0.32, z + _offset_z) * 8.0
	var ridge := _noise_ridge.get_noise_3d(x - _ridge_offset, 0.52, z + _ridge_offset * 0.35) * 3.0

	# Mountain peaks — multiple peaks with varying height and radius
	var peak_lift := 0.0
	for peak in _mountain_peaks:
		var peak_dist := Vector2(x - peak["pos"].x, z - peak["pos"].z).length()
		var peak_raw := maxf(0.0, 1.0 - peak_dist / peak["radius"])
		peak_lift += _smootherstep(peak_raw) * peak["height"]

	# Forest lift
	var forest_dist := Vector2(x - _forest_center.x, z - _forest_center.z).length()
	var forest_lift := maxf(0.0, 1.0 - forest_dist / 132.0) * 3.0

	# Castle lift
	var castle_dist_lift := Vector2(x - _castle_center.x, z - _castle_center.z).length()
	var castle_lift := maxf(0.0, 1.0 - castle_dist_lift / 108.0) * 4.0

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
	if castle_distance < 96.0:
		var plateau := 12.0 + _noise_broad.get_noise_3d(
			(x + _offset_x) * (70.0 / 30.0), 0.1, (z + _offset_z) * (70.0 / 30.0)
		) * 0.32
		var blend := _smootherstep(castle_distance / 96.0)
		height = lerpf(plateau, height, blend)

	# View lane from spawn to castle
	var castle_length_sq := _castle_center.x * _castle_center.x + _castle_center.z * _castle_center.z
	if castle_length_sq > 0.0:
		var projection := (x * _castle_center.x + z * _castle_center.z) / castle_length_sq
		var clamped_proj := clampf(projection, 0.0, 1.0)
		var nearest_x := _castle_center.x * clamped_proj
		var nearest_z := _castle_center.z * clamped_proj
		var view_distance := Vector2(x - nearest_x, z - nearest_z).length()
		var in_view_lane := clamped_proj > 0.12 and clamped_proj < 0.88 and view_distance < 72.0

		if in_view_lane:
			var lane_target := (
				8.4
				+ _noise_broad.get_noise_3d(
					(x + _offset_x) * (70.0 / 20.0), 0.12, (z + _offset_z) * (70.0 / 20.0)
				) * 0.12
				+ clamped_proj * 0.6
			)
			var lane_blend := 1.0 - _smootherstep(view_distance / 72.0)
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

	# Build triangles — indexed for smooth normals
	# First pass: add all vertices with normals computed from height gradient
	for iz in range(seg + 1):
		for ix in range(seg + 1):
			var idx := iz * (seg + 1) + ix
			var x := -half + ix * step
			var z := -half + iz * step
			var y := heights[idx]

			# Compute normal from height gradient (central differences)
			var dx_val: float
			var dz_val: float
			if ix > 0 and ix < seg:
				dx_val = (heights[iz * (seg + 1) + ix + 1] - heights[iz * (seg + 1) + ix - 1]) / (2.0 * step)
			elif ix == 0:
				dx_val = (heights[iz * (seg + 1) + 1] - y) / step
			else:
				dx_val = (y - heights[iz * (seg + 1) + ix - 1]) / step

			if iz > 0 and iz < seg:
				dz_val = (heights[(iz + 1) * (seg + 1) + ix] - heights[(iz - 1) * (seg + 1) + ix]) / (2.0 * step)
			elif iz == 0:
				dz_val = (heights[(seg + 1) + ix] - y) / step
			else:
				dz_val = (y - heights[(iz - 1) * (seg + 1) + ix]) / step

			var normal := Vector3(-dx_val, 1.0, -dz_val).normalized()

			st.set_color(colors[idx])
			st.set_normal(normal)
			st.set_uv(Vector2(float(ix) / float(seg), float(iz) / float(seg)))
			st.add_vertex(Vector3(x, y, z))

	# Second pass: add triangle indices
	for iz in range(seg):
		for ix in range(seg):
			var i00 := iz * (seg + 1) + ix
			var i10 := i00 + 1
			var i01 := i00 + (seg + 1)
			var i11 := i01 + 1
			st.add_index(i00)
			st.add_index(i10)
			st.add_index(i01)
			st.add_index(i10)
			st.add_index(i11)
			st.add_index(i01)

	var arr_mesh := st.commit()
	self.mesh = arr_mesh

	# Material — smooth shading with noise normal map for micro detail
	var mat := StandardMaterial3D.new()
	mat.vertex_color_use_as_albedo = true
	mat.roughness = 0.92
	mat.metallic = 0.02
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_PER_PIXEL

	# Procedural normal map — adds surface texture that catches light
	var normal_noise := FastNoiseLite.new()
	normal_noise.noise_type = FastNoiseLite.TYPE_PERLIN
	normal_noise.frequency = 0.3
	normal_noise.seed = seed_val + 999
	var normal_tex := NoiseTexture2D.new()
	normal_tex.noise = normal_noise
	normal_tex.width = 512
	normal_tex.height = 512
	normal_tex.as_normal_map = true
	normal_tex.bump_strength = 4.0
	mat.normal_enabled = true
	mat.normal_texture = normal_tex
	mat.normal_scale = 0.6
	mat.uv1_scale = Vector3(8.0, 8.0, 8.0)

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

	# Snow and rock at high elevations (mountain peaks)
	if y > 80.0:
		# Snow — mix in based on height, with noise for patchy snowline
		var snow_noise := _noise_moisture.get_noise_3d(x * 0.1, 2.0, z * 0.1) * 10.0
		if y > 100.0 + snow_noise:
			return PALETTE_SNOW
		else:
			return PALETTE_SNOW.lerp(PALETTE_ROCK, clampf((100.0 + snow_noise - y) / 20.0, 0.0, 1.0))
	elif y > 50.0:
		# Rocky slopes below snowline
		var rock_blend := clampf((y - 50.0) / 30.0, 0.0, 1.0)
		return PALETTE_HIGHLAND.lerp(PALETTE_ROCK, rock_blend)
	elif spawn_dist < GameState.SPAWN_BLEND_RADIUS + 8:
		return PALETTE_SPAWN
	elif forest_dist < 156.0:
		return PALETTE_FOREST
	elif y > 30.0:
		return PALETTE_HIGHLAND
	elif y > 20.0:
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
