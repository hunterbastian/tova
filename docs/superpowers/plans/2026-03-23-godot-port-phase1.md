# TOVA Godot Port — Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port TOVA's core loop (FPS controller + procedural terrain + world structures + environment) from Three.js to Godot 4.6.

**Architecture:** Scene-per-system composition with one GameState autoload for shared constants/state/signals. Each system is its own .tscn scene with an attached GDScript. Main scene composes them all.

**Tech Stack:** Godot 4.6, GDScript, Forward+ renderer, FastNoiseLite, SurfaceTool/ArrayMesh, MultiMeshInstance3D.

**Spec:** `docs/superpowers/specs/2026-03-23-godot-port-design.md`

**Source reference:** `tova-web/src/` (Three.js version being ported)

**Verification:** Godot projects don't have a CLI test runner. Each task is verified by launching the project in Godot and confirming behavior. Use `print()` for debug output visible in the Godot console.

---

## File Map

| File | Responsibility |
|------|----------------|
| `tova-godot/project.godot` | Project config, autoloads, input map, display settings |
| `tova-godot/scripts/autoload/game_state.gd` | Constants, mutable state, signals |
| `tova-godot/scripts/player/player_controller.gd` | FPS controller: mouse look, WASD, gravity, jump, head bob, landing dip |
| `tova-godot/scripts/world/terrain_generator.gd` | Noise-based heightmap mesh, vertex colors, `sample_height()` |
| `tova-godot/scripts/world/structure_builder.gd` | Shrine, forest, castle, haze, rocks — all with collision |
| `tova-godot/scripts/environment/environment_setup.gd` | Position spawn fill light after terrain gen |
| `tova-godot/scripts/main.gd` | Orchestrator: wire signals, trigger world gen, handle R key |
| `tova-godot/scenes/player/player.tscn` | CharacterBody3D + CapsuleShape + Head + Camera3D |
| `tova-godot/scenes/world/world.tscn` | MeshInstance3D (terrain) + Node3D (structures) |
| `tova-godot/scenes/environment/environment.tscn` | WorldEnvironment + DirectionalLight3D + OmniLight3D + moon |
| `tova-godot/scenes/main.tscn` | Root — instances player, world, environment |

---

### Task 1: Project Scaffold + GameState Autoload

**Files:**
- Create: `tova-godot/project.godot`
- Create: `tova-godot/scripts/autoload/game_state.gd`

- [ ] **Step 1: Create directory structure**

```bash
mkdir -p ~/Desktop/Code/games/active/tova/tova-godot/{scenes/{player,world,environment},scripts/{autoload,player,world,environment},assets}
```

Also create `tova-godot/.gitignore`:
```
.godot/
.import/
*.import
export_presets.cfg
```

- [ ] **Step 2: Write `game_state.gd`**

Create `tova-godot/scripts/autoload/game_state.gd` with:
- All constants from spec (WORLD_SIZE, PLAYER_HEIGHT, WALK_SPEED, GRAVITY, etc.)
- Mutable state variables (seed, mode, health, magicka, fatigue, has_sword, kills, is_dead, forest_center, castle_center, sword_pickup_position)
- All signals (world_regenerated, player_damaged, player_died, player_respawned, sword_taken, status_changed)
- `reset_state()` function that resets all mutable state to defaults

Include Phase 1 constants (movement, terrain, bob) from `tova-web/src/constants.js` lines 3-30. Combat constants (lines 10-18: SWORD_*, ENEMY_*, PLAYER_DAMAGE_*, PLAYER_RESPAWN_DELAY) and HOTBAR (lines 35-41) are Phase 2 — include state variables for `health`, `magicka`, `fatigue`, `has_sword`, `kills`, `is_dead` as stubs for future phases but don't implement any combat logic.

Reference: `tova-web/src/constants.js` for constant values, lines 61-92 for `createGameState()` state shape.

- [ ] **Step 3: Write `project.godot`**

Create `tova-godot/project.godot` with:
- `config_version=5`
- `config/name="TOVA"`
- `run/main_scene="res://scenes/main.tscn"`
- `config/features=PackedStringArray("4.6", "Forward Plus")`
- Autoload: `GameState="*res://scripts/autoload/game_state.gd"`
- Input actions: move_forward (W + Up), move_back (S + Down), move_left (A + Left), move_right (D + Right), jump (Space), sprint (Shift), interact (E), regenerate (R)
- Display: 1920x1080, stretch mode canvas_items

