# TOVA Godot Port — Phase 1 Design

Port TOVA from Three.js (`tova-web/`) to Godot 4.6 (`tova-godot/`). Phase 1 covers the core loop: first-person controller, procedural terrain, world structures, collision, and environment. Combat, UI, audio, and post-processing are deferred to later phases.

## Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Language | GDScript (C#-ready structure) | Already familiar, tighter engine integration |
| Rendering | Native Forward+ first, retro shader toggle later | Get the game working before styling |
| Scope | Core loop only (controller + terrain + collision) | Foundation everything else builds on |
| Terrain | ArrayMesh + FastNoiseLite | 1:1 translation of existing noise-based heightmap |
| Architecture | Scene-per-system with signal communication | Godot-native, modular, testable |

## Project Structure

```
tova-godot/
├── project.godot
├── scenes/
│   ├── main.tscn                  # Root — composes all systems
│   ├── player/
│   │   └── player.tscn            # CharacterBody3D + Camera3D
│   ├── world/
│   │   └── world.tscn             # Terrain + structures + props
│   └── environment/
│       └── environment.tscn       # Sky, lights, fog, tonemap
├── scripts/
│   ├── autoload/
│   │   └── game_state.gd          # Singleton: constants + shared state + signals
│   ├── player/
│   │   └── player_controller.gd
│   ├── world/
│   │   ├── terrain_generator.gd
│   │   └── structure_builder.gd
│   └── environment/
│       └── environment_setup.gd
└── assets/
    └── (empty — all procedural)
```

## System Designs

### GameState Autoload (`game_state.gd`)

Singleton registered in project.godot. Holds constants, mutable game state, and cross-system signals.

**Constants** (ported from `constants.js`):

```
WORLD_SIZE = 220        WORLD_SEGMENTS = 110
SPAWN_RADIUS = 14       SPAWN_BLEND_RADIUS = 30
PLAYER_HEIGHT = 1.8     WALK_SPEED = 6.1      SPRINT_SPEED = 8.7
GRAVITY = 24            JUMP_SPEED = 8.8
MOVE_ACCEL = 14         MOVE_DECEL = 10
BOB_WALK_FREQ = 1.8     BOB_SPRINT_FREQ = 2.4
BOB_VERTICAL_AMP = 0.044  BOB_ROLL_AMP = 0.006
LAND_DIP_SCALE = 0.012  LAND_DIP_MAX = 0.14   LAND_DIP_RECOVERY = 8
```

**State** (reset on world regeneration):

```
seed: int
mode: String              # "intro" | "ready"
health: float = 1.0
magicka: float = 0.88
fatigue: float = 0.84
has_sword: bool = false
kills: int = 0
is_dead: bool = false
forest_center: Vector3
castle_center: Vector3
sword_pickup_position: Vector3
```

**Signals:**

```gdscript
signal world_regenerated(seed: int)
signal player_damaged(amount: float)
signal player_died
signal player_respawned
signal sword_taken
signal status_changed(message: String)
```

### Player System

**Scene tree:**

```
Player (CharacterBody3D)
├── CollisionShape3D (CapsuleShape3D, height=1.8, radius=0.35)
├── Head (Node3D, y=1.6)
│   └── Camera3D (fov=72, near=0.1, far=600)
```

**`player_controller.gd`** — attached to the Player node.

Input handling:
- `_unhandled_input()`: mouse motion for look (sensitivity 0.0022, pitch clamped to ~89 degrees)
- Mouse captured on click via `Input.mouse_mode = Input.MOUSE_MODE_CAPTURED`, released on Escape
- The Three.js version has a `walkMode` fallback for when pointer lock fails — this is not needed in Godot since `MOUSE_MODE_CAPTURED` works reliably across platforms

Movement in `_physics_process(delta)`:
- Read WASD via Input actions, compute forward/right from camera yaw
- Acceleration/deceleration: `current_speed += (target - current_speed) * (1 - exp(-rate * delta))`
- Apply velocity via `velocity` property, call `move_and_slide()`
- Gravity: `velocity.y -= GRAVITY * delta`, floor detection via `is_on_floor()`
- Jump: set `velocity.y = JUMP_SPEED` when grounded + space pressed

