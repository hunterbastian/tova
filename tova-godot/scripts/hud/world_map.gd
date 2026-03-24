extends Control

var _player: CharacterBody3D
var _terrain: MeshInstance3D
var _is_open := false

func set_player(player: CharacterBody3D) -> void:
	_player = player

func set_terrain(terrain: MeshInstance3D) -> void:
	_terrain = terrain

func _ready() -> void:
	visible = false
	process_mode = Node.PROCESS_MODE_ALWAYS

func _unhandled_input(event: InputEvent) -> void:
	if event.is_action_pressed("toggle_map"):
		_is_open = not _is_open
		visible = _is_open
		if _is_open:
			queue_redraw()

func _process(_delta: float) -> void:
	if _is_open:
		queue_redraw()

func _draw() -> void:
	if not _player or not _is_open:
		return

	var screen := get_viewport_rect().size
	var map_size := minf(screen.x, screen.y) * 0.75
	var offset := (screen - Vector2(map_size, map_size)) * 0.5
	var half_world := float(GameState.WORLD_SIZE) / 2.0
	var scale_factor := map_size / float(GameState.WORLD_SIZE)
	var player_pos := _player.global_position
	var yaw := _player.rotation.y

	# Background
	draw_rect(Rect2(Vector2.ZERO, screen), Color(0, 0, 0, 0.7))

	# Map border
	draw_rect(Rect2(offset - Vector2(2, 2), Vector2(map_size + 4, map_size + 4)), Color("#4a4a40"), false, 2.0)

	# Terrain height map — sample a grid and draw colored squares
	var grid := 80
	var cell := map_size / float(grid)
	for iz in range(grid):
		for ix in range(grid):
			var wx := -half_world + (float(ix) + 0.5) / float(grid) * float(GameState.WORLD_SIZE)
			var wz := -half_world + (float(iz) + 0.5) / float(grid) * float(GameState.WORLD_SIZE)
			var h: float = _terrain.sample_height(wx, wz)

			var color: Color
			if h > 100.0:
				color = Color("#e8e8f0")  # snow
			elif h > 50.0:
				var blend := (h - 50.0) / 50.0
				color = Color("#5a8a48").lerp(Color("#8a8a82"), blend)  # grass to rock
			elif h > 20.0:
				color = Color("#4a7a3a")  # highland
			else:
				color = Color("#5a9a48")  # meadow

			# Darken slightly based on height for depth
			color = color.darkened(clampf((30.0 - h) * 0.008, 0.0, 0.2))

			var rect_pos := offset + Vector2(ix * cell, iz * cell)
			draw_rect(Rect2(rect_pos, Vector2(cell + 1, cell + 1)), color)

	# Mountain peak markers
	for peak in _terrain._mountain_peaks:
		var peak_pos: Vector3 = peak["pos"]
		var px: float = offset.x + (peak_pos.x + half_world) * scale_factor
		var pz: float = offset.y + (peak_pos.z + half_world) * scale_factor
		# Triangle marker
		var tri := PackedVector2Array([
			Vector2(px, pz - 6),
			Vector2(px + 5, pz + 4),
			Vector2(px - 5, pz + 4),
		])
		draw_colored_polygon(tri, Color("#c0c0c8"))
		draw_string(ThemeDB.fallback_font, Vector2(px - 3, pz - 8), "^", HORIZONTAL_ALIGNMENT_LEFT, -1, 8, Color("#e8e8f0"))

	# Spawn marker
	var sx := offset.x + half_world * scale_factor
	var sz := offset.y + half_world * scale_factor
	draw_circle(Vector2(sx, sz), 4.0, Color("#8fa358"))
	draw_string(ThemeDB.fallback_font, Vector2(sx - 10, sz - 8), "Spawn", HORIZONTAL_ALIGNMENT_LEFT, -1, 9, Color("#b0c880"))

	# Player position + direction
	var player_mx := offset.x + (player_pos.x + half_world) * scale_factor
	var player_mz := offset.y + (player_pos.z + half_world) * scale_factor

	# Player direction arrow
	var arrow_len := 10.0
	var dir_x := -sin(yaw) * arrow_len
	var dir_z := -cos(yaw) * arrow_len
	var player_center := Vector2(player_mx, player_mz)
	draw_line(player_center, player_center + Vector2(dir_x, dir_z), Color("#f0e0c0"), 2.0)

	# Player dot
	draw_circle(player_center, 5.0, Color("#f0d888"))
	draw_circle(player_center, 3.0, Color("#f8e8b0"))

	# Title
	draw_string(ThemeDB.fallback_font, Vector2(offset.x, offset.y - 12), "WORLD MAP  [M to close]", HORIZONTAL_ALIGNMENT_LEFT, -1, 14, Color("#c0b898"))
