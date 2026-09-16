#!/usr/bin/env bash

set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUTPUT_DIR="${PROJECT_ROOT}/output"
ARCHIVE_NAME="DomainEgress-source-$(date '+%Y%m%d-%H%M%S').tar.gz"
ARCHIVE_PATH="${OUTPUT_DIR}/${ARCHIVE_NAME}"

mkdir -p "${OUTPUT_DIR}"

# Keep the archive limited to Rust/Vue source and the files required to restore
# their dependency/build configuration. Build output, dependencies, caches and
# local runtime data are intentionally not included.
tar -czf "${ARCHIVE_PATH}" \
  -C "${PROJECT_ROOT}" \
  --exclude='./target' \
  --exclude='./vue3/node_modules' \
  --exclude='./vue3/dist' \
  --exclude='./vue3/.vite' \
  --exclude='./.DS_Store' \
  --exclude='*/.DS_Store' \
  vue3/index.html \
  vue3/package.json \
  vue3/package-lock.json \
  vue3/tsconfig.json \
  vue3/vite.config.ts \
  vue3/src \
  vue3/public \
  vue3/src-tauri

echo "已创建源代码归档：${ARCHIVE_PATH}"
echo "归档内容："
tar -tzf "${ARCHIVE_PATH}"