Reference: `~/Desktop/Code/games/active/path-godot/project.godot` for correct Godot 4.6 input action format. Alternatively, define input actions via the Godot editor's Input Map GUI (Project > Project Settings > Input Map) — this is easier than writing the verbose project.godot format by hand.

- [ ] **Step 4: Verify**

Open `tova-godot/project.godot` in Godot editor. Confirm:
- Project loads without errors
- GameState appears in Autoload list (Project > Project Settings > Autoload)
- Input actions appear in Input Map
- Print `GameState.WORLD_SIZE` from a test script to verify autoload works

- [ ] **Step 5: Commit**

```bash
cd ~/Desktop/Code/games/active/tova
git add tova-godot/
git commit -m "feat(godot): scaffold project with GameState autoload and input map"
```

---

### Task 2: Environment Scene

**Files:**
- Create: `tova-godot/scenes/environment/environment.tscn`
- Create: `tova-godot/scripts/environment/environment_setup.gd`

- [ ] **Step 1: Write `environment_setup.gd`**

Create script with:
- `@onready` references to DirectionalLight3D, OmniLight3D (spawn fill), moon MeshInstance3D
- `_ready()`: configure all light properties, create and assign Environment resource programmatically
- Environment resource: Sky with ProceduralSkyMaterial, fog (exponential, density 0.012, color `#d8c8b8`), tonemap (Reinhard, exposure 1.35), ambient (color `#c8a878`, energy 1.1)
- ProceduralSkyMaterial starting values (tune visually): `sky_top_color = Color("#5a7fa0")`, `sky_horizon_color = Color("#e8c090")`, `ground_bottom_color = Color("#8a7060")`, `ground_horizon_color = Color("#d8c8b8")`. The sun direction comes automatically from the DirectionalLight3D's rotation — do NOT set a separate sun direction property.
- Ambient mode: `AMBIENT_LIGHT_COLOR` combined with sky contribution approximates both the Three.js AmbientLight (`#c8a878`, energy 1.1) and HemisphereLight (sky `#d8c0a0`, ground `#8a7060`, energy 1.2). A single ambient color + sky ambient is sufficient.
- DirectionalLight3D: rotate to look from position `(88, 132, -24)` toward the origin — use `look_at_from_position(Vector3(88, 132, -24), Vector3.ZERO)`. Color `#f0c890`, energy 2.0, shadows on, shadow map 2048, `directional_shadow_max_distance = 280`. The ProceduralSkyMaterial picks up the sun direction from this light's `-basis.z` automatically.
- OmniLight3D: color `#e0c898`, energy 1.6, range 52, attenuation 2. Position set via `update_spawn_light(height: float)` called after terrain gen.
- Moon: SphereMesh radius 7, unshaded `#e8d8c0`, position `(-110, 92, -210)`

Reference: `tova-web/src/main.js` lines 44-93 for all light/sky values.

- [ ] **Step 2: Create `environment.tscn`**

Scene tree (can be written as text .tscn or built in editor):
```
Environment (Node3D) [script: environment_setup.gd]
├── WorldEnvironment
├── DirectionalLight3D (name: "Sun")
├── OmniLight3D (name: "SpawnFillLight")
└── MeshInstance3D (name: "Moon")
```

- [ ] **Step 3: Verify**

Create a temporary main scene that instances `environment.tscn`. Run the project:
- Sky renders with warm golden-hour tones
- Fog is visible (warm beige, exponential)
- Sun casts directional shadows
- Moon sphere visible in the distance

- [ ] **Step 4: Commit**

```bash
git add tova-godot/scenes/environment/ tova-godot/scripts/environment/
git commit -m "feat(godot): add environment scene — sky, lights, fog, moon"
```

---

### Task 3: Terrain Generator

**Files:**
- Create: `tova-godot/scripts/world/terrain_generator.gd`
- Create: `tova-godot/scenes/world/world.tscn`

- [ ] **Step 1: Write `terrain_generator.gd` — noise setup and `sample_height()`**

