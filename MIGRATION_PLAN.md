# Tova Rust + wgpu — Migration Plan

Migrating Tova from Three.js (browser) to Rust + wgpu (native desktop) for maximum performance and Steam/Steam Deck distribution.

## Phase 1: Foundation (Window + Renderer)
- Set up Rust project with `cargo`
- Window creation with `winit`
- Basic wgpu render pipeline (clear screen with a color)
- Camera (first-person, mouse look)
- WASD movement (flying camera, no terrain yet)
- **Result:** You can fly around an empty colored sky

## Phase 2: Voxel Core
- Define block types (Air, Grass, Dirt, Stone, Sand, Water, Cobble)
- Implement Chunk data structure (16×16×128)
- Port greedy meshing algorithm from `src/game/mesh/VoxelMesher.js`
- Render a single hardcoded chunk with colored faces
- **Result:** A single voxel chunk visible on screen

## Phase 3: Terrain Generation
- Port noise functions from `src/game/world/TerrainField.js` (FBM, ridged, value noise)
- Port biome system (Plains, Forest, Alpine, Coast)
- Port `src/game/world/WorldGen.js` — generate chunk data from noise
- Render multiple chunks in a grid
- **Result:** Procedural terrain you can fly over

## Phase 4: Chunk Streaming
- ChunkManager — load/unload chunks around player position
- Multithreaded generation — spawn chunk gen on background threads
- Frame budget system (or just let threads deliver finished chunks)
- **Result:** Infinite streaming terrain as you move

## Phase 5: Player & Physics
- Ground collision / terrain snapping
- Walk mode vs fly mode
- Eye height, movement speeds matching current game feel
- **Result:** Walk around on terrain like current Three.js version

## Phase 6: World Content
- Lighting (sun, ambient, shadows)
- Sky/fog/atmosphere
- Ocean plane with animation
- Mountain rings
- Instanced trees (forest)
- Castle and town structures
- **Result:** Feature parity with current Three.js version

## Phase 7: Polish & UI
- HUD (crosshair, FPS, coordinates, minimap)
- Command palette (fly/walk/day/night)
- Audio (ambient music)
- Postprocessing (bloom, tone mapping)
- **Result:** Feels like a complete game

## Phase 8: Distribution
- Steam integration (`steamworks` crate)
- Build targets: Windows, Mac, Linux
- Auto-updater / launcher
- Steam Deck testing + controller input
- **Result:** Ready to ship on Steam

## Phase 9 (Future): Mobile
- iOS port via Metal backend
- Touch controls
- App Store packaging

## Project Structure

```
tova-engine/
├── Cargo.toml
├── src/
│   ├── main.rs           # Entry point, window, event loop
│   ├── renderer/         # wgpu pipeline, shaders, camera
│   ├── voxel/            # Chunk, meshing, block types
│   ├── world/            # Terrain gen, noise, biomes, chunk manager
│   ├── player/           # Movement, collision, modes
│   ├── ui/               # egui HUD, command palette
│   └── audio/            # Music, SFX
├── assets/
│   └── shaders/          # WGSL shader files
```

## Key Crates

- `wgpu` — GPU rendering (Vulkan/Metal/DX12/WebGPU)
- `winit` — window creation and input
- `glam` — fast math (vectors, matrices)
- `noise` — procedural noise functions
- `egui` + `egui-wgpu` — immediate mode UI
- `rodio` or `kira` — audio
- `steamworks` — Steam integration

## Reference

The Three.js version at `src/` remains the reference implementation. Key files to port from:
- `src/game/mesh/VoxelMesher.js` — greedy meshing algorithm
- `src/game/world/TerrainField.js` — noise and biome sampling
- `src/game/world/WorldGen.js` — chunk data generation
- `src/game/world/ChunkManager.js` — chunk streaming logic
- `src/game/world/constants.js` — CHUNK_SIZE=16, WORLD_HEIGHT=128, SEA_LEVEL=48
