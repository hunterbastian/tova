# CLAUDE.md

## Project Overview

Tova is a first-person 3D exploration game that currently ships as a web-first Three.js experience in `/Users/hunterbastian/Desktop/Code/tova/tova-web`. The maintained product surface is the browser client; the older Rust + `wgpu` prototype in `tova-engine/` is archived reference code, not the active delivery target.

## Quick Start

```sh
./scripts/run_tova_web.sh

cd tova-web
npm install
npm run dev
npm run build
```

## Tech Stack

- **JavaScript** — browser game runtime
- **Three.js** — scene graph, rendering, lighting, and first-person world presentation
- **Vite** — local dev server and production bundling
- **PointerLockControls** — first-person browser controls
- **Sky / ImprovedNoise** — atmosphere and procedural terrain shaping

## Project Structure

```text
tova-web/
├── package.json
├── src/
│   ├── main.js               # Browser bootstrap, scene wiring, test hooks
│   ├── constants.js          # Shared gameplay constants and state factory
│   ├── ui.js                 # HUD shell, compass, prompt, hotbar, minimap
│   ├── player.js             # Input, movement, jump, sprint, fullscreen
│   ├── weapon.js             # Sword pickup, viewmodel, swing animation
│   ├── world.js              # Procedural terrain, shrine, forest, castle, haze
│   ├── actors.js             # Skeleton enemies: spawning, AI, combat, meshes
│   ├── audio.js              # Web Audio procedural SFX (footsteps, wind, combat)
│   ├── collision.js          # Cylinder and box collider resolution
│   ├── interactables.js      # Proximity-based interact system (E to use)
│   └── style.css             # Web UI styling
└── public/
    └── favicon.svg
```

## Architecture Notes

- **Bootstrap** lives in `src/main.js` and wires together UI, world, player, weapon, actor, audio, collision, and interactable systems.
- **World generation** builds procedural terrain with a grassy spawn, nearby forest, castle landmark, shrine sword pickup, and haze layers.
- **Movement** uses grounded first-person browser controls with gravity, jumping, sprinting, and webdriver-only automation toggles.
- **Actors** are skeleton enemies with pursue/attack AI, stagger, death animations, and shared GPU resources.
- **Audio** is fully procedural via Web Audio API — footsteps, wind, sword swings, hits, and damage are all synthesized (no audio files).
- **Collision** resolves player-vs-world overlaps using cylinder and box colliders registered during world generation.
- **Interactables** provide proximity-triggered E-to-interact prompts (shrine sword pickup, extensible for future items).
- **HUD** is a restrained Elder Scrolls-style overlay with compass, hotbar, vitals, minimap, prompt text, and seed/status labels.
- **Testing hooks** expose `window.render_game_to_text()` and `window.advanceTime(ms)` for deterministic Playwright coverage.

## Controls

- **Click** — grab cursor
- **Esc** — release cursor
- **WASD / Arrow keys** — move
- **Space** — jump
- **Shift** — sprint
- **Mouse** — look around
- **E** — take sword
- **R** — regenerate world
- **1-5** — hotbar selection

## Status

- [x] Maintained web client in `tova-web/`
- [x] Procedural terrain, forest, castle, shrine sword pickup
- [x] Browser-first controls and HUD
- [x] Playwright test loop with state output
- [x] Skeleton enemy combat with AI, stagger, and death
- [x] Procedural audio (wind, footsteps, combat SFX)
- [x] Collision system and interactable framework
- [x] Compass and minimap HUD elements
- [ ] Bundle-size cleanup / code splitting
- [ ] Deeper game systems beyond exploration and pickup

## Archived Native Prototype

The old Rust + `wgpu` prototype remains in `tova-engine/` for reference only. If that path is revived later, treat it as a fresh product decision instead of the current default direction.

## Testing & Linting

Use the web client checks:

```sh
cd tova-web
npm run build
```

For interactive/browser validation, launch `./scripts/run_tova_web.sh` and use the Playwright client against `http://127.0.0.1:4174`.
