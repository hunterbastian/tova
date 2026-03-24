extends Node3D

# ---------------------------------------------------------------------------
# Member variables
# ---------------------------------------------------------------------------
var _rng: RandomNumberGenerator
var _material_cache: Dictionary = {}


# ---------------------------------------------------------------------------
# Material helpers (cached to avoid duplicates)
# ---------------------------------------------------------------------------
func _create_flat_material(color: String, roughness: float = 0.96, metalness: float = 0.02) -> StandardMaterial3D:
	var key := "%s_%.2f_%.2f" % [color, roughness, metalness]
	if _material_cache.has(key):
		return _material_cache[key]
	var mat := StandardMaterial3D.new()
	mat.albedo_color = Color(color)
	mat.roughness = roughness
	mat.metallic = metalness
	_material_cache[key] = mat
	return mat


func _create_unshaded_material(color: String) -> StandardMaterial3D:
	var key := "unshaded_" + color
	if _material_cache.has(key):
		return _material_cache[key]
	var mat := StandardMaterial3D.new()
	mat.albedo_color = Color(color)
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	_material_cache[key] = mat
	return mat


# ---------------------------------------------------------------------------
# Rock helper — port of createRock (world.js 287-310)
# ---------------------------------------------------------------------------
func _create_rock(pos: Vector3, scale_val: float, color: String = "#6e6a63") -> void:
	var mesh_inst := MeshInstance3D.new()
	var sphere := SphereMesh.new()
	sphere.radius = 1.0
	sphere.height = 2.0
	sphere.rings = 3
	sphere.radial_segments = 5
	mesh_inst.mesh = sphere
	mesh_inst.material_override = _create_flat_material(color)
	mesh_inst.position = pos
	mesh_inst.scale = Vector3.ONE * scale_val
	mesh_inst.rotation = Vector3(_rng.randf() * PI, _rng.randf() * PI, _rng.randf() * PI)
	mesh_inst.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
	add_child(mesh_inst)

	# Collision
	var body := StaticBody3D.new()
	body.position = pos
	var col := CollisionShape3D.new()
	var shape := CylinderShape3D.new()
	shape.radius = scale_val * 0.55
	shape.height = scale_val * 2.0
	col.shape = shape
	body.add_child(col)
	add_child(body)


# ---------------------------------------------------------------------------
# Brazier helper — port of createBrazier (world.js 312-337)
# ---------------------------------------------------------------------------
func _create_brazier(pos: Vector3) -> void:
	var brazier := Node3D.new()
	brazier.position = pos

	# Bowl
	var bowl_mesh := CylinderMesh.new()
	bowl_mesh.top_radius = 0.22
	bowl_mesh.bottom_radius = 0.3
	bowl_mesh.height = 0.24
	bowl_mesh.radial_segments = 8
	var bowl := MeshInstance3D.new()
	bowl.mesh = bowl_mesh
	bowl.material_override = _create_flat_material("#50443a")
	bowl.position.y = 1.25
	bowl.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
	brazier.add_child(bowl)

	# Stem
	var stem_mesh := CylinderMesh.new()
	stem_mesh.top_radius = 0.06
	stem_mesh.bottom_radius = 0.08
	stem_mesh.height = 1.1
	stem_mesh.radial_segments = 6
	var stem := MeshInstance3D.new()
	stem.mesh = stem_mesh
	stem.material_override = _create_flat_material("#70645a")
	stem.position.y = 0.55
	stem.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
	brazier.add_child(stem)

	# Flame
	var flame_mesh := SphereMesh.new()
	flame_mesh.radius = 0.14
	flame_mesh.height = 0.28
	flame_mesh.rings = 10
	flame_mesh.radial_segments = 12
	var flame := MeshInstance3D.new()
	flame.mesh = flame_mesh
	flame.material_override = _create_unshaded_material("#f6c56d")
	flame.position.y = 1.32
	brazier.add_child(flame)

	# Light
	var light := OmniLight3D.new()
	light.light_color = Color("#f0bf63")
	light.light_energy = 1.2
	light.omni_range = 13.0
	light.omni_attenuation = 2.0
	light.position.y = 1.5
	brazier.add_child(light)

	add_child(brazier)


