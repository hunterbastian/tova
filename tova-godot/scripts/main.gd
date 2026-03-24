extends Node3D

var _environment: Node3D
var _terrain: MeshInstance3D
var _structures: Node3D
var _player: CharacterBody3D

func _ready() -> void:
	_setup_scene_tree()
	# Defer first world gen so all scripts are fully initialized
	call_deferred("regenerate_world")

func _setup_scene_tree() -> void:
	# Create nodes using script's new() so the script is baked in from creation
	var EnvScript := preload("res://scripts/environment/environment_setup.gd")
	var TerrainScript := preload("res://scripts/world/terrain_generator.gd")
	var StructureScript := preload("res://scripts/world/structure_builder.gd")

	# Environment
	_environment = EnvScript.new()
	_environment.name = "Environment"
	add_child(_environment)

	# World container
	var world := Node3D.new()
	world.name = "World"
	add_child(world)

	# Terrain
	_terrain = TerrainScript.new()
	_terrain.name = "TerrainMesh"
	world.add_child(_terrain)

	# Structures
	_structures = StructureScript.new()
	_structures.name = "Structures"
	world.add_child(_structures)

	# Player (CharacterBody3D)
	_player = CharacterBody3D.new()
	_player.name = "Player"
	_player.set_script(load("res://scripts/player/player_controller.gd"))

	# Player collision shape
	var col_shape := CollisionShape3D.new()
	var capsule := CapsuleShape3D.new()
	capsule.radius = 0.18
	capsule.height = 0.9
	col_shape.shape = capsule
	_player.add_child(col_shape)

	# Player head + camera
	var head := Node3D.new()
	head.name = "Head"
	head.position.y = 0.75
	_player.add_child(head)

	var camera := Camera3D.new()
	camera.name = "Camera3D"
	camera.fov = 72.0
	camera.near = 0.1
	camera.far = 600.0
	head.add_child(camera)

	add_child(_player)

func _unhandled_input(event: InputEvent) -> void:
	if event.is_action_pressed("regenerate"):
		regenerate_world()

func regenerate_world() -> void:
	var seed_val := randi()

	GameState.reset_state()
	GameState.seed_value = seed_val

	# Clear existing world
	_terrain.clear_terrain()
	_structures.clear_structures()

	# Generate new world — nature only
	_terrain.generate_terrain(seed_val)
	_structures.build_forest(seed_val, _terrain)
	_structures.build_haze(seed_val, _terrain)

	# Position environment lights
	_environment.update_spawn_light(_terrain.sample_height(4.0, 0.0))

	# Spawn player
	_player.respawn_at_spawn(_terrain)

	GameState.world_regenerated.emit(seed_val)
	GameState.status_changed.emit("Green rise ahead. The keep waits in the distance.")
