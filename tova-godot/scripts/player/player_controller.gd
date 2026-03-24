extends CharacterBody3D

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------
const LOOK_SENSITIVITY := 0.0022
const MAX_PITCH := PI / 2.0 - 0.04
const HEAD_BASE_Y := 0.75
const SWORD_PICKUP_RADIUS := 3.2

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
var _camera_sway_time: float = 0.0

# Sword viewmodel
var _sword_group: Node3D
var _sword_anchor: Node3D
var _sword_swing: float = 0.0
const SWORD_SWING_DURATION := 0.28

# ---------------------------------------------------------------------------
# _ready
# ---------------------------------------------------------------------------
func _ready() -> void:
	Input.mouse_mode = Input.MOUSE_MODE_CAPTURED
	_setup_sword_viewmodel()

# ---------------------------------------------------------------------------
# Sword viewmodel — visible when has_sword is true
# ---------------------------------------------------------------------------
func _setup_sword_viewmodel() -> void:
	_sword_anchor = Node3D.new()
	_sword_anchor.name = "SwordAnchor"
	_sword_anchor.position = Vector3(0.38, -0.35, -0.5)
	_camera.add_child(_sword_anchor)

	_sword_group = Node3D.new()
	_sword_group.name = "Sword"
	_sword_group.scale = Vector3.ONE * 0.24
	_sword_group.rotation = Vector3(-0.18, -0.14, 0.46)
	_sword_group.visible = false

	var steel_mat := StandardMaterial3D.new()
	steel_mat.albedo_color = Color("#cbc6bb")
	steel_mat.roughness = 0.34
	steel_mat.metallic = 0.82

	var guard_mat := StandardMaterial3D.new()
	guard_mat.albedo_color = Color("#8f7444")
	guard_mat.roughness = 0.58
	guard_mat.metallic = 0.46

	var grip_mat := StandardMaterial3D.new()
	grip_mat.albedo_color = Color("#3c2f28")
	grip_mat.roughness = 0.92
	grip_mat.metallic = 0.08

	# Blade
	var blade_mesh := BoxMesh.new()
	blade_mesh.size = Vector3(0.12, 2.55, 0.05)
	var blade := MeshInstance3D.new()
	blade.mesh = blade_mesh
	blade.material_override = steel_mat
	blade.position.y = 1.4
	_sword_group.add_child(blade)

	# Tip
	var tip_mesh := CylinderMesh.new()
	tip_mesh.top_radius = 0.0
	tip_mesh.bottom_radius = 0.12
	tip_mesh.height = 0.36
	tip_mesh.radial_segments = 4
	var tip := MeshInstance3D.new()
	tip.mesh = tip_mesh
	tip.material_override = steel_mat
	tip.position.y = 2.82
	tip.rotation.z = PI
	_sword_group.add_child(tip)

	# Guard
	var guard_mesh := BoxMesh.new()
	guard_mesh.size = Vector3(0.62, 0.08, 0.12)
	var guard := MeshInstance3D.new()
	guard.mesh = guard_mesh
	guard.material_override = guard_mat
	guard.position.y = 0.12
	_sword_group.add_child(guard)

	# Grip
	var grip_mesh := CylinderMesh.new()
	grip_mesh.top_radius = 0.06
	grip_mesh.bottom_radius = 0.075
	grip_mesh.height = 0.62
	grip_mesh.radial_segments = 8
	var grip := MeshInstance3D.new()
	grip.mesh = grip_mesh
	grip.material_override = grip_mat
	grip.position.y = -0.25
	_sword_group.add_child(grip)

	# Pommel
	var pommel_mesh := SphereMesh.new()
	pommel_mesh.radius = 0.12
	pommel_mesh.height = 0.24
	var pommel := MeshInstance3D.new()
	pommel.mesh = pommel_mesh
	pommel.material_override = guard_mat
	pommel.position.y = -0.62
	_sword_group.add_child(pommel)

	_sword_anchor.add_child(_sword_group)

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
		elif GameState.has_sword and _sword_swing <= 0:
			_sword_swing = SWORD_SWING_DURATION

	if event.is_action_pressed("ui_cancel"):
		Input.mouse_mode = Input.MOUSE_MODE_VISIBLE

	# Sword pickup
	if event.is_action_pressed("interact"):
		_try_pickup_sword()

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

	# ── head bob (heavier for Elder Scrolls feel) ──────────────────────────
	var moving_on_ground := _current_speed > 0.5 and is_on_floor()
	_bob_blend += ((1.0 if moving_on_ground else 0.0) - _bob_blend) * (1.0 - exp(-8.0 * delta))

	if moving_on_ground:
		var freq := GameState.BOB_SPRINT_FREQ if is_sprinting else GameState.BOB_WALK_FREQ
		_bob_phase += freq * PI * 2.0 * delta

	# Stronger bob amplitudes — weightier movement
	var vertical_bob := sin(_bob_phase) * GameState.BOB_VERTICAL_AMP * 1.8 * _bob_blend
	var roll_bob := cos(_bob_phase * 0.5) * GameState.BOB_ROLL_AMP * 2.0 * _bob_blend

	# ── camera sway (idle breathing + movement weight) ─────────────────────
	_camera_sway_time += delta
	var idle_sway_y := sin(_camera_sway_time * 0.8) * 0.003
	var idle_sway_x := sin(_camera_sway_time * 0.5) * 0.002

	# ── apply camera offsets ───────────────────────────────────────────────
	_head.position.y = HEAD_BASE_Y + vertical_bob - _land_dip_offset + idle_sway_y
	_camera.rotation.z = roll_bob + idle_sway_x

	# ── sword viewmodel update ─────────────────────────────────────────────
	_update_sword(delta)

	# ── fatigue / magicka ──────────────────────────────────────────────────
	var effort: float
	if _current_speed > 0.5:
		effort = 0.5 if is_sprinting else 0.28
	else:
		effort = -0.22
	GameState.fatigue = clampf(GameState.fatigue - effort * delta, 0.22, 1.0)
	GameState.magicka = clampf(GameState.magicka + 0.05 * delta, 0.18, 0.88)

