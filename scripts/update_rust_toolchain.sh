#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
status_file="$repo_root/tova-engine/rust-toolchain-status.txt"

rustup update stable
rustup default stable
rustup component add rustfmt clippy
rustup target add x86_64-unknown-linux-gnu aarch64-apple-darwin x86_64-apple-darwin x86_64-pc-windows-msvc

rustc_version="$(rustc -V)"
updated_at="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

cat > "$status_file" <<EOF
channel=stable
rustc=$rustc_version
updated_at=$updated_at
EOF

echo "Rust toolchain updated: $rustc_version"
echo "Tracker updated: $status_file"
