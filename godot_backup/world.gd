extends Node3D

const BLOCK_TYPES := {
	"air":   {"solid": false, "color": Color.BLACK},
	"grass": {"solid": true,  "color": Color8(95,125,74)},
	"dirt":  {"solid": true,  "color": Color8(122,90,58)},
	"stone": {"solid": true,  "color": Color8(138,138,138)},
	"wood":  {"solid": true,  "color": Color8(107,75,42)},
	"plank": {"solid": true,  "color": Color8(179,138,98)},
	"thatch":{"solid": true,  "color": Color8(206,180,100)},
}

const SX := 32
const SZ := 32
const SY := 48
const BASE_H := 18
const NOISE_SCALE := 0.08

var grid := {} # key String "x,y,z" -> block string
var multimeshes := {} # type -> MultiMeshInstance3D
var counts := {}      # type -> int
var dirty := false

# tiny value noise (deterministic)
var _seed := 1337

func _ready() -> void:
	_init_multimeshes()
	_generate_world()
	_draw_all()

func _process(_dt: float) -> void:
	if dirty:
		_draw_all()
		dirty = false

func _init_multimeshes() -> void:
	for t in BLOCK_TYPES.keys():
		if not BLOCK_TYPES[t].solid: continue
		var mmi := MultiMeshInstance3D.new()
		var mm := MultiMesh.new()
		mm.transform_format = MultiMesh.TRANSFORM_3D
		mm.color_format = MultiMesh.COLOR_8BIT
		mm.custom_data_format = MultiMesh.CUSTOM_DATA_NONE
		mm.mesh = _colored_box_mesh(BLOCK_TYPES[t].color)
		mm.instance_count = SX*SY*SZ
		mm.visible_instance_count = 0
		mmi.multimesh = mm
		add_child(mmi)
		multimeshes[t] = mmi
		counts[t] = 0

func _colored_box_mesh(color: Color) -> ArrayMesh:
	var box := BoxMesh.new()
	box.size = Vector3.ONE
	box.material = _flat_material(color)
	var am := ArrayMesh.new()
	am.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, box.get_mesh_arrays())
	am.surface_set_material(0, box.material)
	return am

func _flat_material(color: Color) -> StandardMaterial3D:
	var m := StandardMaterial3D.new()
	m.albedo_color = color
	m.metallic = 0.0
	m.roughness = 1.0
	m.vertex_color_use_as_albedo = false
	return m

func _key(x:int,y:int,z:int) -> String: return "%d,%d,%d" % [x,y,z]
func _parse_key(k:String) -> Vector3i:
	var p := k.split(",")
	return Vector3i(int(p[0]), int(p[1]), int(p[2]))

func _rand(x:int, y:int) -> float:
	var s := sin(x * 127.1 + y * 311.7 + float(_seed)) * 43758.5453
	return s - floor(s)

func _value_noise2d(x: float, y: float) -> float:
	var x0 := floor(x); var y0 := floor(y)
	var x1 := x0 + 1.0; var y1 := y0 + 1.0
	var sx := x - x0; var sy := y - y0
	var n00 := _rand(int(x0), int(y0))
	var n10 := _rand(int(x1), int(y0))
	var n01 := _rand(int(x0), int(y1))
	var n11 := _rand(int(x1), int(y1))
	var ix0 := lerp(n00, n10, sx)
	var ix1 := lerp(n01, n11, sx)
	return lerp(ix0, ix1, sy)

func _generate_world() -> void:
	for x in SX:
		for z in SZ:
			var hnoise := _value_noise2d(x * NOISE_SCALE, z * NOISE_SCALE)
			var hill := int(floor(hnoise * 10.0))
			var height := clamp(BASE_H + hill, 6, SY - 1)
			for y in height + 1:
				var t := "stone"
				if y == height: t = "grass"
				elif y > height - 3: t = "dirt"
				grid[_key(x,y,z)] = t

	# small fenced square + two huts near center
	var cx := SX/2; var cz := SZ/2
	var gy := ground_y(cx, cz) + 1
	for dx in range(-6,7):
		set_block(cx+dx, gy, cz-6, "wood", true)
		set_block(cx+dx, gy, cz+6, "wood", true)
	for dz in range(-6,7):
		set_block(cx-6, gy, cz+dz, "wood", true)
		set_block(cx+6, gy, cz+dz, "wood", true)
	_hut(cx-2, gy, cz-2)
	_hut(cx+3, gy, cz+1)

func _hut(ax:int, ay:int, az:int) -> void:
	for x in 3:
		for z in 3:
			set_block(ax+x, ay-1, az+z, "stone", true)
	for y in 3:
		for i in 3:
			set_block(ax+i, ay+y, az, "plank", true)
			set_block(ax+i, ay+y, az+2, "plank", true)
			set_block(ax, ay+y, az+i, "plank", true)
			set_block(ax+2, ay+y, az+i, "plank", true)
	for x in range(-1,4):
		for z in range(-1,4):
			set_block(ax+x, ay+3, az+z, "thatch", true)

func ground_y(x:int, z:int) -> int:
	for y in range(SY-1, -1, -1):
		var t := grid.get(_key(x,y,z), null)
		if t != null and BLOCK_TYPES[t].solid:
			return y
	return 0

func set_block(x:int, y:int, z:int, t:String, drawing := false) -> void:
	if x < 0 or z < 0 or y < 0 or x >= SX or z >= SZ or y >= SY: return
	if t == "air":
		grid.erase(_key(x,y,z))
	else:
		grid[_key(x,y,z)] = t
	if not drawing: dirty = true

func place_block_worldspace(world_pos: Vector3, t: String) -> void:
	var gx := int(floor(world_pos.x))
	var gy := int(floor(world_pos.y))
	var gz := int(floor(world_pos.z))
	set_block(gx, gy, gz, t)

func break_at_hit(hit: Dictionary) -> void:
	# Called by Player when ray hits a block instance; convert hit to grid coords.
	# hit.position is on the surface—snap inward slightly along normal.
	var p: Vector3 = hit.position - hit.normal * 0.01
	var x := int(floor(p.x))
	var y := int(floor(p.y))
	var z := int(floor(p.z))
	set_block(x, y, z, "air")

func _draw_all() -> void:
	for t in multimeshes.keys():
		counts[t] = 0

	var mm_trans := Transform3D.IDENTITY
	for k in grid.keys():
		var t := grid[k]
		if not BLOCK_TYPES[t].solid: continue
		var p := _parse_key(k)
		if not _has_any_air_neighbor(p.x, p.y, p.z): continue
		var mmi: MultiMeshInstance3D = multimeshes[t]
		var idx := counts[t]
		counts[t] += 1
		mm_trans.origin = Vector3(p.x + 0.5, p.y + 0.5, p.z + 0.5)
		mmi.multimesh.set_instance_transform(idx, mm_trans)

	for t in multimeshes.keys():
		var mmi: MultiMeshInstance3D = multimeshes[t]
		mmi.multimesh.visible_instance_count = counts[t]

func _has_any_air_neighbor(x:int,y:int,z:int) -> bool:
	var n = [[1,0,0],[-1,0,0],[0,1,0],[0,-1,0],[0,0,1],[0,0,-1]]
	for d in n:
		var k := _key(x+d[0], y+d[1], z+d[2])
		if not grid.has(k) or not BLOCK_TYPES[grid.get(k,"air")].solid:
			return true
	return false