# ---------------------------------------------------------------------------
# Collider helpers
# ---------------------------------------------------------------------------
func _add_box_collider(pos: Vector3, half_x: float, half_z: float, half_y: float = 5.0) -> void:
	var body := StaticBody3D.new()
	body.position = pos
	var col := CollisionShape3D.new()
	var shape := BoxShape3D.new()
	shape.size = Vector3(half_x * 2.0, half_y * 2.0, half_z * 2.0)
	col.shape = shape
	body.add_child(col)
	add_child(body)


func _add_cylinder_collider(pos: Vector3, radius: float, height: float = 10.0) -> void:
	var body := StaticBody3D.new()
	body.position = pos
	var col := CollisionShape3D.new()
	var shape := CylinderShape3D.new()
	shape.radius = radius
	shape.height = height
	col.shape = shape
	body.add_child(col)
	add_child(body)


# ---------------------------------------------------------------------------
# build_spawn_sanctum — port of buildSpawnSanctum (world.js 339-451)
# ---------------------------------------------------------------------------
func build_spawn_sanctum(seed_val: int, terrain: MeshInstance3D) -> void:
	_rng = RandomNumberGenerator.new()
	_rng.seed = seed_val ^ 0xa7810d3f

	var shrine_x := 7.6 + _rng.randf() * 1.4
	var shrine_z := 2.4 + _rng.randf() * 1.2
	var shrine_y: float = terrain.sample_height(shrine_x, shrine_z)

	var shrine := Node3D.new()
	shrine.position = Vector3(shrine_x, shrine_y, shrine_z)

	# Dais
	var dais_mesh := CylinderMesh.new()
	dais_mesh.top_radius = 1.7
	dais_mesh.bottom_radius = 2.1
	dais_mesh.height = 0.72
	dais_mesh.radial_segments = 10
	var dais := MeshInstance3D.new()
	dais.mesh = dais_mesh
	dais.material_override = _create_flat_material("#84796f")
	dais.position.y = 0.36
	dais.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
	shrine.add_child(dais)

	# Altar
	var altar_mesh := BoxMesh.new()
	altar_mesh.size = Vector3(0.86, 1.28, 0.86)
	var altar := MeshInstance3D.new()
	altar.mesh = altar_mesh
	altar.material_override = _create_flat_material("#84796f")
	altar.position.y = 1.05
	altar.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
	shrine.add_child(altar)

	# Stele
	var stele_mesh := BoxMesh.new()
	stele_mesh.size = Vector3(1.2, 2.8, 0.34)
	var stele := MeshInstance3D.new()
	stele.mesh = stele_mesh
	stele.material_override = _create_flat_material("#84796f")
	stele.position = Vector3(0.0, 1.8, 1.1)
	stele.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
	shrine.add_child(stele)

	# Arch left post
	var arch_post_mesh := BoxMesh.new()
	arch_post_mesh.size = Vector3(0.28, 2.2, 0.28)
	var arch_left := MeshInstance3D.new()
	arch_left.mesh = arch_post_mesh
	arch_left.material_override = _create_flat_material("#84796f")
	arch_left.position = Vector3(-0.95, 1.4, 0.82)
	arch_left.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
	shrine.add_child(arch_left)

	# Arch right post
	var arch_right := MeshInstance3D.new()
	arch_right.mesh = arch_post_mesh
	arch_right.material_override = _create_flat_material("#84796f")
	arch_right.position = Vector3(0.95, 1.4, 0.82)
	arch_right.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
	shrine.add_child(arch_right)

	# Arch cap
	var arch_cap_mesh := BoxMesh.new()
	arch_cap_mesh.size = Vector3(2.18, 0.26, 0.28)
	var arch_cap := MeshInstance3D.new()
	arch_cap.mesh = arch_cap_mesh
	arch_cap.material_override = _create_flat_material("#84796f")
	arch_cap.position = Vector3(0.0, 2.45, 0.82)
	arch_cap.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
	shrine.add_child(arch_cap)

	add_child(shrine)

	# Sword pickup position
	GameState.sword_pickup_position = Vector3(shrine_x, shrine_y + 1.6, shrine_z)

	# Shrine colliders (world-space)
	_add_box_collider(Vector3(shrine_x, shrine_y, shrine_z), 0.5, 0.5)
	_add_box_collider(Vector3(shrine_x, shrine_y, shrine_z + 1.1), 0.65, 0.2)
	_add_cylinder_collider(Vector3(shrine_x - 0.95, shrine_y, shrine_z + 0.82), 0.2)
	_add_cylinder_collider(Vector3(shrine_x + 0.95, shrine_y, shrine_z + 0.82), 0.2)

	# Braziers
	_create_brazier(Vector3(shrine_x - 1.7, shrine_y + 0.02, shrine_z + 0.8))
	_create_brazier(Vector3(shrine_x + 1.7, shrine_y + 0.02, shrine_z + 0.8))

	# Path stones — 5 lerped from origin to shrine
	var path_stone_mat := _create_flat_material("#84796f")
	for index in range(5):
		var t := float(index) / 6.0
		var px := lerpf(0.0, shrine_x, t)
		var pz := lerpf(0.0, shrine_z, t)
		var py: float = terrain.sample_height(px, pz) + 0.05
		var stone_mesh := BoxMesh.new()
		stone_mesh.size = Vector3(
			0.44 + _rng.randf() * 0.18,
			0.12,
			0.62 + _rng.randf() * 0.22
		)
		var stone := MeshInstance3D.new()
		stone.mesh = stone_mesh
		stone.material_override = path_stone_mat
		stone.position = Vector3(px, py, pz)
		stone.rotation.y = _rng.randf() * PI
		stone.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
		add_child(stone)

	# Scatter rocks around spawn — 14
	for index in range(14):
		var angle := _rng.randf() * TAU
		var dist := GameState.SPAWN_RADIUS + 2.0 + _rng.randf() * (GameState.SPAWN_BLEND_RADIUS - GameState.SPAWN_RADIUS + 6.0)
		var rx := cos(angle) * dist
		var rz := sin(angle) * dist
		var ry: float = terrain.sample_height(rx, rz)
		_create_rock(Vector3(rx, ry, rz), 0.25 + _rng.randf() * 0.45)

	# Grass tufts — 180 instanced
	var grass_mm := MultiMesh.new()
	var grass_cone := CylinderMesh.new()
	grass_cone.top_radius = 0.0
	grass_cone.bottom_radius = 0.15
	grass_cone.height = 0.55
	grass_cone.radial_segments = 4
	grass_mm.mesh = grass_cone
	grass_mm.transform_format = MultiMesh.TRANSFORM_3D
	grass_mm.instance_count = 180

	for index in range(180):
		var angle := _rng.randf() * TAU
		var dist := 1.5 + _rng.randf() * (GameState.SPAWN_BLEND_RADIUS + 10.0)
		var gx := cos(angle) * dist
		var gz := sin(angle) * dist
		var gy: float = terrain.sample_height(gx, gz)
		var basis := Basis.from_scale(Vector3(
			0.6 + _rng.randf() * 0.6,
			0.7 + _rng.randf() * 0.8,
			0.6 + _rng.randf() * 0.6
		))
		basis = basis.rotated(Vector3.UP, _rng.randf() * TAU)
		var t3d := Transform3D(basis, Vector3(gx, gy + 0.22, gz))
		grass_mm.set_instance_transform(index, t3d)

	var grass_mmi := MultiMeshInstance3D.new()
	grass_mmi.multimesh = grass_mm
	grass_mmi.material_override = _create_flat_material("#9a8a68")
	add_child(grass_mmi)


