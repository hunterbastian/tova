# AGENTS.md

## Project Scope
Tova is a native Rust + `wgpu` voxel engine located in `/Users/hunterbastian/Desktop/Code/tova/tova-engine`.

## Prerequisites

- Rust toolchain version: `stable` (matches CI in `.github/workflows/release.yml`).
- Required Rust targets:
  - `x86_64-unknown-linux-gnu`
  - `aarch64-apple-darwin`
  - `x86_64-apple-darwin`
  - `x86_64-pc-windows-msvc`
- OS dependencies:
  - macOS: Xcode Command Line Tools (`xcode-select --install`)
  - Ubuntu/Debian: `build-essential`, `pkg-config`, `libasound2-dev`, `libudev-dev`, `libxkbcommon-dev`, `libwayland-dev`, `libx11-dev`, `libxrandr-dev`, `libxi-dev`, `libxcursor-dev`, `libxinerama-dev`
  - Windows: Visual Studio Build Tools 2022 with Desktop C++ workload

### Setup Command Block
```sh
# Ubuntu/Debian only:
sudo apt-get update && sudo apt-get install -y \
  build-essential pkg-config libasound2-dev libudev-dev libxkbcommon-dev \
  libwayland-dev libx11-dev libxrandr-dev libxi-dev libxcursor-dev libxinerama-dev

# macOS only (run once if needed):
# xcode-select --install

rustup toolchain install stable
rustup default stable
rustup component add rustfmt clippy
rustup target add x86_64-unknown-linux-gnu aarch64-apple-darwin x86_64-apple-darwin x86_64-pc-windows-msvc
```

### Rust Update Tracking
Use this from repo root to update Rust and refresh the tracked metadata used by the app title:

```sh
cd /Users/hunterbastian/Desktop/Code/tova
./scripts/update_rust_toolchain.sh
cat /Users/hunterbastian/Desktop/Code/tova/tova-engine/rust-toolchain-status.txt
```

## Core Commands

Run these from `/Users/hunterbastian/Desktop/Code/tova/tova-engine`:

```sh
cargo run
cargo run --release
cargo build
cargo build --release
cargo check
cargo test
cargo clippy --all-targets --all-features
```

## Workflows

### 1) Local Development Loop
```sh
cd /Users/hunterbastian/Desktop/Code/tova/tova-engine
cargo check
cargo run
```

### 2) Pre-PR Verification
```sh
cd /Users/hunterbastian/Desktop/Code/tova/tova-engine
cargo fmt --all --check
cargo clippy --all-targets --all-features
cargo test
cargo build --release
```

### 3) Release Workflow (GitHub Actions)
This repo ships binaries through `.github/workflows/release.yml` when a tag matching `v*` is pushed.

```sh
cd /Users/hunterbastian/Desktop/Code/tova
git tag -a v0.1.0 -m "v0.1.0"
git push origin v0.1.0
```

The workflow builds and publishes:
- Linux: `x86_64-unknown-linux-gnu`
- macOS Apple Silicon: `aarch64-apple-darwin`
- macOS Intel: `x86_64-apple-darwin`
- Windows: `x86_64-pc-windows-msvc`

### 4) Changelog Update Workflow
Before tagging a release, add a dated entry in `/Users/hunterbastian/Desktop/Code/tova/CHANGELOG.md`:

```sh
cd /Users/hunterbastian/Desktop/Code/tova
git add CHANGELOG.md
git commit -m "docs: update changelog"
```

## Notes
- Current release automation is tag-driven only (`push.tags: v*`).
- `rodio` audio is enabled in the engine and should degrade gracefully if no output device is available.
