#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PROJECT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
APP_SOURCE="$PROJECT_DIR/vue3/src-tauri/target/release/bundle/macos/DomainEgress.app"
DOC_SOURCE="$PROJECT_DIR/vue3/src-tauri/resources/DomainEgress-使用说明.md"
DMG_DIR="$PROJECT_DIR/vue3/src-tauri/target/release/bundle/dmg"
DMG_PATH="$DMG_DIR/DomainEgress_0.1.0_aarch64.dmg"
STAGE=$(mktemp -d)
trap 'rm -rf "$STAGE"' EXIT

mkdir -p "$DMG_DIR"
cp -R "$APP_SOURCE" "$STAGE/DomainEgress.app"
cp "$DOC_SOURCE" "$STAGE/DomainEgress-使用说明.md"
hdiutil create -volname "DomainEgress" -srcfolder "$STAGE" -ov -format UDZO "$DMG_PATH" >/dev/null
echo "已生成：$DMG_PATH"
