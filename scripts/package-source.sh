#!/usr/bin/env bash
set -euo pipefail
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUTPUT_DIR="${1:-${PROJECT_ROOT}/output}"
mkdir -p "$OUTPUT_DIR"
OUTPUT_DIR="$(cd "$OUTPUT_DIR" && pwd)"
ARCHIVE_PATH="$OUTPUT_DIR/DomainEgress-source-$(date '+%Y%m%d-%H%M%S').tar.gz"
# Include build scripts, CI, licensing, and the canonical bundled user guide.
tar -czf "$ARCHIVE_PATH" -C "$PROJECT_ROOT" \
  --exclude='*/target' --exclude='*/node_modules' --exclude='*/dist' \
  --exclude='*/.vite' --exclude='*/.DS_Store' \
  .gitignore .github LICENSE THIRD_PARTY_NOTICES.md README.md \
  package.json package-lock.json tsconfig.json vite.config.ts index.html \
  src src-tauri public docs scripts homebrew
printf '已创建源代码归档：%s\n' "$ARCHIVE_PATH"
