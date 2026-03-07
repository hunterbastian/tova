#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "Starting Tova web on http://127.0.0.1:4174" >&2

cd "${REPO_ROOT}/tova-web"
npm run dev -- "$@"