Create script extending `MeshInstance3D` with:
- 3 `FastNoiseLite` instances (broad, hills, ridge), all TYPE_PERLIN
- Frequencies: broad 0.0143, hills 0.0417, ridge 0.0909
- Member variables for terrain context: `_offset_x`, `_offset_z`, `_ridge_offset`, `_forest_center`, `_castle_center`, `_mountain_peak`, `_rng`
- `_build_terrain_context(seed: int)`: initialize RNG, compute offsets and zone centers (same logic as `buildTerrainContext()` in `tova-web/src/world.js` lines 138-206)
- `sample_height(x: float, z: float) -> float`: sum noise layers + zone shaping. Port the height function exactly from `world.js` lines 148-199 (broad + hills + ridge + peak lift + forest lift + castle lift + spawn flatten + castle plateau + view lane)
- `_smootherstep(t: float) -> float`: `t * t * t * (t * (t * 6 - 15) + 10)`

- [ ] **Step 2: Write `terrain_generator.gd` — mesh generation**

Add `generate_terrain(seed: int)` method:
- Call `_build_terrain_context(seed)`
- Use `SurfaceTool` to build grid mesh:
  - `(WORLD_SEGMENTS + 1)^2` vertices in XZ plane spanning `-WORLD_SIZE/2` to `+WORLD_SIZE/2`
  - Each vertex Y = `sample_height(x, z)`
  - Vertex color from palette: check zone in priority order (spawn → forest → highland → slope → dry → grass). Use `Color()` constructor with hex values from spec.
  - Moisture for dry check: separate noise sample `noise.noise_3d((x + seed) / 16.0, 1.4, (z - seed) / 16.0) * 0.5 + 0.5`
- Generate triangle indices (two triangles per quad)
- `generate_normals()`, `commit()` to ArrayMesh
- Assign material: `StandardMaterial3D` with `vertex_color_use_as_albedo = true`, `roughness = 0.96`, `metalness = 0.02`, `flat_shading = true`
- **Terrain collision:** After committing the ArrayMesh, add a `StaticBody3D` as sibling with `ConcavePolygonShape3D` generated from the mesh faces: `collision_shape.shape = ConcavePolygonShape3D.new()`, `collision_shape.shape.set_faces(mesh.get_faces())`. This is critical — without it, `CharacterBody3D.move_and_slide()` has no floor to detect and the player falls through the world.
- Store `GameState.forest_center`, `GameState.castle_center` from context
- Use a 4th `FastNoiseLite` instance for the moisture noise sample (vertex coloring). The moisture noise is sampled differently from the terrain layers — it uses the seed directly in the coordinates: `moisture_noise.noise_3d((x + seed) / 16.0, 1.4, (z - seed) / 16.0) * 0.5 + 0.5`

Reference: `tova-web/src/world.js` lines 236-285 for mesh generation, lines 11-18 for palette, line 250 for moisture noise.

- [ ] **Step 3: Create `world.tscn`**

Scene tree:
```
World (Node3D) [script: none — just a container]
├── TerrainMesh (MeshInstance3D) [script: terrain_generator.gd]
└── Structures (Node3D) [script: structure_builder.gd — added in Task 5]
```

- [ ] **Step 4: Verify terrain generation**

Create a temporary main scene with environment + world. Add a call to `generate_terrain(12345)` in `_ready()`. Run:
- Terrain mesh renders with vertex colors
- Spawn area is visibly flat
- Hills and elevation changes are visible
- Fog blends with terrain in the distance
- `print(sample_height(0, 0))` returns ~8.6 (spawn area height)

- [ ] **Step 5: Commit**

```bash
git add tova-godot/scripts/world/terrain_generator.gd tova-godot/scenes/world/
git commit -m "feat(godot): add procedural terrain generator with noise-based heightmap"
```

---

### Task 4: Player Controller

**Files:**
- Create: `tova-godot/scripts/player/player_controller.gd`
- Create: `tova-godot/scenes/player/player.tscn`

- [ ] **Step 1: Write `player_controller.gd` — mouse look + capture**

Create script extending `CharacterBody3D` with:
- `@onready var head: Node3D = $Head`
- `@onready var camera: Camera3D = $Head/Camera3D`
- `const LOOK_SENSITIVITY = 0.0022`
- `const MAX_PITCH = PI / 2.0 - 0.04`
- `_ready()`: set `Input.mouse_mode = Input.MOUSE_MODE_CAPTURED`
- `_unhandled_input(event)`:
  - If `InputEventMouseMotion`: rotate body Y by `-event.relative.x * LOOK_SENSITIVITY`, rotate head X by `-event.relative.y * LOOK_SENSITIVITY`, clamp head X to `[-MAX_PITCH, MAX_PITCH]`
  - If `InputEventMouseButton` left click + mouse not captured: capture mouse
  - If Escape pressed: `Input.mouse_mode = Input.MOUSE_MODE_VISIBLE`