# ---------------------------------------------------------------------------
# build_forest — port of buildForest (world.js 454-515)
# ---------------------------------------------------------------------------
func build_forest(seed_val: int, terrain: MeshInstance3D) -> void:
	_rng = RandomNumberGenerator.new()
	_rng.seed = seed_val ^ 0x1f123bb5

	const TREE_COUNT := 6000

	# Clearing noise — drives where fields/clearings appear
	var clearing_noise := FastNoiseLite.new()
	clearing_noise.noise_type = FastNoiseLite.TYPE_PERLIN
	clearing_noise.frequency = 0.02
	clearing_noise.seed = seed_val

	# Skyrim-style boreal pine: tall trunk + 4 tiered branch layers
	# Trunk — tall, tapered
	var trunk_mesh := CylinderMesh.new()
	trunk_mesh.top_radius = 0.12
	trunk_mesh.bottom_radius = 0.4
	trunk_mesh.height = 1.0  # scaled per-instance
	trunk_mesh.radial_segments = 6

	var trunk_mm := MultiMesh.new()
	trunk_mm.mesh = trunk_mesh
	trunk_mm.transform_format = MultiMesh.TRANSFORM_3D
	trunk_mm.instance_count = TREE_COUNT

	# 4 canopy tiers — each a cone, progressively smaller toward top
	# Tier 1 (bottom): widest
	# Tier 2: medium
	# Tier 3: narrow
	# Tier 4 (top): pointed cap
	var tier_meshes: Array[CylinderMesh] = []
	var tier_radii := [2.4, 1.9, 1.4, 0.7]
	var tier_heights := [2.2, 2.0, 1.6, 1.3]
	var tier_mms: Array[MultiMesh] = []

	for i in range(4):
		var m := CylinderMesh.new()
		m.top_radius = tier_radii[i] * 0.15  # slight taper, not pure cone
		m.bottom_radius = tier_radii[i]
		m.height = tier_heights[i]
		m.radial_segments = 7
		tier_meshes.append(m)

		var mm := MultiMesh.new()
		mm.mesh = m
		mm.transform_format = MultiMesh.TRANSFORM_3D
		mm.instance_count = TREE_COUNT
		tier_mms.append(mm)

	var cc := GameState.castle_center
	var half := float(GameState.WORLD_SIZE) / 2.0

	# Single StaticBody3D for all tree colliders (optimization)
	var tree_collider_body := StaticBody3D.new()
	var shared_cylinder := CylinderShape3D.new()
	shared_cylinder.radius = 0.38
	shared_cylinder.height = 10.0

	var placed := 0
	var attempts := 0
	while placed < TREE_COUNT and attempts < 40000:
		attempts += 1
		var x := _rng.randf_range(-half + 5.0, half - 5.0)
		var z := _rng.randf_range(-half + 5.0, half - 5.0)

		# Skip spawn area
		var spawn_distance := Vector2(x, z).length()
		if spawn_distance < GameState.SPAWN_BLEND_RADIUS + 4.0:
			continue
		# Skip castle interior
		var castle_distance := Vector2(x - cc.x, z - cc.z).length()
		if castle_distance < 16.0:
			continue

		# Clearing check — noise > 0.25 means open field
		var clearing_val := clearing_noise.get_noise_2d(x, z)
		if clearing_val > 0.25:
			continue
		# Density falloff at clearing edges
		if clearing_val > 0.1 and _rng.randf() > 0.5:
			continue

		var y: float = terrain.sample_height(x, z)
		var trunk_height := 3.5 + _rng.randf() * 6.0  # 3.5-9.5 units — big variation
		var tree_scale := 0.6 + _rng.randf() * 0.8  # 0.6-1.4 — wide size range

		# Trunk — tall and tapered
		trunk_mm.set_instance_transform(placed, Transform3D(
			Basis.from_scale(Vector3(tree_scale, trunk_height, tree_scale)),
			Vector3(x, y + trunk_height * 0.5, z)
		))

		# 4 canopy tiers stacked up the trunk — each rotated and slightly irregular
		var canopy_start := y + trunk_height * 0.4
		var tier_spacing := trunk_height * 0.17
		var tier_overlap := 0.3 + _rng.randf() * 0.2  # how much tiers overlap
		for tier_idx in range(4):
			var tier_y := canopy_start + tier_idx * (tier_spacing + tier_overlap)
			var tier_scale := tree_scale * (1.0 - tier_idx * 0.06)
			# Irregular spread — stretch X or Z slightly per tier
			var stretch_x := tier_scale * (0.9 + _rng.randf() * 0.3)
			var stretch_z := tier_scale * (0.9 + _rng.randf() * 0.3)
			# Rotate each tier so branches don't stack
			var tier_rot := _rng.randf() * TAU
			var basis := Basis(Vector3.UP, tier_rot) * Basis.from_scale(Vector3(stretch_x, 1.0, stretch_z))
			tier_mms[tier_idx].set_instance_transform(placed, Transform3D(
				basis, Vector3(x, tier_y, z)
			))

		# Batched collision — single body, many shapes
		var col := CollisionShape3D.new()
		col.shape = shared_cylinder
		col.position = Vector3(x, y, z)
		tree_collider_body.add_child(col)

		placed += 1

	add_child(tree_collider_body)

	# Resize MultiMesh if we placed fewer than allocated
	if placed < TREE_COUNT:
		trunk_mm.instance_count = placed
		for mm in tier_mms:
			mm.instance_count = placed

	# Trunk — dark bark with slight roughness
	var trunk_mmi := MultiMeshInstance3D.new()
	trunk_mmi.multimesh = trunk_mm
	trunk_mmi.material_override = _create_flat_material("#3a2a1a", 0.95, 0.02)
	trunk_mmi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
	add_child(trunk_mmi)

	# Canopy tiers — gradient from dark (top) to lighter (bottom)
	var tier_colors := ["#1a3a18", "#224422", "#2a5228", "#326032"]
	for tier_idx in range(4):
		var mmi := MultiMeshInstance3D.new()
		mmi.multimesh = tier_mms[tier_idx]
		mmi.material_override = _create_flat_material(tier_colors[tier_idx])
		mmi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
		add_child(mmi)

	# Scatter rocks across the map — 60
	for index in range(60):
		var x := _rng.randf_range(-half + 5.0, half - 5.0)
		var z := _rng.randf_range(-half + 5.0, half - 5.0)
		var y: float = terrain.sample_height(x, z)
		_create_rock(Vector3(x, y, z), 0.45 + _rng.randf() * 0.55)

	# Ground grass — thousands of small blades across the map
	var grass_blade := CylinderMesh.new()
	grass_blade.top_radius = 0.0
	grass_blade.bottom_radius = 0.06
	grass_blade.height = 0.4
	grass_blade.radial_segments = 3

	const GRASS_COUNT := 8000
	var grass_mm := MultiMesh.new()
	grass_mm.mesh = grass_blade
	grass_mm.transform_format = MultiMesh.TRANSFORM_3D
	grass_mm.instance_count = GRASS_COUNT

	for gi in range(GRASS_COUNT):
		var gx := _rng.randf_range(-half + 2.0, half - 2.0)
		var gz := _rng.randf_range(-half + 2.0, half - 2.0)
		var gy: float = terrain.sample_height(gx, gz)
		var blade_height := 0.3 + _rng.randf() * 0.35
		var blade_lean := (_rng.randf() - 0.5) * 0.3
		var basis := Basis(Vector3.UP, _rng.randf() * TAU) * Basis.from_scale(Vector3(
			0.6 + _rng.randf() * 0.5,
			blade_height / 0.4,
			0.6 + _rng.randf() * 0.5
		))
		basis = basis.rotated(Vector3.RIGHT, blade_lean)
		grass_mm.set_instance_transform(gi, Transform3D(basis, Vector3(gx, gy + blade_height * 0.4, gz)))

	var grass_mmi := MultiMeshInstance3D.new()
	grass_mmi.multimesh = grass_mm
	grass_mmi.material_override = _create_flat_material("#4a7a38")
	grass_mmi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(grass_mmi)


