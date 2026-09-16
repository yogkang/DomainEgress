#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PROJECT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
APP_SOURCE="$PROJECT_DIR/vue3/src-tauri/target/release/bundle/macos/DomainEgress.app"
APP_DEST="/Applications/DomainEgress.app"

# 先生成最新 Vue 桌面包，再覆盖安装到系统应用目录。
"$SCRIPT_DIR/package-vue.sh"

mkdir -p "$APP_DEST"
rsync -a --delete "$APP_SOURCE/" "$APP_DEST/"
echo "已安装：$APP_DEST"
open "$APP_DEST"
