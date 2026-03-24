extends Control

const COMPASS_WIDTH := 300.0
const COMPASS_RANGE := PI * 0.5

var _player: CharacterBody3D

var _directions := [
	{ "label": "N",  "angle": 0.0,             "major": true },
	{ "label": "NE", "angle": -PI / 4.0,       "major": false },
	{ "label": "E",  "angle": -PI / 2.0,       "major": true },
	{ "label": "SE", "angle": -3.0 * PI / 4.0, "major": false },
	{ "label": "S",  "angle": PI,              "major": true },
	{ "label": "SW", "angle": 3.0 * PI / 4.0,  "major": false },
	{ "label": "W",  "angle": PI / 2.0,        "major": true },
	{ "label": "NW", "angle": PI / 4.0,        "major": false },
]

func _ready() -> void:
	custom_minimum_size = Vector2(COMPASS_WIDTH, 24)
	size = Vector2(COMPASS_WIDTH, 24)

func set_player(player: CharacterBody3D) -> void:
	_player = player

func _process(_delta: float) -> void:
	queue_redraw()

func _normalize_angle(a: float) -> float:
	var result := a
	while result > PI:
		result -= TAU
	while result < -PI:
		result += TAU
	return result

func _draw() -> void:
	if not _player:
		return

	var yaw := _player.rotation.y
	var half := COMPASS_WIDTH / 2.0

	# Notch at center
	draw_line(Vector2(half, 0), Vector2(half, 6), Color("#c4aa69"), 1.5)

	# Direction markers
	for dir in _directions:
		var rel := _normalize_angle(dir["angle"] - yaw)
		if absf(rel) < COMPASS_RANGE:
			var px := half - (rel / COMPASS_RANGE) * half
			var color: Color
			if dir["label"] == "N":
				color = Color("#c4aa69")
			elif dir["major"]:
				color = Color(0.9, 0.86, 0.78, 0.8)
			else:
				color = Color(0.9, 0.86, 0.78, 0.4)
			var font_size := 11 if dir["major"] else 9
			draw_string(ThemeDB.fallback_font, Vector2(px - 6, 20), dir["label"], HORIZONTAL_ALIGNMENT_LEFT, -1, font_size, color)
