# Changelog

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
