extends CharacterBody3D

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------
const LOOK_SENSITIVITY := 0.0022
const MAX_PITCH := PI / 2.0 - 0.04
const HEAD_BASE_Y := 0.75

# ---------------------------------------------------------------------------
# Node references
# ---------------------------------------------------------------------------
@onready var _head: Node3D = $Head
@onready var _camera: Camera3D = $Head/Camera3D

# ---------------------------------------------------------------------------
# Private state
# ---------------------------------------------------------------------------
var _current_speed: float = 0.0
var _bob_phase: float = 0.0
var _bob_blend: float = 0.0
var _was_grounded: bool = true
var _land_dip_offset: float = 0.0
var _last_step_index: int = 0

# ---------------------------------------------------------------------------
# _ready
# ---------------------------------------------------------------------------
func _ready() -> void:
	Input.mouse_mode = Input.MOUSE_MODE_CAPTURED

# ---------------------------------------------------------------------------
# _unhandled_input
# ---------------------------------------------------------------------------
func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion:
		rotate_y(-event.relative.x * LOOK_SENSITIVITY)
		_head.rotation.x = clampf(
			_head.rotation.x - event.relative.y * LOOK_SENSITIVITY,
			-MAX_PITCH, MAX_PITCH
		)

	if event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
		if Input.mouse_mode != Input.MOUSE_MODE_CAPTURED:
			Input.mouse_mode = Input.MOUSE_MODE_CAPTURED

	if event.is_action_pressed("ui_cancel"):
		Input.mouse_mode = Input.MOUSE_MODE_VISIBLE

# ---------------------------------------------------------------------------
# _physics_process
# ---------------------------------------------------------------------------
func _physics_process(delta: float) -> void:
	# ── input ──────────────────────────────────────────────────────────────
	var input_dir := Input.get_vector("move_left", "move_right", "move_forward", "move_back")
	var is_sprinting := Input.is_action_pressed("sprint")
	var has_input := input_dir.length_squared() > 0.001
	var target_speed := 0.0
	if has_input:
		target_speed = GameState.SPRINT_SPEED if is_sprinting else GameState.WALK_SPEED

	# ── acceleration / deceleration ────────────────────────────────────────
	var rate := GameState.MOVE_ACCEL if _current_speed < target_speed else GameState.MOVE_DECEL
	_current_speed += (target_speed - _current_speed) * (1.0 - exp(-rate * delta))
	if _current_speed < 0.08:
		_current_speed = 0.0

	# ── horizontal movement ────────────────────────────────────────────────
	var forward := -transform.basis.z
	forward.y = 0
	forward = forward.normalized()
	var right := transform.basis.x
	right.y = 0
	right = right.normalized()

	# input_dir.y is forward/back; negative = forward in get_vector ordering
	var move_dir: Vector3
	if has_input:
		move_dir = (forward * -input_dir.y + right * input_dir.x).normalized()
	else:
		move_dir = Vector3.ZERO

	velocity.x = move_dir.x * _current_speed
	velocity.z = move_dir.z * _current_speed

	# ── gravity + jump ─────────────────────────────────────────────────────
	var pre_ground_vy := velocity.y
	velocity.y -= GameState.GRAVITY * delta

	if is_on_floor() and Input.is_action_just_pressed("jump"):
		velocity.y = GameState.JUMP_SPEED

	move_and_slide()

	# ── landing camera dip ─────────────────────────────────────────────────
	if is_on_floor() and not _was_grounded:
		var fall_speed := absf(pre_ground_vy)
		_land_dip_offset = minf(fall_speed * GameState.LAND_DIP_SCALE, GameState.LAND_DIP_MAX)
	_was_grounded = is_on_floor()
	_land_dip_offset *= exp(-GameState.LAND_DIP_RECOVERY * delta)

	# ── head bob ───────────────────────────────────────────────────────────
	var moving_on_ground := _current_speed > 0.5 and is_on_floor()
	_bob_blend += ((1.0 if moving_on_ground else 0.0) - _bob_blend) * (1.0 - exp(-12.0 * delta))

	if moving_on_ground:
		var freq := GameState.BOB_SPRINT_FREQ if is_sprinting else GameState.BOB_WALK_FREQ
		_bob_phase += freq * PI * 2.0 * delta

	var vertical_bob := sin(_bob_phase) * GameState.BOB_VERTICAL_AMP * _bob_blend
	var roll_bob := cos(_bob_phase * 0.5) * GameState.BOB_ROLL_AMP * _bob_blend

	# ── apply camera offsets ───────────────────────────────────────────────
	_head.position.y = HEAD_BASE_Y + vertical_bob - _land_dip_offset
	_camera.rotation.z = roll_bob

	# ── fatigue / magicka ──────────────────────────────────────────────────
	var effort: float
	if _current_speed > 0.5:
		effort = 0.5 if is_sprinting else 0.28
	else:
		effort = -0.22
	GameState.fatigue = clampf(GameState.fatigue - effort * delta, 0.22, 1.0)
	GameState.magicka = clampf(GameState.magicka + 0.05 * delta, 0.18, 0.88)

# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------
func respawn_at_spawn(terrain) -> void:
	var spawn_y: float = terrain.sample_height(0.0, 0.0) + GameState.PLAYER_HEIGHT
	global_position = Vector3(0.0, spawn_y, 0.0)
	velocity = Vector3.ZERO
	_current_speed = 0.0
	_bob_phase = 0.0
	_bob_blend = 0.0
	_land_dip_offset = 0.0
	_was_grounded = true
	_camera.rotation.z = 0.0

	# Look toward castle
	rotation.y = atan2(GameState.castle_center.x, GameState.castle_center.z)
	_head.rotation.x = -0.08