- [ ] **Step 2: Write `player_controller.gd` — movement + gravity + jump**

Add to `_physics_process(delta)`:
- Read input: `move_forward`, `move_back`, `move_left`, `move_right`, `sprint`, `jump`
- Compute `input_dir` Vector2, normalize
- Get forward/right from head rotation (only Y component for horizontal movement)
- Target speed: `SPRINT_SPEED` if sprinting else `WALK_SPEED` (0 if no input)
- Acceleration: `_current_speed += (target - _current_speed) * (1.0 - exp(-rate * delta))`, rate = MOVE_ACCEL or MOVE_DECEL. Snap to 0 if < 0.08.
- Set `velocity.x` and `velocity.z` from direction * _current_speed. Note: `velocity` here is `CharacterBody3D.velocity` (Godot's built-in property), not a custom variable.
- Gravity: `velocity.y -= GameState.GRAVITY * delta`
- Jump: if `is_on_floor()` and jump pressed: `velocity.y = GameState.JUMP_SPEED`. Note: `is_on_floor()` works because the terrain has a `ConcavePolygonShape3D` collider (added in Task 3). No manual `sample_height()` ground check needed for movement — Godot's physics handles it.
- Call `move_and_slide()`

Reference: `tova-web/src/player.js` lines 141-234 for exact movement logic.

- [ ] **Step 3: Write `player_controller.gd` — head bob + landing dip**

Add head bob and landing dip logic:
- Track `_bob_phase`, `_bob_blend`, `_was_grounded`, `_land_dip_offset`
- Bob blend: `_bob_blend += ((1.0 if moving_on_ground else 0.0) - _bob_blend) * (1.0 - exp(-12.0 * delta))`
- Phase advance: `_bob_phase += freq * PI * 2.0 * delta` (freq = BOB_SPRINT_FREQ or BOB_WALK_FREQ)
- Vertical bob: `sin(_bob_phase) * BOB_VERTICAL_AMP * _bob_blend`
- Roll bob: `cos(_bob_phase * 0.5) * BOB_ROLL_AMP * _bob_blend`
- Landing dip: on grounded transition, `_land_dip_offset = min(abs(pre_ground_vy) * LAND_DIP_SCALE, LAND_DIP_MAX)`. Recover: `_land_dip_offset *= exp(-LAND_DIP_RECOVERY * delta)`
- Apply: `head.position.y = 1.6 + vertical_bob - _land_dip_offset`, `camera.rotation.z = roll_bob`
- **Note:** In Three.js, bob is applied to the player root's Y position. In Godot, modifying CharacterBody3D's Y would fight with `move_and_slide()`, so we deliberately apply it to the Head node instead. This is a correct Godot adaptation, not a bug.

Reference: `tova-web/src/player.js` lines 200-228.

- [ ] **Step 4: Add `respawn_at_spawn()` and fatigue/magicka**

- `respawn_at_spawn(terrain)`: position at `(0, terrain.sample_height(0, 0) + PLAYER_HEIGHT, 0)`, reset velocity, reset bob state. Look toward castle: compute `rotation.y = atan2(GameState.castle_center.x, GameState.castle_center.z)`, set `head.rotation.x = -0.08` (slight downward pitch). Reference: `player.js` lines 123-139.
- Fatigue drain: same formula from `player.js` lines 225-227
- Magicka regen: same formula

- [ ] **Step 5: Create `player.tscn`**

Scene tree:
```
Player (CharacterBody3D) [script: player_controller.gd]
├── CollisionShape3D
│   └── CapsuleShape3D (height=1.8, radius=0.35)
├── Head (Node3D, position.y=1.6)
│   └── Camera3D (fov=72, near=0.1, far=600)
```

- [ ] **Step 6: Verify**

Instance player + world + environment in a test scene. Run:
- Click captures mouse, Escape releases
- WASD moves player across terrain
- Player follows terrain height (goes up hills, down valleys)
- Sprinting is noticeably faster
- Jump works, gravity pulls back down
- Head bob visible while walking (subtle vertical + roll)
- Landing dip visible when landing from jump
- Can't fall through terrain

- [ ] **Step 7: Commit**

```bash
git add tova-godot/scripts/player/ tova-godot/scenes/player/
git commit -m "feat(godot): add FPS controller with head bob, landing dip, sprint, jump"
```

---

### Task 5: Structure Builder — Spawn Sanctum

**Files:**
- Create: `tova-godot/scripts/world/structure_builder.gd`

- [ ] **Step 1: Write `structure_builder.gd` — material helpers + rock mesh**

Create script extending `Node3D` with:
- `_create_flat_material(color: String, roughness: float = 0.96, metalness: float = 0.02) -> StandardMaterial3D`: creates a flat-shaded material with given color
- `_create_unshaded_material(color: String) -> StandardMaterial3D`: for flame meshes
- `_create_dodecahedron_mesh() -> ArrayMesh`: build a dodecahedron via SurfaceTool (12 pentagonal faces → 36 triangles). Use the 20 vertices of a regular dodecahedron at unit scale.
- `_add_static_collider(parent: Node3D, shape: Shape3D)`: helper to add StaticBody3D + CollisionShape3D child

- [ ] **Step 2: Write `build_spawn_sanctum()`**

Port `buildSpawnSanctum()` from `world.js` lines 339-451:
- Shrine group: dais (CylinderMesh 1.7/2.1 radii, height 0.72), altar (BoxMesh 0.86x1.28x0.86), stele (BoxMesh 1.2x2.8x0.34), arch posts (BoxMesh 0.28x2.2x0.28), arch cap (BoxMesh 2.18x0.26x0.28). All using shrine stone material `#84796f`.
- Two braziers: bowl (CylinderMesh `#50443a`), stem (CylinderMesh `#70645a`), flame (SphereMesh `#f6c56d` unshaded), OmniLight3D (`#f0bf63`, energy 1.2, range 13)
- Path stones: 5 BoxMesh stones lerped from origin to shrine position
- Scatter rocks: 14 dodecahedron meshes at random angles/distances around spawn
- Instanced grass: MultiMeshInstance3D with ConeMesh, 180 instances scattered around spawn
- Add StaticBody3D + CollisionShape3D to shrine pieces, rocks
- Set `GameState.sword_pickup_position` from shrine position
- **Note:** Pedestal sword mesh is deferred to Phase 2 (weapon system). Set `sword_pickup_position` now for future use.

All positions derived from terrain's `sample_height()`. Use `RandomNumberGenerator` seeded from `seed ^ 0xa7810d3f`.

- [ ] **Step 3: Verify**

Run project with terrain + sanctum:
- Shrine visible near spawn with dais, altar, stele, arch
- Two lit braziers flanking shrine
- Path stones leading from origin to shrine
- Rocks scattered around spawn area
- Grass tufts visible
- Player collides with shrine pieces and rocks

- [ ] **Step 4: Commit**

```bash
git add tova-godot/scripts/world/structure_builder.gd
git commit -m "feat(godot): add spawn sanctum — shrine, braziers, rocks, grass"
```

---

### Task 6: Structure Builder — Forest

**Files:**
- Modify: `tova-godot/scripts/world/structure_builder.gd`

- [ ] **Step 1: Write `build_forest()`**

Port `buildForest()` from `world.js` lines 454-515:
- Two `MultiMeshInstance3D` nodes: trunks (CylinderMesh, 220 instances) and canopies (ConeMesh, 220 instances)
- Trunk mesh: CylinderMesh top_radius=0.18, bottom_radius=0.28, height=2.8, radial_segments=7
- Canopy mesh: ConeMesh radius=1.35, height=3.8, radial_segments=8
- Trunk material: `#5a4838`, flat shaded. Canopy material: `#7a8060`, flat shaded.
- Placement: polar scatter around `GameState.forest_center`, distance `4 + sqrt(rng.randf()) * 18` (range 4-22), skip if too close to spawn (SPAWN_BLEND_RADIUS + 4) or castle (< 16). Use a `while placed < tree_count` loop with a max-attempts guard (break after 2000 iterations to avoid infinite loop if forest center is in an exclusion zone).
- Seed RNG with `seed ^ 0x1f123bb5`
- Per-tree random: trunk height 2-3.6, canopy height 3.1-4.5, canopy scale 0.8-1.35
- Set Transform3D per instance via `MultiMesh.set_instance_transform()`
- Add StaticBody3D + CylinderShape3D (radius 0.38) at each tree trunk base
- 28 scatter rocks around forest edges
- Enable cast_shadow on MultiMesh

Reference: `tova-web/src/world.js` lines 454-515.

- [ ] **Step 2: Verify**

Run project:
- Dense forest visible around forest_center
- Trees have trunks + canopies with correct colors
- No trees in spawn area or overlapping castle
- Player collides with tree trunks
- Rocks scattered around forest edges

- [ ] **Step 3: Commit**

```bash
git add tova-godot/scripts/world/structure_builder.gd
git commit -m "feat(godot): add forest with 220 instanced trees and collision"
```

---

### Task 7: Structure Builder — Castle

**Files:**
- Modify: `tova-godot/scripts/world/structure_builder.gd`

- [ ] **Step 1: Write `build_castle()`**

Port `buildCastle()` from `world.js` lines 517-613:
- Castle group positioned at `GameState.castle_center`
- Courtyard: BoxMesh 22x1.9x18, wall material `#68675f`
- 5 wall segments (BoxMesh) at exact positions from source
- 4 corner towers: CylinderMesh (radius 1.95/2.15, height 11.4, segments 10) + ConeMesh roof (radius 2.85, height 3.8)
- Tower offsets: `[-9.4, 0, -7.7], [9.4, 0, -7.7], [-9.4, 0, 7.7], [9.4, 0, 7.7]`
- Gate: BoxMesh 5.1x5.4x1.5, roof material `#3f3933`, at z=8.5
- Collision (exact dimensions from `world.js` lines 586-601):
  - Back wall: box at `(cx, cz - 8.4)`, half-extents `(11, 0.65)`
  - Front wall left: box at `(cx - 6.775, cz + 8.4)`, half-extents `(4.225, 0.65)`
  - Front wall right: box at `(cx + 6.775, cz + 8.4)`, half-extents `(4.225, 0.65)`
  - Left wall: box at `(cx - 10.3, cz)`, half-extents `(0.65, 7.75)`
  - Right wall: box at `(cx + 10.3, cz)`, half-extents `(0.65, 7.75)`
  - Keep: box at `(cx, cz + 0.5)`, half-extents `(4.2, 3.3)`
  - 4 tower cylinders: radius 2.2 at each tower offset
- 10 scatter rocks around castle (color `#7a756c`)
- Seed RNG with `seed ^ 0x9e3779b9`

- [ ] **Step 2: Verify**

Run project:
- Castle visible in the distance from spawn
- Walls, towers with roofs, gate visible
- Player can walk through gate opening
- Player collides with walls, towers, keep
- Rocks around castle perimeter

- [ ] **Step 3: Commit**

```bash
git add tova-godot/scripts/world/structure_builder.gd
git commit -m "feat(godot): add castle with walls, towers, gate, and collision"
```

---

### Task 8: Structure Builder — Haze and Landmarks

**Files:**
- Modify: `tova-godot/scripts/world/structure_builder.gd`

- [ ] **Step 1: Write `build_haze()`**

Port `buildHazeAndLandmarks()` from `world.js` lines 615-668. Seed RNG with `seed ^ 0x53142fcd`:
- 12 mist spheres: SphereMesh (radius 10-22), transparent material (color `#d0c0a8`, opacity 0.18, depth_draw disabled), flattened scale (1.7, 0.44, 1.1)
- Obelisk: BoxMesh 2.4x9x2.4, material `#7c6f5d`, at `(22, terrain_height + 4.5, 12)`. Add box collider.
- 3 ruins: each a group of two pillars (BoxMesh 0.42x3.4x0.48) + lintel (BoxMesh 2.8x0.42x0.52), material `#756857`, random positions

- [ ] **Step 2: Verify**

Run project:
- Mist spheres add atmospheric depth
- Obelisk visible as landmark
- Ruins scattered in the landscape
- Player collides with obelisk

- [ ] **Step 3: Commit**

```bash
git add tova-godot/scripts/world/structure_builder.gd
git commit -m "feat(godot): add haze, obelisk, and ruins"
```

---

### Task 9: Main Scene — Composition and World Regeneration

**Files:**
- Create: `tova-godot/scripts/main.gd`
- Create: `tova-godot/scenes/main.tscn`

- [ ] **Step 1: Write `main.gd`**

Create script extending `Node3D`:
- `@onready var world: Node3D = $World`
- `@onready var player: CharacterBody3D = $Player`
- `@onready var environment: Node3D = $Environment`
- References to terrain_generator and structure_builder via world's children
- `_ready()`: call `regenerate_world()`, connect GameState signals
- `_unhandled_input(event)`: if `regenerate` action pressed, call `regenerate_world()`
- `regenerate_world()`:
  1. Generate random seed: `randi()`
  2. `GameState.reset_state()`
  3. `GameState.seed = seed`
  4. Clear existing structures (free children of Structures node)
  5. `terrain_generator.generate_terrain(seed)`
  6. `structure_builder.build_spawn_sanctum(seed, terrain_generator)`
  7. `structure_builder.build_forest(seed, terrain_generator)`
  8. `structure_builder.build_castle(seed, terrain_generator)`
  9. `structure_builder.build_haze(seed, terrain_generator)`
  10. Position spawn fill light: `environment.update_spawn_light(terrain_generator.sample_height(4, 0))`
  11. `player.respawn_at_spawn(terrain_generator)`
  12. `GameState.world_regenerated.emit(seed)`
  13. `GameState.status_changed.emit("Green rise ahead. The keep waits in the distance.")`

- [ ] **Step 2: Create `main.tscn`**

Scene tree:
```
Main (Node3D) [script: main.gd]
├── Environment (instance of environment.tscn)
├── World (instance of world.tscn)
└── Player (instance of player.tscn)
```

- [ ] **Step 3: Verify — full integration**

Run the complete project:
1. World generates on launch — terrain, shrine, forest, castle, haze all visible
2. Player spawns at origin, looking toward castle
3. Walk to shrine — collide with altar
4. Walk to forest — collide with trees
5. Walk to castle — enter through gate, collide with walls
6. Press R — world regenerates with new seed, player respawns
7. Fog, sky, lighting create warm atmospheric mood
8. Head bob, sprint, jump all work on terrain
9. Mouse capture/release works

- [ ] **Step 4: Commit**

```bash
git add tova-godot/scripts/main.gd tova-godot/scenes/main.tscn
git commit -m "feat(godot): wire main scene — world gen, player spawn, R to regenerate"
```

---

### Task 10: Polish Pass + CLAUDE.md

**Files:**
- Create: `tova-godot/CLAUDE.md`
- Modify: `tova-godot/project.godot` (if needed)

- [ ] **Step 1: Write `tova-godot/CLAUDE.md`**

Document:
- Project overview (Godot 4.6 port of TOVA)
- Quick start: open in Godot, press F5 to run
- Project structure with file responsibilities
- Architecture notes: scene-per-system, GameState autoload, signal communication
- How to verify: list of things to check when running
- Phase status: Phase 1 complete, Phase 2-4 planned
- Controls: click to capture, WASD, shift sprint, space jump, R regenerate, Esc release

- [ ] **Step 2: Tune terrain and environment**

Run the project and compare atmosphere to the Three.js version at the deployed URL. Adjust:
- ProceduralSkyMaterial colors if sky looks too blue/too dark
- Fog density if terrain fades too quickly or not enough
- Light energy values if scene is too dark/bright
- Terrain noise amplitudes if terrain is too flat/too mountainous

This is a visual tuning step — iterate until the mood matches.

- [ ] **Step 3: Final commit**

```bash
git add tova-godot/
git commit -m "feat(godot): Phase 1 complete — core loop ported to Godot 4.6"
```

---

## Task Dependency Graph

```
Task 1 (scaffold + GameState)
  ├── Task 2 (environment)
  ├── Task 3 (terrain)
  │   ├── Task 5 (sanctum)
  │   ├── Task 6 (forest)
  │   ├── Task 7 (castle)
  │   └── Task 8 (haze)
  └── Task 4 (player)
       └── Task 9 (main scene — needs all above)
            └── Task 10 (polish + CLAUDE.md)
```

Tasks 2, 3, and 4 can run in parallel after Task 1. Tasks 5-8 can run in parallel after Task 3. Task 9 requires all prior tasks. Task 10 is final polish.