Head bob:
- Same sine-based formula: vertical = `sin(phase) * AMP * blend`, roll = `cos(phase * 0.5) * ROLL_AMP * blend`
- Phase advances at `BOB_WALK_FREQ` or `BOB_SPRINT_FREQ` based on sprint state
- Vertical bob and landing dip applied to the Head node's `position.y` (not the CharacterBody3D root)
- Roll applied to Camera3D's `rotation.z`

Landing dip:
- On transition from airborne to grounded, set `land_dip_offset = min(fall_speed * SCALE, MAX)`
- Recover each frame: `land_dip_offset *= exp(-RECOVERY * delta)`

Fatigue/magicka:
- Same drain formula: effort based on speed, clamp to ranges

**Key difference from Three.js:** `CharacterBody3D.move_and_slide()` handles collision with static bodies automatically. No manual collision resolution system needed — colliders are placed directly on world objects.

### Terrain Generation (`terrain_generator.gd`)

Attached to a `MeshInstance3D` in world.tscn.

**Noise setup:**

The Three.js version uses `ImprovedNoise` (classic Perlin) sampled as 3D noise with fixed Y values. `FastNoiseLite` must be configured to match:

- 3 separate `FastNoiseLite` instances, all `TYPE_PERLIN`
- **Broad:** frequency `1/70 = 0.0143`, sampled as `noise_3d(x, 0.15, z)`, amplitude 10
- **Hills:** frequency `1/24 = 0.0417`, sampled as `noise_3d(x, 0.32, z)`, amplitude 5
- **Ridge:** frequency `1/11 = 0.0909`, sampled as `noise_3d(x, 0.52, z)`, amplitude 2.1, uses separate XZ offset (`ridge_offset`)
- Each instance seeded via `noise.seed = seed`
- XZ offsets (`offset_x`, `offset_z`) are derived from a seeded RNG, not baked into the noise seed

**Note:** `FastNoiseLite` output may differ slightly from `ImprovedNoise`. Seed compatibility with the Three.js version is not a goal — the terrain should look similar in character (rolling hills, flat spawn, castle plateau) but doesn't need to produce identical heightmaps for the same seed. Godot's `RandomNumberGenerator` replaces `mulberry32` for deterministic placement of structures and props.

**`generate_terrain(seed: int)` method:**

1. Create `SurfaceTool`, begin `PRIMITIVE_TRIANGLES`
2. Build a grid of `(WORLD_SEGMENTS + 1)^2` vertices
3. For each vertex, call `sample_height(x, z)` — same algorithm as Three.js version:
   - Sum broad + hills + ridge noise
   - Add peak lift, forest lift, castle lift (smootherstep blended)
   - Flatten spawn area and castle plateau
   - Carve view lane from spawn to castle
4. Assign vertex colors from the terrain palette:
   - Grass: `#c4a56e` (default)
   - Spawn: `#d4b87a` (within `SPAWN_BLEND_RADIUS + 8`)
   - Forest: `#8a7256` (within 26 units of forest center)
   - Highland: `#9c8872` (height > 20)
   - Slope: `#b09878` (height > 15)
   - Dry: `#c8a878` (moisture < 0.32, from a separate noise sample)
5. Generate triangle indices, compute normals
6. Commit to `ArrayMesh`, assign to `MeshInstance3D.mesh`

**Terrain context state:** The `terrain_generator.gd` script stores the following as member variables after generation (mirroring the closure state in Three.js `buildTerrainContext()`):
- `_offset_x`, `_offset_z`: random XZ offsets for noise sampling
- `_ridge_offset`: separate offset for ridge noise layer
- `_forest_center`, `_castle_center`, `_mountain_peak`: `Vector3` zone centers
- `_rng`: `RandomNumberGenerator` instance seeded from the world seed

**`sample_height(x: float, z: float) -> float`** — public method that uses the stored context. Called by player (ground check), structure builder (placement), and future AI system.

