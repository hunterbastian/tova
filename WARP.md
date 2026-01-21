# WARP.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Project overview

This repository contains a Three.js-powered first-person exploration scene, built and served with Vite.

The current runtime entrypoint is the web client:
- `index.html` is a minimal shell that shows a "Loading Kingdom..." overlay and loads `src/main.js` as a module.
- `src/main.js` sets up the Three.js renderer, camera, and scene, creates the world objects (environment, terrain, ocean, castle, town) and the `Player` controller, and runs the main animation loop.

There is also a legacy/backup Godot 4 project under `godot_backup/` that implements a voxel-style world with first-person controls. It is not wired into the Vite/Three.js app but is a useful reference for world generation and interaction mechanics.

## Common commands

From the repository root:

- Install dependencies (Node + npm):
  - `npm install`
- Start the development server (Vite):
  - `npm run dev`
- Build a production bundle:
  - `npm run build`
- Preview the production build locally (serves the built `dist/` directory via Vite):
  - `npm run preview`

There are currently no npm scripts configured for tests or linting in `package.json`. If you need tests or linting, add the appropriate tooling and scripts before expecting those commands to exist.

## Frontend architecture (Vite + Three.js)

### Entrypoint and render loop

- `index.html`
  - Provides the `<canvas>` host (via the Three.js renderer) and a `#loading` overlay that is faded out and removed once `src/main.js` starts.
  - Loads `src/main.js` as a module.

- `src/main.js`
  - Creates the Three.js `Scene`, `PerspectiveCamera`, and `WebGLRenderer`.
  - Enables shadows and ACES filmic tone mapping on the renderer.
  - Sets up a postprocessing pipeline using `EffectComposer` with:
    - `RenderPass` (scene + camera),
    - `UnrealBloomPass` (glow/bloom),
    - `OutputPass`.
  - Constructs and attaches world components:
    - `Environment` (lighting, sky, fog),
    - `Terrain` (procedural ground mesh + height queries),
    - `Ocean` (large animated water plane),
    - `Castle` and `Town` (static structures positioned relative to the terrain),
    - `Player` (first-person pointer-lock controller bound to the camera and terrain).
  - On each animation frame (`animate()`):
    - Uses a shared `THREE.Clock` to compute `delta` and elapsed `time`.
    - Calls `ocean.update(time)` for simple water motion.
    - Calls `player.update(delta)` for movement and terrain alignment.
    - Renders via `composer.render()` (postprocessed output) instead of `renderer.render(...)`.
  - Handles window `resize` events by updating camera aspect and resizing both the renderer and composer.

The render loop is the central place where time-based behavior is driven. New animated or simulation components should either be updated from here or follow the same pattern.

### World & environment modules

- `src/world/Environment.js`
  - Owns global scene appearance:
    - Sets a sky color and exponential fog for depth.
    - Adds ambient, hemisphere, and a warm directional "sun" light.
  - Configures the directional light for high-quality shadows (large shadow map, tuned camera frustum, soft shadows via `shadow.radius` and `bias`).

- `src/world/Terrain.js`
  - Builds a large `THREE.PlaneGeometry` (500x500 with 128x128 segments) and procedurally displaces its vertices to form:
    - A central hill (for the castle),
    - A flatter region for the town,
    - A lowered region that transitions toward the ocean.
  - After deformation, recomputes vertex normals and creates a `MeshStandardMaterial` representing grass/earth.
  - Rotates the plane to lie horizontally (`rotation.x = -Math.PI / 2`) and enables both casting and receiving shadows.
  - Adds the terrain mesh to the scene.
  - Exposes `getHeightAt(x, z)`, which:
    - Uses a downward `THREE.Raycaster` from a high Y to intersect the terrain mesh.
    - Returns the intersection Y coordinate or `0` if nothing is hit.
  - This method is the primary way other systems (Player, Castle, Town) query ground height and is called every frame for the player.

- `src/world/Ocean.js`
  - Creates a very large `PlaneGeometry` below the terrain to represent the sea.
  - Uses a `MeshPhysicalMaterial` tuned for a glassy, water-like look (transmission, clearcoat, low roughness).
  - Rotates the plane horizontally and positions it at a fixed base water level.
  - `update(time)` applies a gentle sinusoidal vertical motion to simulate tides.

### Structures