# ---------------------------------------------------------------------------
# build_castle — port of buildCastle (world.js 517-613)
# ---------------------------------------------------------------------------
func build_castle(seed_val: int, terrain: MeshInstance3D) -> void:
	_rng = RandomNumberGenerator.new()
	_rng.seed = seed_val ^ 0x9e3779b9

	var cx := GameState.castle_center.x
	var cz := GameState.castle_center.z
	var base_y: float = terrain.sample_height(cx, cz)

	var wall_mat := _create_flat_material("#68675f", 0.95, 0.03)
	var roof_mat := _create_flat_material("#3f3933", 0.92, 0.01)

	var castle := Node3D.new()
	castle.position = Vector3(cx, base_y, cz)

	# Courtyard
	var courtyard_mesh := BoxMesh.new()
	courtyard_mesh.size = Vector3(22.0, 1.9, 18.0)
	var courtyard := MeshInstance3D.new()
	courtyard.mesh = courtyard_mesh
	courtyard.material_override = wall_mat
	courtyard.position.y = 0.8
	courtyard.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
	castle.add_child(courtyard)

	# Wall segments
	var wall_data: Array = [
		[Vector3(22.0, 5.8, 1.3),  Vector3(0.0, 3.8, -8.4)],
		[Vector3(22.0, 5.8, 1.3),  Vector3(0.0, 3.8, 8.4)],
		[Vector3(1.3, 5.8, 15.5),  Vector3(-10.3, 3.8, 0.0)],
		[Vector3(1.3, 5.8, 15.5),  Vector3(10.3, 3.8, 0.0)],
		[Vector3(8.4, 8.6, 6.6),   Vector3(0.0, 5.4, 0.5)],
	]
	for entry in wall_data:
		var wall_mesh := BoxMesh.new()
		wall_mesh.size = entry[0]
		var wall := MeshInstance3D.new()
		wall.mesh = wall_mesh
		wall.material_override = wall_mat
		wall.position = entry[1]
		wall.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
		castle.add_child(wall)

	# Corner towers
	var tower_offsets: Array = [
		Vector3(-9.4, 0.0, -7.7),
		Vector3(9.4, 0.0, -7.7),
		Vector3(-9.4, 0.0, 7.7),
		Vector3(9.4, 0.0, 7.7),
	]
	for offset in tower_offsets:
		var tower_mesh := CylinderMesh.new()
		tower_mesh.top_radius = 1.95
		tower_mesh.bottom_radius = 2.15
		tower_mesh.height = 11.4
		tower_mesh.radial_segments = 10
		var tower := MeshInstance3D.new()
		tower.mesh = tower_mesh
		tower.material_override = wall_mat
		tower.position = Vector3(offset.x, 5.7 + offset.y, offset.z)
		tower.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
		castle.add_child(tower)

		var roof_mesh := CylinderMesh.new()
		roof_mesh.top_radius = 0.0
		roof_mesh.bottom_radius = 2.85
		roof_mesh.height = 3.8
		roof_mesh.radial_segments = 10
		var roof := MeshInstance3D.new()
		roof.mesh = roof_mesh
		roof.material_override = roof_mat
		roof.position = Vector3(offset.x, 12.8 + offset.y, offset.z)
		roof.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
		castle.add_child(roof)

	# Gate
	var gate_mesh := BoxMesh.new()
	gate_mesh.size = Vector3(5.1, 5.4, 1.5)
	var gate := MeshInstance3D.new()
	gate.mesh = gate_mesh
	gate.material_override = roof_mat
	gate.position = Vector3(0.0, 3.1, 8.5)
	gate.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
	castle.add_child(gate)

	add_child(castle)

	# Colliders (world-space)
	# Back wall
	_add_box_collider(Vector3(cx, base_y, cz - 8.4), 11.0, 0.65)
	# Front wall — split for gate opening
	_add_box_collider(Vector3(cx - 6.775, base_y, cz + 8.4), 4.225, 0.65)
	_add_box_collider(Vector3(cx + 6.775, base_y, cz + 8.4), 4.225, 0.65)
	# Side walls
	_add_box_collider(Vector3(cx - 10.3, base_y, cz), 0.65, 7.75)
	_add_box_collider(Vector3(cx + 10.3, base_y, cz), 0.65, 7.75)
	# Keep
	_add_box_collider(Vector3(cx, base_y, cz + 0.5), 4.2, 3.3)
	# Corner tower cylinders
	for offset in tower_offsets:
		_add_cylinder_collider(Vector3(cx + offset.x, base_y, cz + offset.z), 2.2)

	# Scatter rocks around castle — 10
	for index in range(10):
		var angle := _rng.randf() * TAU
		var distance := 14.0 + _rng.randf() * 12.0
		var x := cx + cos(angle) * distance
		var z := cz + sin(angle) * distance
		var y: float = terrain.sample_height(x, z)
		_create_rock(Vector3(x, y, z), 0.5 + _rng.randf() * 0.7, "#7a756c")


