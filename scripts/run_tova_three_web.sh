#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "./scripts/run_tova_three_web.sh is now a compatibility alias." >&2
echo "Use ./scripts/run_tova_web.sh for the single supported Tova web client." >&2

exec "${REPO_ROOT}/scripts/run_tova_web.sh" "$@"