**Material:** `StandardMaterial3D` with `vertex_color_use_as_albedo = true`, `roughness = 0.96`, `metalness = 0.02`. No texture needed.

### Structure Builder (`structure_builder.gd`)

Attached to a child node of world.tscn. Called after terrain is generated.

**`build_spawn_sanctum(seed, terrain)`:**
- Shrine group: dais (CylinderMesh), altar (BoxMesh), stele (BoxMesh), arch posts + cap
- Two braziers flanking the shrine: each is a group of bowl (CylinderMesh `#50443a`), stem (CylinderMesh `#70645a`), flame (SphereMesh `#f6c56d` with unshaded material), and `OmniLight3D` (color `#f0bf63`, energy 1.2, range 13, attenuation 2)
- Each mesh piece wrapped in `StaticBody3D` + `CollisionShape3D` (replaces manual `addBox`/`addCylinder`)
- Path stones from spawn to shrine
- Scatter rocks: build a dodecahedron via `SurfaceTool`/`ArrayMesh` (12 pentagonal faces → 36 triangles) to preserve the angular, faceted look. A low-poly sphere is too round. Alternatively use an `IcoSphereMesh` with 0 subdivisions as a closer approximation. Apply random scale and rotation per instance.
- Instanced grass: `MultiMeshInstance3D` with 180 instances (ConeMesh), same scatter pattern

**`build_forest(seed, terrain)`:**
- Two `MultiMeshInstance3D` nodes: trunks (CylinderMesh, 220 instances) and canopies (ConeMesh, 220 instances)
- Same placement logic: polar scatter around `forest_center`, skip spawn/castle zones
- Each tree trunk base gets a `StaticBody3D` + `CylinderShape3D` for collision
- Scatter rocks around forest edges

**`build_castle(seed, terrain)`:**
- Courtyard, walls, gate, keep as BoxMesh nodes
- 4 corner towers as CylinderMesh + ConeGeometry roofs
- Each structural piece gets `StaticBody3D` + collision shape
- Gate: a dark BoxMesh (`#3f3933`) overlaid on the front wall as a visual gate piece. The front wall collision is split into two segments with a gap for player passage (left half-width 4.225, right half-width 4.225, gate gap 5.1 wide)

**`build_haze(seed, terrain)`:**
- Mist: SphereMesh with transparent material (opacity 0.18), flattened scale
- Obelisk, ruins as simple mesh groups

**Materials:** All use `StandardMaterial3D` with flat shading. Same hex colors from Three.js:
- Rock: `#6e6a63`, Castle wall: `#68675f`, Roof: `#3f3933`
- Shrine stone: `#84796f`, Tree trunk: `#5a4838`, Canopy: `#7a8060`

### Environment (`environment.tscn`)

**Scene tree:**

```
Environment (Node3D)
├── WorldEnvironment
│   └── Environment resource
├── DirectionalLight3D (sun)
├── OmniLight3D (spawn fill light)
└── MeshInstance3D (moon — decorative)
```

**Environment resource settings:**
- Background: `Sky` with `ProceduralSkyMaterial`. The Three.js Sky uses turbidity=14, rayleigh=3.2, mieCoefficient=0.03, mieDirectionalG=0.92. Godot's `ProceduralSkyMaterial` uses different params — tune `sky_top_color`, `sky_horizon_color`, `ground_bottom_color` to match the warm golden-hour atmosphere. Start with warm oranges/yellows at horizon, pale blue at zenith.
- Fog: `fog_mode = FOG_MODE_EXPONENTIAL` (matches Three.js `FogExp2`), density 0.012, color `#d8c8b8`
- Tonemap: Reinhard, exposure 1.35
- Ambient light: color `#c8a878`, energy 1.1
- Ambient mode: `AMBIENT_LIGHT_COLOR` combined with sky contribution to approximate the hemisphere light (sky `#d8c0a0`, ground `#8a7060`, energy 1.2)

