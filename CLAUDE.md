# CLAUDE.md

## Project Overview

Tova is a first-person 3D exploration game with procedurally generated voxel terrain, built with Rust and wgpu. It targets native desktop (Windows, Mac, Linux) and Steam/Steam Deck, with future iOS support planned.

## Quick Start

```sh
cd tova-engine
cargo run            # Debug build + run
cargo run --release  # Optimized build + run
cargo build          # Build without running
```

## Tech Stack

- **Rust** — systems language, no GC
- **wgpu** — GPU rendering (Vulkan/Metal/DX12/WebGPU)
- **winit** — window creation and input
- **glam** — fast vector/matrix math
- **bytemuck** — safe casting for GPU buffer data
- **WGSL** — shader language

## Project Structure

```
tova-engine/
├── Cargo.toml
├── src/
│   ├── main.rs               # Entry point: window, event loop, input dispatch
│   ├── renderer/
│   │   ├── mod.rs             # Module exports
│   │   ├── state.rs           # wgpu device, surface, pipeline, render loop
│   │   ├── camera.rs          # First-person camera (yaw/pitch, view-proj matrix)
│   │   └── vertex.rs          # Vertex layout, ground grid geometry
│   └── player/
│       └── mod.rs             # Input tracking (WASD, Space, Shift, mouse)
├── assets/
│   └── shaders/
│       └── shader.wgsl        # Vertex/fragment shaders with camera uniform
```

## Architecture Notes

- **Event loop** runs via winit's `ApplicationHandler`. `about_to_wait` triggers continuous redraws.
- **Render pipeline** uses wgpu with depth buffer, perspective camera (70° FOV), and sky-blue clear color.
- **Camera** supports fly mode (WASD + Space/Shift). Mouse look with yaw/pitch via `DeviceEvent::MouseMotion`.
- **Cursor grab** on click, release on Esc. Uses `CursorGrabMode::Locked` with `Confined` fallback.
- **Ground grid** is a flat checkerboard of colored quads for spatial reference (temporary — will be replaced by voxel terrain).

## Controls

- **Click** — grab cursor
- **Esc** — release cursor
- **WASD / Arrow keys** — move
- **Space** — fly up
- **Shift** — fly down
- **Mouse** — look around

## Migration Status

See `MIGRATION_PLAN.md` for the full roadmap. Current status:

- [x] Phase 1: Foundation (window, renderer, camera, movement)
- [ ] Phase 2: Voxel Core (blocks, chunks, greedy meshing)
- [ ] Phase 3: Terrain Generation (noise, biomes)
- [ ] Phase 4: Chunk Streaming (multithreaded)
- [ ] Phase 5: Player & Physics
- [ ] Phase 6: World Content (lighting, ocean, structures)
- [ ] Phase 7: Polish & UI
- [ ] Phase 8: Steam Distribution

## Testing & Linting

No test or lint configuration yet. Use `cargo clippy` for linting and `cargo test` when tests are added.
