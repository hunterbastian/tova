extends Node

# ---------------------------------------------------------------------------
# Constants (ported from the Three.js version)
# ---------------------------------------------------------------------------
const WORLD_SIZE := 220
const WORLD_SEGMENTS := 110
const SPAWN_RADIUS := 14
const SPAWN_BLEND_RADIUS := 30
const PLAYER_HEIGHT := 1.8
const WALK_SPEED := 6.1
const SPRINT_SPEED := 8.7
const GRAVITY := 24.0
const JUMP_SPEED := 8.8
const MOVE_ACCEL := 14.0
const MOVE_DECEL := 10.0
const BOB_WALK_FREQ := 1.8
const BOB_SPRINT_FREQ := 2.4
const BOB_VERTICAL_AMP := 0.044
const BOB_ROLL_AMP := 0.006
const LAND_DIP_SCALE := 0.012
const LAND_DIP_MAX := 0.14
const LAND_DIP_RECOVERY := 8.0

# ---------------------------------------------------------------------------
# Signals
# ---------------------------------------------------------------------------
signal world_regenerated(seed_value: int)
signal player_damaged(amount: float)
signal player_died
signal player_respawned
signal sword_taken
signal status_changed(message: String)

# ---------------------------------------------------------------------------
# Mutable state
# ---------------------------------------------------------------------------
var seed_value: int = 0
var mode: String = "intro"
var health: float = 1.0
var magicka: float = 0.88
var fatigue: float = 0.84
var has_sword: bool = false
var kills: int = 0
var is_dead: bool = false
var forest_center: Vector3 = Vector3.ZERO
var castle_center: Vector3 = Vector3.ZERO
var sword_pickup_position: Vector3 = Vector3.ZERO

# ---------------------------------------------------------------------------
# reset_state — restore all mutable vars to their defaults
# ---------------------------------------------------------------------------
func reset_state() -> void:
	seed_value = 0
	mode = "intro"
	health = 1.0
	magicka = 0.88
	fatigue = 0.84
	has_sword = false
	kills = 0
	is_dead = false
	forest_center = Vector3.ZERO
	castle_center = Vector3.ZERO
	sword_pickup_position = Vector3.ZERO
