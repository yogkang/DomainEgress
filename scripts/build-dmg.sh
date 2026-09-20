#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PROJECT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
APP_SOURCE="$PROJECT_DIR/vue3/src-tauri/target/release/bundle/macos/DomainEgress.app"
DOC_SOURCE="$PROJECT_DIR/vue3/src-tauri/resources/DomainEgress-使用说明.md"
DMG_DIR="$PROJECT_DIR/vue3/src-tauri/target/release/bundle/dmg"
VERSION=$(sed -n 's/^version = "\([^"]*\)"$/\1/p' "$PROJECT_DIR/vue3/src-tauri/Cargo.toml" | head -n 1)
ARCH=$(uname -m)
case "$ARCH" in
  arm64|aarch64) ARCH="arm64" ;;
  x86_64|amd64) ARCH="x86_64" ;;
  *) echo "不支持的 macOS 架构：$ARCH" >&2; exit 1 ;;
esac
DMG_PATH="$DMG_DIR/DomainEgress_${VERSION}_${ARCH}.dmg"
STAGE=$(mktemp -d)
trap 'rm -rf "$STAGE"' EXIT

mkdir -p "$DMG_DIR"
codesign --force --deep --sign - "$APP_SOURCE"
cp -R "$APP_SOURCE" "$STAGE/DomainEgress.app"
cp "$DOC_SOURCE" "$STAGE/DomainEgress-使用说明.md"
diskutil image create from --volname "DomainEgress" --format UDZO "$STAGE" "$DMG_PATH" >/dev/null
echo "已生成：$DMG_PATH"