**DirectionalLight3D (sun):**
- Position: `Vector3(88, 132, -24)` (this is the light position, not the sky sun direction)
- Sky sun direction (for ProceduralSkyMaterial): computed from spherical coords `(1, PI * 0.47, PI * 0.12)`
- Color `#f0c890`, energy 2.0
- Shadows enabled, shadow map size 2048x2048
- Shadow frustum: orthogonal size 140 (covers -140 to 140), near 1, far 280 (not camera far of 600)

**OmniLight3D (spawn fill light):**
- Color `#e0c898`, energy 1.6, range 52, attenuation 2
- Position: `Vector3(4, sample_height(4, 0) + 7.5, 8)` — set after terrain generation

**Moon (decorative):**
- SphereMesh, radius 7, 24 segments, unshaded material color `#e8d8c0`
- Position: `Vector3(-110, 92, -210)`

**All materials use `shading_mode = SHADING_MODE_PER_PIXEL` with `flat_shading = true`** unless otherwise noted. This is a core part of the visual identity — applies to rocks, shrine, castle, trees, terrain, and all structures.

### Main Scene (`main.tscn`)

Composes everything:

```
Main (Node3D)
├── environment.tscn (instance)
├── world.tscn (instance)
└── player.tscn (instance)
```

**`main.gd`** — minimal orchestrator:
- `_ready()`: connect signals, call initial world generation
- `regenerate_world()`: reset state, rebuild terrain + structures, respawn player
- R key triggers regeneration

## Godot Project Configuration (`project.godot`)

```ini
config/name="TOVA"
run/main_scene="res://scenes/main.tscn"
config/features=PackedStringArray("4.6", "Forward Plus")

[autoload]
GameState="*res://scripts/autoload/game_state.gd"

[input]
move_forward = W
move_back = S
move_left = A
move_right = D
jump = Space
sprint = Shift
interact = E
regenerate = R

[display]
window/size/viewport_width=1920
window/size/viewport_height=1080
```

## Porting Mapping

| Three.js | Godot | Notes |
|----------|-------|-------|
| `PointerLockControls` | `Input.mouse_mode = CAPTURED` | Built-in |
| `PlaneGeometry` + vertex displacement | `SurfaceTool` → `ArrayMesh` | Same vertex loop |
| `ImprovedNoise` | `FastNoiseLite` | Slightly different output, tune params |
| `InstancedMesh` | `MultiMeshInstance3D` | Same instancing concept |
| `MeshStandardMaterial` | `StandardMaterial3D` | Direct mapping |
| `collisionSystem.resolve()` | `CharacterBody3D.move_and_slide()` | Godot handles it |
| `addCylinder`/`addBox` | `StaticBody3D` + `CollisionShape3D` | On each world object |
| `Raycaster` | `PhysicsRayQueryParameters3D` | For future weapon hits |
| `THREE.FogExp2` | `Environment.fog_density` | Built-in |
| `Sky` + uniforms | `ProceduralSkyMaterial` | Built-in |
| `performance.now()` delta | `_physics_process(delta)` | Engine-provided |
| callback wiring in `main.js` | Godot signals | `GameState.signal.connect()` |

## Deferred Phases

| Phase | Systems | Approach |
|-------|---------|----------|
| Phase 2 | Combat (actors + weapon + interactables) | `actors.tscn` with skeleton FSM, `Area3D` for weapon hit, `Area3D` for interactables |
| Phase 3 | HUD/UI + procedural audio | `Control` node tree, `AudioStreamGenerator` |
| Phase 4 | Retro post-processing | `SubViewport` + pixel shader + color banding shader, toggle in settings |

## Success Criteria

Phase 1 is complete when:
1. Player can walk, sprint, jump on procedural terrain with head bob and landing dip
2. Terrain generates from seed with correct zone shaping (spawn flat, forest, castle plateau)
3. Shrine, forest (220 trees), castle, rocks, grass, haze all render with correct materials
4. Collision works — player can't walk through castle walls, trees, shrine, rocks
5. R key regenerates world with new seed
6. Environment looks warm and atmospheric (fog, sky, lighting match Three.js mood)
7. Mouse capture works (click to capture, Escape to release)