# ---------------------------------------------------------------------------
# build_haze — port of buildHazeAndLandmarks (world.js 615-668)
# ---------------------------------------------------------------------------
func build_haze(seed_val: int, terrain: MeshInstance3D) -> void:
	_rng = RandomNumberGenerator.new()
	_rng.seed = seed_val ^ 0x53142fcd

	# Obelisk
	var obelisk_mesh := BoxMesh.new()
	obelisk_mesh.size = Vector3(2.4, 9.0, 2.4)
	var obelisk := MeshInstance3D.new()
	obelisk.mesh = obelisk_mesh
	obelisk.material_override = _create_flat_material("#7c6f5d")
	obelisk.position = Vector3(22.0, terrain.sample_height(22.0, 12.0) + 4.5, 12.0)
	obelisk.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
	add_child(obelisk)

	_add_box_collider(Vector3(22.0, terrain.sample_height(22.0, 12.0), 12.0), 1.3, 1.3)

	# Scattered ruins — archways across the whole map (Elder Scrolls feel)
	var ruin_mat := _create_flat_material("#5a5248")
	var pillar_mesh := BoxMesh.new()
	pillar_mesh.size = Vector3(0.42, 3.4, 0.48)
	var lintel_mesh := BoxMesh.new()
	lintel_mesh.size = Vector3(2.8, 0.42, 0.52)
	var half := float(GameState.WORLD_SIZE) / 2.0

	for index in range(12):
		var origin_x := _rng.randf_range(-half + 10.0, half - 10.0)
		var origin_z := _rng.randf_range(-half + 10.0, half - 10.0)
		if Vector2(origin_x, origin_z).length() < GameState.SPAWN_BLEND_RADIUS + 6.0:
			continue
		var origin_y: float = terrain.sample_height(origin_x, origin_z)

		var ruin := Node3D.new()
		ruin.position = Vector3(origin_x, origin_y, origin_z)
		ruin.rotation.y = _rng.randf() * TAU

		var left_pillar := MeshInstance3D.new()
		left_pillar.mesh = pillar_mesh
		left_pillar.material_override = ruin_mat
		left_pillar.position = Vector3(-1.2, 1.7, 0.0)
		left_pillar.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
		ruin.add_child(left_pillar)

		var right_pillar := MeshInstance3D.new()
		right_pillar.mesh = pillar_mesh
		right_pillar.material_override = ruin_mat
		right_pillar.position = Vector3(1.2, 1.7, 0.0)
		right_pillar.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
		ruin.add_child(right_pillar)

		# Only some ruins still have lintels (others crumbled)
		if _rng.randf() > 0.35:
			var lintel := MeshInstance3D.new()
			lintel.mesh = lintel_mesh
			lintel.material_override = ruin_mat
			lintel.position = Vector3(0.0, 3.3, 0.0)
			lintel.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
			ruin.add_child(lintel)

		add_child(ruin)

	# Standing stones — circles of tall thin stones (like Skyrim)
	var stone_mat := _create_flat_material("#4a4640")
	for circle_idx in range(3):
		var cx := _rng.randf_range(-half + 20.0, half - 20.0)
		var cz := _rng.randf_range(-half + 20.0, half - 20.0)
		if Vector2(cx, cz).length() < GameState.SPAWN_BLEND_RADIUS + 8.0:
			continue
		var cy: float = terrain.sample_height(cx, cz)
		var stone_count := 5 + _rng.randi_range(0, 3)
		var circle_radius := 3.0 + _rng.randf() * 2.0

		for stone_idx in range(stone_count):
			var angle := (float(stone_idx) / float(stone_count)) * TAU + _rng.randf() * 0.3
			var sx := cx + cos(angle) * circle_radius
			var sz := cz + sin(angle) * circle_radius
			var sy: float = terrain.sample_height(sx, sz)
			var stone_height := 2.5 + _rng.randf() * 2.0
			var stone_width := 0.3 + _rng.randf() * 0.2

			var stone_mesh := BoxMesh.new()
			stone_mesh.size = Vector3(stone_width, stone_height, stone_width * 0.8)
			var stone := MeshInstance3D.new()
			stone.mesh = stone_mesh
			stone.material_override = stone_mat
			stone.position = Vector3(sx, sy + stone_height * 0.5, sz)
			# Slight tilt — weathered, ancient
			stone.rotation = Vector3(
				(_rng.randf() - 0.5) * 0.15,
				angle + PI,
				(_rng.randf() - 0.5) * 0.1
			)
			stone.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
			add_child(stone)

	# Watchtowers — tall cylindrical ruins with broken tops
	var tower_mat := _create_flat_material("#5a5550", 0.94, 0.03)
	for tower_idx in range(2):
		var tx := _rng.randf_range(-half + 15.0, half - 15.0)
		var tz := _rng.randf_range(-half + 15.0, half - 15.0)
		if Vector2(tx, tz).length() < GameState.SPAWN_BLEND_RADIUS + 10.0:
			continue
		var castle_dist := Vector2(tx - GameState.castle_center.x, tz - GameState.castle_center.z).length()
		if castle_dist < 20.0:
			continue
		var ty: float = terrain.sample_height(tx, tz)
		var tower_height := 8.0 + _rng.randf() * 4.0

		var tower_mesh := CylinderMesh.new()
		tower_mesh.top_radius = 1.4
		tower_mesh.bottom_radius = 1.8
		tower_mesh.height = tower_height
		tower_mesh.radial_segments = 8
		var tower := MeshInstance3D.new()
		tower.mesh = tower_mesh
		tower.material_override = tower_mat
		tower.position = Vector3(tx, ty + tower_height * 0.5, tz)
		tower.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
		add_child(tower)
		_add_cylinder_collider(Vector3(tx, ty, tz), 1.8)

		# Broken parapet ring on top
		var parapet_mesh := CylinderMesh.new()
		parapet_mesh.top_radius = 1.9
		parapet_mesh.bottom_radius = 1.9
		parapet_mesh.height = 0.6
		parapet_mesh.radial_segments = 8
		var parapet := MeshInstance3D.new()
		parapet.mesh = parapet_mesh
		parapet.material_override = tower_mat
		parapet.position = Vector3(tx, ty + tower_height + 0.1, tz)
		parapet.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
		add_child(parapet)


# ---------------------------------------------------------------------------
# clear_structures — free all children
# ---------------------------------------------------------------------------
func clear_structures() -> void:
	for child in get_children():
		remove_child(child)
		child.queue_free()
	_material_cache.clear()
