# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Tova** is a Three.js-powered first-person exploration game featuring a medieval kingdom with a castle, town, and ocean. It's built with Vite and uses ES modules.

The web client is the current runtime:
- `index.html` - Minimal shell with loading overlay, loads `src/main.js`
- `src/main.js` - Three.js renderer, scene setup, postprocessing pipeline, and animation loop

A legacy Godot 4 project exists in `godot_backup/` for reference (voxel-style world with block interactions).

## Common Commands

```bash
npm install      # Install dependencies
npm run dev      # Start Vite dev server
npm run build    # Production build to dist/
npm run preview  # Preview production build
```

No tests or linting are currently configured.

## Architecture

### Directory Structure

```
src/
├── main.js              # Entry point, scene setup, render loop
├── controls/
│   └── Player.js        # First-person PointerLockControls + terrain alignment
├── structures/
│   ├── Castle.js        # Castle group (keep + towers + walls)
│   └── Town.js          # Procedural house placement
└── world/
    ├── Environment.js   # Lighting, fog, sky color
    ├── Ocean.js         # Animated water plane
    └── Terrain.js       # Procedural terrain with getHeightAt()
```

### Key Patterns

- **ES Modules**: All imports use `.js` extensions
- **Class-based components**: Each world element is a class that receives `scene` and optionally `terrain` in constructor
- **Terrain height queries**: Components use `terrain.getHeightAt(x, z)` to position objects on ground
- **Postprocessing**: Uses EffectComposer with RenderPass, UnrealBloomPass, OutputPass

### Important Couplings

- `Terrain` is centered at `(0, 0)` - Castle positions at origin, Town in range x:[20,100], z:[-50,50]
- `Player.alignToTerrain()` calls `terrain.getHeightAt()` every frame
- Player depends on PointerLockControls and DOM for overlay/input

### Player Controls

- Click to enable pointer lock
- WASD/arrows to move
- Press `/` to open chat, type `fly` or `walk` to toggle flight mode
- In fly mode: Space=up, Shift=down

## Code Style

- No semicolons required (codebase uses semicolons)
- camelCase for variables and functions
- PascalCase for classes
- Single quotes for strings preferred in some files, double in others (be consistent with file you're editing)
- Constants at class level (e.g., `this.walkSpeed = 30`)