# ---------------------------------------------------------------------------
# Sword
# ---------------------------------------------------------------------------
func _try_pickup_sword() -> void:
	if GameState.has_sword:
		return
	var dist := global_position.distance_to(GameState.sword_pickup_position)
	if dist < SWORD_PICKUP_RADIUS:
		GameState.has_sword = true
		_sword_group.visible = true
		GameState.sword_taken.emit()
		GameState.status_changed.emit("Iron sword taken")

func _update_sword(delta: float) -> void:
	_sword_group.visible = GameState.has_sword
	if not GameState.has_sword:
		return

	_sword_swing = maxf(0.0, _sword_swing - delta)
	var swing_progress := 0.0
	if _sword_swing > 0:
		swing_progress = 1.0 - _sword_swing / SWORD_SWING_DURATION
	var slash_arc := sin(swing_progress * PI)

	# Weapon sway from movement
	var move_amount := 1.0 if velocity.length_squared() > 0.5 else 0.0
	var bob_time := _camera_sway_time * 8.0
	var sway_x := sin(bob_time) * 0.008 * move_amount
	var sway_y := absf(cos(bob_time)) * 0.005 * move_amount

	_sword_anchor.position = Vector3(0.38 + sway_x, -0.35 + sway_y, -0.5)
	_sword_anchor.rotation = Vector3(
		-0.44 + slash_arc * 0.48,
		-0.12 - slash_arc * 0.18,
		0.52 - slash_arc * 0.92
	)

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
	_camera_sway_time = 0.0

	# Look toward castle
	rotation.y = atan2(GameState.castle_center.x, GameState.castle_center.z)
	_head.rotation.x = -0.08
