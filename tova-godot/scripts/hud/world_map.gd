extends Control

var _player: CharacterBody3D
var _terrain: MeshInstance3D
var _is_open := false

# Fog of war — tracks which grid cells the player has revealed
const FOG_GRID := 80
const REVEAL_RADIUS := 60.0  # world units around player that get revealed
var _revealed: Array[bool] = []

func set_player(player: CharacterBody3D) -> void:
	_player = player

func set_terrain(terrain: MeshInstance3D) -> void:
	_terrain = terrain

func _ready() -> void:
	visible = false
	process_mode = Node.PROCESS_MODE_ALWAYS
	# Initialize fog — all hidden
	_revealed.resize(FOG_GRID * FOG_GRID)
	_revealed.fill(false)

func _unhandled_input(event: InputEvent) -> void:
	if event.is_action_pressed("toggle_map"):
		_is_open = not _is_open
		visible = _is_open

func _process(_delta: float) -> void:
	if not _player:
		return
	# Continuously reveal cells near player (even when map is closed)
	_reveal_around_player()
	if _is_open:
		queue_redraw()

func _reveal_around_player() -> void:
	var half_world := float(GameState.WORLD_SIZE) / 2.0
	var cell_world := float(GameState.WORLD_SIZE) / float(FOG_GRID)
	var pos := _player.global_position
	# Which cells are within reveal radius
	var cx := int((pos.x + half_world) / cell_world)
	var cz := int((pos.z + half_world) / cell_world)
	var cell_radius := int(ceili(REVEAL_RADIUS / cell_world))
	for iz in range(maxi(0, cz - cell_radius), mini(FOG_GRID, cz + cell_radius + 1)):
		for ix in range(maxi(0, cx - cell_radius), mini(FOG_GRID, cx + cell_radius + 1)):
			var wx := -half_world + (float(ix) + 0.5) * cell_world
			var wz := -half_world + (float(iz) + 0.5) * cell_world
			var dist := Vector2(wx - pos.x, wz - pos.z).length()
			if dist < REVEAL_RADIUS:
				_revealed[iz * FOG_GRID + ix] = true

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
	draw_rect(Rect2(Vector2.ZERO, screen), Color(0, 0, 0, 0.75))

	# Map border
	draw_rect(Rect2(offset - Vector2(2, 2), Vector2(map_size + 4, map_size + 4)), Color("#5a5a50"), false, 2.0)

	# Terrain height map with fog of war
	var cell := map_size / float(FOG_GRID)
	for iz in range(FOG_GRID):
		for ix in range(FOG_GRID):
			var idx := iz * FOG_GRID + ix
			var rect_pos := offset + Vector2(ix * cell, iz * cell)

			if not _revealed[idx]:
				# Unexplored — dark with subtle noise texture
				draw_rect(Rect2(rect_pos, Vector2(cell + 1, cell + 1)), Color(0.08, 0.08, 0.06, 1.0))
				continue

			var wx := -half_world + (float(ix) + 0.5) / float(FOG_GRID) * float(GameState.WORLD_SIZE)
			var wz := -half_world + (float(iz) + 0.5) / float(FOG_GRID) * float(GameState.WORLD_SIZE)
			var h: float = _terrain.sample_height(wx, wz)

			# Check for water
			var river_n := absf(_terrain._noise_broad.get_noise_3d(
				(wx + _terrain._offset_z * 0.5) * 0.4, 0.9, (wz - _terrain._offset_x * 0.5) * 0.4
			))
			var is_water := river_n < 0.06 and h < 50.0

			var color: Color
			if is_water:
				color = Color("#3a6a8a")
			elif h > 100.0:
				color = Color("#e8e8f0")
			elif h > 50.0:
				var blend := (h - 50.0) / 50.0
				color = Color("#5a8a48").lerp(Color("#8a8a82"), blend)
			elif h > 20.0:
				color = Color("#4a7a3a")
			else:
				color = Color("#5a9a48")

			color = color.darkened(clampf((30.0 - h) * 0.008, 0.0, 0.2))

			# Fade edges of revealed area — cells near fog boundary are dimmer
			var fade := _get_edge_fade(ix, iz)
			color = color.lerp(Color(0.08, 0.08, 0.06), 1.0 - fade)

			draw_rect(Rect2(rect_pos, Vector2(cell + 1, cell + 1)), color)

	# Mountain peak markers (only if revealed)
	for peak in _terrain._mountain_peaks:
		var peak_pos: Vector3 = peak["pos"]
		var peak_ix := int((peak_pos.x + half_world) / (float(GameState.WORLD_SIZE) / float(FOG_GRID)))
		var peak_iz := int((peak_pos.z + half_world) / (float(GameState.WORLD_SIZE) / float(FOG_GRID)))
		if peak_ix < 0 or peak_ix >= FOG_GRID or peak_iz < 0 or peak_iz >= FOG_GRID:
			continue
		if not _revealed[peak_iz * FOG_GRID + peak_ix]:
			continue
		var px: float = offset.x + (peak_pos.x + half_world) * scale_factor
		var pz: float = offset.y + (peak_pos.z + half_world) * scale_factor
		var tri := PackedVector2Array([
			Vector2(px, pz - 6),
			Vector2(px + 5, pz + 4),
			Vector2(px - 5, pz + 4),
		])
		draw_colored_polygon(tri, Color("#c0c0c8"))

	# Spawn marker (always visible)
	var sx := offset.x + half_world * scale_factor
	var sz := offset.y + half_world * scale_factor
	draw_circle(Vector2(sx, sz), 4.0, Color("#8fa358"))

	# Player position + direction
	var player_mx := offset.x + (player_pos.x + half_world) * scale_factor
	var player_mz := offset.y + (player_pos.z + half_world) * scale_factor
	var arrow_len := 10.0
	var dir_x := -sin(yaw) * arrow_len
	var dir_z := -cos(yaw) * arrow_len
	var player_center := Vector2(player_mx, player_mz)
	draw_line(player_center, player_center + Vector2(dir_x, dir_z), Color("#f0e0c0"), 2.0)
	draw_circle(player_center, 5.0, Color("#f0d888"))
	draw_circle(player_center, 3.0, Color("#f8e8b0"))

	# Title
	draw_string(ThemeDB.fallback_font, Vector2(offset.x, offset.y - 12), "WORLD MAP  [M to close]", HORIZONTAL_ALIGNMENT_LEFT, -1, 14, Color("#c0b898"))

# How close a revealed cell is to fog — returns 0 at fog edge, 1 fully revealed
func _get_edge_fade(ix: int, iz: int) -> float:
	var min_dist := 3  # cells of fade
	var closest_fog := min_dist
	for dz in range(-min_dist, min_dist + 1):
		for dx in range(-min_dist, min_dist + 1):
			var nx := ix + dx
			var nz := iz + dz
			if nx < 0 or nx >= FOG_GRID or nz < 0 or nz >= FOG_GRID:
				# Edge of map counts as fog
				var d := maxi(absi(dx), absi(dz))
				closest_fog = mini(closest_fog, d)
			elif not _revealed[nz * FOG_GRID + nx]:
				var d := maxi(absi(dx), absi(dz))
				closest_fog = mini(closest_fog, d)
	return float(closest_fog) / float(min_dist)
