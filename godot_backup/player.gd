extends CharacterBody3D

@onready var cam: Camera3D = $Camera3D
@onready var torch: OmniLight3D = $Torch
@onready var world: Node = get_node("../World")

var look_sens := 0.12
var speed := 6.0
var sprint_bonus := 3.0
var gravity := ProjectSettings.get_setting("physics/3d/default_gravity")
var jump_vel := 10.5

var vel: Vector3
var yaw := 0.0
var pitch := 0.0
var mouse_captured := false
var current_block := "wood"

func _ready() -> void:
	Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)
	mouse_captured = true
	torch.light_energy = 0.0

func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion and mouse_captured:
		yaw -= event.relative.x * look_sens * 0.01
		pitch -= event.relative.y * look_sens * 0.01
		pitch = clamp(pitch, deg_to_rad(-85), deg_to_rad(85))
		rotation.y = yaw
		cam.rotation.x = pitch

	if event.is_action_pressed("toggle_torch"):
		torch.light_energy = (torch.light_energy > 0.0) ? 0.0 : 2.2
	if event.is_action_pressed("toggle_overcast"):
		get_node("../Sun").call("toggle_overcast")

	for i in range(1, 6):
		if event.is_action_pressed("hotbar_%d" % i):
			current_block = ["wood","plank","thatch","stone","dirt"][i-1]

	if event.is_action_pressed("break_block"):
		_break_block()
	if event.is_action_pressed("place_block"):
		_place_block()

func _physics_process(dt: float) -> void:
	var dir := Vector3.ZERO
	var forward := -cam.global_transform.basis.z; forward.y = 0; forward = forward.normalized()
	var right := cam.global_transform.basis.x; right.y = 0; right = right.normalized()

	if Input.is_action_pressed("move_forward"): dir += forward
	if Input.is_action_pressed("move_back"):    dir -= forward
	if Input.is_action_pressed("move_left"):    dir -= right
	if Input.is_action_pressed("move_right"):   dir += right
	if dir.length() > 0.0: dir = dir.normalized()

	var spd = speed + (Input.is_action_pressed("sprint") ? sprint_bonus : 0.0)
	vel.x = dir.x * spd
	vel.z = dir.z * spd

	if not is_on_floor():
		vel.y -= gravity * dt
	else:
		if Input.is_action_just_pressed("jump"):
			vel.y = jump_vel
		else:
			vel.y = -2.0

	velocity = vel
	move_and_slide()

	# keep torch near camera (lantern vibe)
	torch.global_transform.origin = cam.global_transform.origin + cam.global_transform.basis.x * 0.25 - cam.global_transform.basis.z * 0.5 - Vector3(0,0.4,0)

func _cast_from_center(max_dist := 6.0) -> Dictionary:
	var space := get_world_3d().direct_space_state
	var from := cam.project_ray_origin(get_viewport().get_visible_rect().size/2)
	var to := from + cam.project_ray_normal(get_viewport().get_visible_rect().size/2) * max_dist
	var res := space.intersect_ray(PhysicsRayQueryParameters3D.create(from, to))
	return res

func _break_block() -> void:
	var hit := _cast_from_center()
	if hit.has("collider") and hit.collider and hit.collider.has_method("break_at_hit"):
		hit.collider.break_at_hit(hit)

func _place_block() -> void:
	var hit := _cast_from_center()
	if hit.is_empty(): return
	var n: Vector3 = hit.get("normal", Vector3.UP)
	var pos: Vector3 = hit.position + n * 0.5
	world.call("place_block_worldspace", pos, current_block)