- `src/structures/Castle.js`
  - Builds a castle as a `THREE.Group` of simple primitives:
    - A central keep (`BoxGeometry`),
    - Four cylindrical corner towers,
    - Connecting wall segments.
  - Enables shadows for all major pieces.
  - Positions the entire castle group at the terrain height at `(0, 0)` via `terrain.getHeightAt(0, 0)`, assuming the main hill is centered there.

- `src/structures/Town.js`
  - Creates a `THREE.Group` of simple houses near the hill in the "flattened" region defined in `Terrain.js` (x in ~[20, 100], z in ~[-50, 50]).
  - Each house is a small group composed of a wooden box base and a conical roof, both casting shadows.
  - For each house, queries `terrain.getHeightAt(x, z)` to place it snugly on the ground and randomizes rotation for variation.

The castle and town both depend on the terrain's coordinate system and on `getHeightAt` for correct vertical placement.

### Player and controls

- `src/controls/Player.js`
  - Wraps Three.js `PointerLockControls` and treats its underlying object as the player/camera rig.
  - Manages input state for movement (WASD / arrow keys) via DOM `keydown` / `keyup` listeners.
  - Displays a centered overlay with the title "Tova" and basic instructions, and uses pointer lock events to show/hide it.
  - In `update(delta)`:
    - Applies forward/strafe movement based on key state and a `walkSpeed` scalar.
    - Calls `alignToTerrain(delta)` each frame.
  - `alignToTerrain(delta)`:
    - Uses `terrain.getHeightAt(x, z)` to compute a target ground height plus an `eyeHeight` offset.
    - On first call (no `delta`), snaps directly to the ground.
    - Thereafter, smoothly interpolates the Y position toward the ground using `groundSnapSpeed`, producing stable walking over uneven terrain.

The `Player` has a hard dependency on `Terrain` (for ground snapping) and on the DOM (for instruction overlay and pointer lock). Any significant changes to terrain scale or coordinate system should be reflected here.

## Godot backup project

The `godot_backup/` directory contains a Godot 4 project named "Tova" with its own 3D world and controls. Key pieces:

- `godot_backup/project.godot`
  - Declares the application name and main scene (`world.tscn`).
  - Defines input actions for movement, jumping, sprinting, and block interactions (break/place, hotbar selection, toggling torch and overcast lighting).

- `godot_backup/player.gd`
  - A `CharacterBody3D` implementing first-person movement and looking via mouse input.
  - Handles sprinting, jumping, and gravity, and keeps a torch light positioned near the camera.
  - Implements block interaction:
    - `_cast_from_center()` raycasts from the center of the viewport.
    - `_break_block()` and `_place_block()` delegate to methods on the `World` node based on raycast hits.

- `godot_backup/world.gd`
  - Implements a simple voxel world:
    - Stores block types and colors in `BLOCK_TYPES`.
    - Maintains a `grid` of blocks and per-type `MultiMeshInstance3D` for efficient rendering.
    - Generates terrain using a small value-noise function and a base height, then adds a fenced area and huts near the center.
  - Provides APIs used by the player:
    - `place_block_worldspace(world_pos, t)` to add blocks at world coordinates.
    - `break_at_hit(hit)` to remove a block at a raycast hit position.
  - Rebuilds visible instances in `_draw_all()` when the world is marked `dirty`.

- `godot_backup/sun.gd`
  - A `DirectionalLight3D` that toggles between normal and overcast lighting states.

This Godot project is independent from the Three.js/Vite frontend but encodes similar gameplay ideas (first-person exploration, editable block world) and can serve as a design and logic reference if you need to port or compare behaviors.

## Non-obvious couplings and invariants

- The Three.js `Terrain` is assumed to be centered around `(0, 0)` in X/Z; the `Castle` positions itself exactly at this origin using `getHeightAt(0, 0)`, and the `Town` expects its flattened region to remain in the hard-coded coordinate ranges used in both `Terrain.js` and `Town.js`.
- `Terrain.getHeightAt(x, z)` performs a raycast from Y=100 straight down every time it is called; the `Player` calls this every frame, which is acceptable for a single player but could become expensive if many entities start querying height each frame.
- The `Player` assumes that the camera is the only viewpoint and that pointer lock is available (e.g., in a desktop browser). Any alternate control scheme or camera system will need to revisit `Player` and possibly how the main loop in `src/main.js` is structured.
