# CLAUDE.md

## Project Overview

TOVA Godot is a port of the TOVA first-person exploration game from Three.js to Godot 4.6. The game features procedural terrain, a shrine, forest, castle, and atmospheric lighting in a warm, earthy palette.

## Quick Start

Open `project.godot` in Godot 4.6, press F5 to run.

## Tech Stack

- **Godot 4.6** — Forward+ renderer
- **GDScript** — all game logic
- **FastNoiseLite** — procedural terrain noise
- **SurfaceTool/ArrayMesh** — terrain mesh generation
- **MultiMeshInstance3D** — instanced trees and grass

## Project Structure

```text
tova-godot/
├── project.godot
├── scenes/
│   └── main.tscn                  # Root scene (minimal — tree built in code)
├── scripts/
│   ├── autoload/
│   │   └── game_state.gd          # Singleton: constants, state, signals
│   ├── main.gd                    # Orchestrator: scene setup, world gen, R key
│   ├── player/
│   │   └── player_controller.gd   # FPS: mouse look, WASD, gravity, jump, head bob
│   ├── world/
│   │   ├── terrain_generator.gd   # Noise heightmap, vertex colors, collision
│   │   └── structure_builder.gd   # Shrine, forest, castle, haze, rocks
│   └── environment/
│       └── environment_setup.gd   # Sky, sun, fog, moon, fill light
└── assets/
    └── (empty — all procedural)
```

## Architecture

- **Scene-per-system** with signal communication
- **One autoload** (`GameState`) for shared constants and state
- Scene tree is built programmatically in `main.gd`
- Systems communicate via `GameState` signals

## Controls

- **Click** — capture mouse
- **WASD / Arrows** — move
- **Mouse** — look
- **Space** — jump
- **Shift** — sprint
- **R** — regenerate world (new seed)
- **Esc** — release cursor

## Verification

Run the project in Godot (F5) and check:
1. Terrain renders with vertex colors and zone shaping
2. Player walks, sprints, jumps with head bob
3. Shrine, forest, castle, and haze are visible
4. Collision works (can't walk through walls, trees, shrine)
5. R regenerates with a new seed
6. Fog and sky create warm atmosphere

## Phase Status

- [x] Phase 1 — Core loop (controller, terrain, structures, environment)
- [ ] Phase 2 — Combat (skeleton AI, sword, interactables)
- [ ] Phase 3 — HUD/UI + procedural audio
- [ ] Phase 4 — Retro post-processing (pixel shader + color banding)

## Port Source

Three.js version: `../tova-web/src/`
Design spec: `../docs/superpowers/specs/2026-03-23-godot-port-design.md`
Implementation plan: `../docs/superpowers/plans/2026-03-23-godot-port-phase1.md`
