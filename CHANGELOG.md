# Changelog

## This Week — 2026-03-06
- Switched the web client to default into browser-safe mode, added an explicit enhanced-mode opt-in, and carved a clearer meadow sightline so the castle reads immediately instead of disappearing into hills or browser-sensitive shading.
- Added a browser-safe render mode (`?safe=1`) with simpler Three.js materials, disabled heavy sky/shadow features, and visible intro links so players have a fallback path if their browser renders the main scene poorly.
- Hardened web visibility at spawn with a brighter clear color, stronger ambient/hemi/sun lighting, a local spawn fill light, lighter meadow palette, and softer fog so the opening scene cannot read as black.
- Reframed the web spawn so the opening view now looks across a grassy rise toward the distant castle, moved the shrine off the centerline, and brightened the spawn lighting/fog so the first scene reads clearly.
- Changed the web startup flow so Tova opens in a visible intro state instead of dropping straight into the dark scene, and added a plain HTML boot card in `index.html` so the page still shows guidance even if WebGL fails.
- Fixed the web input bootstrap so a click always enters the game: real browsers still attempt pointer lock, while unsupported or automation environments fall back to walk mode with manual mouse-look instead of silently doing nothing.
- Updated the web HUD copy to advertise the new `Enter` fallback path and removed the pointer-lock error noise from automated browser sessions.
- Linked `tova-web` to the existing Vercel `tova` project, confirmed the GitHub repository connection, and published a production web build.
- Confirmed the live play link responds successfully on Vercel (`https://tova-hunterbastians-projects.vercel.app`) so Tova now has a stable browser URL.
- Added two new gameplay foundations to the web client: a generic interactable registry for pickups/use prompts and an actor system that spawns and tracks dormant skeleton sentries around the world.
- Rewired the shrine sword through the new interactable layer and exposed actor/interactable state in `render_game_to_text` so future combat and encounter work has a stable runtime base.
- Simplified Tova down to a single supported product surface: the web client. `scripts/run_tova_web.sh` now launches `tova-web`, while `scripts/run_tova_three_web.sh` remains only as a compatibility alias.
- Rewrote the top-level project docs to describe Tova as a maintained web-first game and archived the old Rust migration plan instead of presenting it as the active roadmap.
- Split the Three.js browser entry into focused modules (`constants`, `ui`, `player`, `weapon`, `world`) so `tova-web/src/main.js` is now just scene/bootstrap wiring.
- Declared the Three.js client as the maintained browser path and marked the Rust `trunk` launcher as experimental to avoid treating both web routes as equal surfaces.
- Reused shared Three.js geometry/material caches for the sword, terrain, rocks, brazier pieces, and spawn shrine so world regeneration stops reallocating those assets every reset.
- Scoped browser automation controls to webdriver sessions only, keeping `Enter` walk mode and the `B` interaction alias out of normal player input while preserving deterministic Playwright coverage.
- Pushed the Three.js browser client toward a heavier `Morrowind` / `Oblivion` tone with duskier fog, older shrine/castle dressing, a more restrained HUD, and a stronger Elder Scrolls-style mood.
- Added a shrine sword pickup loop to the Three.js client, including an interaction prompt, acquired weapon state, and a held first-person sword model once the weapon is taken.
- Added a separate `tova-web` browser client built heavily around Three.js (`Scene`, `WebGLRenderer`, `PointerLockControls`, `Sky`, `ImprovedNoise`, instanced forests, procedural terrain, and landmark geometry) so the web version can lean into browser-native rendering instead of mirroring the Rust renderer one-to-one.
- Added procedural Three.js terrain with a flat grassy spawn, nearby forest, distant castle, first-person movement, hotbar HUD, and browser test hooks (`render_game_to_text`, `advanceTime`).
- Added `scripts/run_tova_three_web.sh` to launch the Three.js browser build locally on `http://127.0.0.1:4174`.
- Added a browser/WebAssembly build path with `trunk`, including browser-safe timing, canvas resizing, and a loading handoff so Tova can boot into WebGPU on the web.
- Added a `scripts/run_tova_web.sh` helper to serve the browser build locally on `http://127.0.0.1:4173`.
- Replaced the fly camera with grounded first-person movement, including gravity, jump arc, horizontal voxel collision, and player-aware block placement checks.
- Reworked the HUD into a smaller bottom hotbar with subtle corner vitals and a lighter top-center status banner to push the game toward a lower-HUD first-person feel.
- Deepened the atmosphere with denser distance fog, valley haze, moodier lighting, and a darker sky clear color for a more austere mountain mood.
- Replaced the fixed mountain map with seeded procedural terrain while preserving a guaranteed flat spawn clearing at the origin for stable starts.
- Added guaranteed landmarks to the procedural world: a grassy spawn clearing, a nearby forest with tree blocks, and a castle ruin stamped into the terrain.

## This Week — 2026-03-05
- Rebuilt Tova around a new native runtime (`app`, `graphics`, `camera`, `input`) and made it the sole executable path.
- Replaced the old procedural/cave terrain path with a fixed mountain-start world and summit spawn for a stable `v0.1` baseline.
- Added an in-game HUD with crosshair, hotbar, selected block label, controls panel, status line, and FPS readout.
- Switched block edits from full-world remeshing to dirty-chunk remeshing so mine/place only rebuilds affected chunks.
- Removed the legacy renderer/audio/player modules from the active code path and dropped the old `rodio` audio dependency.

## This Week — 2026-02-27
- Reworked the renderer into a staged pipeline (`shadow -> world -> post -> overlay`) with split WGSL shaders and quality presets (`Low/Medium/High/Ultra`).
- Added a title screen and pause overlay flow, plus runtime graphics controls (`F6` preset cycle, `F7` shader pack toggle, `F8` vsync toggle).
- Added soundtrack playlist support from `assets/music` (and `TOVA_MUSIC_DIR`) while keeping procedural ambient wind fallback.
- Upgraded meshing with greedy merged quads and cross-chunk face culling to reduce geometry cost.
- Replaced test terrain with seeded procedural world generation (biomes, caves, water fill) and `TOVA_WORLD_SEED` override.
- Fixed post-process binding/runtime issues on Metal and improved backend startup fallback behavior.

## This Week — 2026-02-18
- Captured a Rust-engine baseline before cleanup to preserve a stable checkpoint for upcoming refactors.
- Added procedural ambient wind audio via a new `audio` module (`rodio`), with graceful fallback when no output device is available.

## This Week — 2026-02-05
- Added a shareable "moment link" flow that deep-links camera position/view/time-of-day with open/click analytics events.
- Added an app summary PDF and a roadmap PDF to support planning and project communication.
- Tuned movement feel by slowing player speed and removing walk sway for steadier motion.
- Added day/night commands and applied world-level tweaks to improve environment control.
- Introduced performance overrides and merged castle meshes by material to reduce rendering overhead.
- Fixed Vercel deployment appearance issues and performed a bug-fix commit scan.

## 0.01 ALPHA — 2026-02-03
- Shipped share UI with Web Share API plus clipboard/download fallbacks and analytics hooks.
- Added fullscreen toggle and deterministic testing hooks to stabilize local testing.
- Refined player movement and world tuning for improved feel and readability.
- Introduced day/night commands and a redesigned HUD with icon-only UI plus optional vignette/grain.
- Rebuilt the Town into a cohesive European fantasy village and later simplified the layout for clarity.
- Reduced draw calls and runtime overhead by merging meshes and freezing static transforms.
- Improved castle mesh batching and optimized environment color allocations and HUD update cadence.
