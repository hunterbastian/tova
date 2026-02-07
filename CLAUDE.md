# CLAUDE.md

## Project Overview

Tova is a first-person 3D exploration game with procedurally generated voxel terrain, built with Three.js and Vite. It features a chunk-streaming world, postprocessed rendering, and a fantasy medieval setting with castles, towns, mountains, forests, and an ocean.

## Quick Start

```sh
npm install
npm run dev        # Dev server at localhost:5173
npm run build      # Production build → dist/
npm run preview    # Serve production build locally
```

## Tech Stack

- **JavaScript (ES6 modules)** — no TypeScript
- **Three.js ^0.160.0** — 3D rendering
- **Vite ^5.0.0** — build tool and dev server
- **Vercel** — deployment target

## Project Structure

```
src/
├── main.js                  # Entry point: scene setup, render loop, UI/HUD
├── controls/Player.js       # First-person pointer-lock controller
├── game/                    # Voxel world generation system
│   ├── world/
│   │   ├── VoxelWorld.js    # Facade managing chunk streaming
│   │   ├── ChunkManager.js  # Chunk load/unload with frame budgets
│   │   ├── Chunk.js         # Individual chunk block data (16×16×128)
│   │   ├── WorldGen.js      # Procedural generation (seed: 133742)
│   │   ├── TerrainField.js  # Noise functions, biome sampling
│   │   └── constants.js     # CHUNK_SIZE=16, WORLD_HEIGHT=128, SEA_LEVEL=48
│   └── mesh/
│       └── VoxelMesher.js   # Greedy meshing algorithm
├── world/                   # Static world components
│   ├── Environment.js       # Lighting, sky, fog
│   ├── Terrain.js           # Ground mesh (legacy)
│   ├── Ocean.js             # Animated water plane
│   ├── Mountains.js         # Alpine mountain rings
│   └── Forest.js            # Instanced trees
├── structures/
│   ├── Castle.js            # Central castle
│   └── Town.js              # Fantasy village
└── assets/
```

Legacy Godot projects exist under `godot_backup/` (GDScript) and `godot_cs/` (C#) for reference.

## Architecture Notes

- **Render loop** runs via `requestAnimationFrame` in `main.js` with a postprocessing pipeline (bloom, tone mapping, optional vignette/grain).
- **Voxel world** streams chunks around the player (load radius: 4 chunks, unload: 5). Frame budgets cap generation at 2 chunk data builds + 1 mesh build per frame.
- **TerrainField** provides deterministic height/biome queries. Authored core (4km×4km) blends into procedural frontier.
- **Player** snaps to terrain each frame via `Terrain.getHeightAt(x, z)` (raycast from Y=100).
- Terrain is centered at origin. Castle sits at (0,0). Town occupies hardcoded coordinate ranges (~[20,100] X, ~[-50,50] Z).
- Static meshes (Town, Castle) are frozen and merged by material to reduce draw calls.

## Testing & Linting

No test, lint, or format scripts are configured yet.

## URL Query Params

- `?pixelRatio=X` — override device pixel ratio (default capped at 1.5)
- `?vignette=0` — disable vignette
- `?grain=0` — disable film grain

## In-Game Commands

Press `/` to open the command palette. Available commands: `fly`, `walk`, `day`, `night`.
