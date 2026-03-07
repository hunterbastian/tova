# AGENTS.md

## Project Scope
Tova is a web-first Three.js game located in `/Users/hunterbastian/Desktop/Code/tova/tova-web`.
The older Rust + `wgpu` prototype in `/Users/hunterbastian/Desktop/Code/tova/tova-engine` is archived reference code, not the active product surface.

## Prerequisites

- Node.js 20+ with `npm`
- A modern browser with WebGL support
- Optional: Rust toolchain only if you need to inspect the archived native prototype

### Setup Command Block
```sh
cd /Users/hunterbastian/Desktop/Code/tova/tova-web
npm install
```

### Archived Native Tracking
Only use this if you intentionally work in the archived Rust prototype:

```sh
cd /Users/hunterbastian/Desktop/Code/tova
./scripts/update_rust_toolchain.sh
cat /Users/hunterbastian/Desktop/Code/tova/tova-engine/rust-toolchain-status.txt
```

## Core Commands

Run these from `/Users/hunterbastian/Desktop/Code/tova/tova-web`:

```sh
npm run dev
npm run build
```

## Workflows

### 1) Local Development Loop
```sh
cd /Users/hunterbastian/Desktop/Code/tova
./scripts/run_tova_web.sh
```

### 2) Pre-PR Verification
```sh
cd /Users/hunterbastian/Desktop/Code/tova/tova-web
npm run build
```

### 3) Release Workflow (GitHub Actions)
No automated release workflow is currently the default for the web build. Treat `tova-web` as the maintained runtime and wire hosting/deployment explicitly when you are ready to publish.

```sh
cd /Users/hunterbastian/Desktop/Code/tova
npm --prefix tova-web run build
```

### 4) Changelog Update Workflow
Before cutting a new web build or major milestone, add a dated entry in `/Users/hunterbastian/Desktop/Code/tova/CHANGELOG.md`:

```sh
cd /Users/hunterbastian/Desktop/Code/tova
git add CHANGELOG.md
git commit -m "docs: update changelog"
```

## Notes
- The supported runtime is the browser client in `tova-web/`.
- Keep `CHANGELOG.md` and `progress.md` current as web work lands.
- Treat `tova-engine/` as archived unless a future task explicitly revives it.
