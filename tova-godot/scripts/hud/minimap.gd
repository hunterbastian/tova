extends Control

const MINIMAP_SIZE := 160.0
const MINIMAP_WORLD_RANGE := 1320.0

var _player: CharacterBody3D

var _landmarks := {
	"shrine": { "color": Color("#c4aa69"), "label": "S" },
	"castle": { "color": Color("#8c8b84"), "label": "C" },
	"spawn":  { "color": Color("#8fa358"), "label": "" },
}

func _ready() -> void:
	custom_minimum_size = Vector2(MINIMAP_SIZE, MINIMAP_SIZE)
	size = Vector2(MINIMAP_SIZE, MINIMAP_SIZE)

func set_player(player: CharacterBody3D) -> void:
	_player = player

func _process(_delta: float) -> void:
	queue_redraw()

func _draw() -> void:
	if not _player:
		return

	var half := MINIMAP_SIZE / 2.0
	var scale_factor := MINIMAP_SIZE / MINIMAP_WORLD_RANGE
	var player_pos := _player.global_position
	var yaw := _player.rotation.y

	# Circular clip — dark background
	draw_circle(Vector2(half, half), half - 1.0, Color(0.035, 0.031, 0.027, 0.82))

	# Terrain boundary ring
	draw_arc(Vector2(half, half), (MINIMAP_WORLD_RANGE / 2.0) * scale_factor, 0, TAU, 64, Color(0.737, 0.643, 0.435, 0.15), 1.0)

	var cos_y := cos(-yaw)
	var sin_y := sin(-yaw)

	# Draw landmarks
	_draw_landmark(half, scale_factor, player_pos, cos_y, sin_y,
		GameState.sword_pickup_position, _landmarks["shrine"])
	_draw_landmark(half, scale_factor, player_pos, cos_y, sin_y,
		GameState.castle_center, _landmarks["castle"])
	_draw_landmark(half, scale_factor, player_pos, cos_y, sin_y,
		Vector3.ZERO, _landmarks["spawn"])

	# Player arrow at center
	var arrow_points := PackedVector2Array([
		Vector2(half, half - 6),
		Vector2(half + 4, half + 5),
		Vector2(half, half + 2),
		Vector2(half - 4, half + 5),
	])
	draw_colored_polygon(arrow_points, Color("#e6dcc7"))

	# North indicator
	var north_angle := -yaw - PI / 2.0
	var nx := half + cos(north_angle) * (half - 10.0)
	var ny := half + sin(north_angle) * (half - 10.0)
	draw_string(ThemeDB.fallback_font, Vector2(nx - 4, ny + 4), "N", HORIZONTAL_ALIGNMENT_CENTER, -1, 9, Color("#c4aa69"))

func _draw_landmark(half: float, scale_factor: float, player_pos: Vector3, cos_y: float, sin_y: float, landmark_pos: Vector3, info: Dictionary) -> void:
	var dx := landmark_pos.x - player_pos.x
	var dz := landmark_pos.z - player_pos.z

	# Rotate so forward faces up
	var rx := dx * cos_y - dz * sin_y
	var rz := dx * sin_y + dz * cos_y

	var sx := half + rx * scale_factor
	var sy := half - rz * scale_factor

	# Skip if outside circle
	var from_center := Vector2(sx - half, sy - half).length()
	if from_center > half - 4.0:
		return

	# Diamond marker
	var diamond := PackedVector2Array([
		Vector2(sx, sy - 3.5),
		Vector2(sx + 3.5, sy),
		Vector2(sx, sy + 3.5),
		Vector2(sx - 3.5, sy),
	])
	draw_colored_polygon(diamond, info["color"])

	# Label
	if info["label"] != "":
		draw_string(ThemeDB.fallback_font, Vector2(sx - 4, sy - 7), info["label"], HORIZONTAL_ALIGNMENT_CENTER, -1, 8, Color(0.9, 0.86, 0.78, 0.7))
