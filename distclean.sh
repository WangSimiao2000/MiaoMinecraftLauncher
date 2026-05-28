#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"

echo "Cleaning build artifacts in $ROOT ..."

rm -rf "$ROOT/target"

rm -rf "$ROOT/dist"

find "$ROOT" -name "*.d" -path "*/deps/*" -delete 2>/dev/null || true

echo "Done. Run 'cargo build' to rebuild."
