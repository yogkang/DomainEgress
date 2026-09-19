#!/usr/bin/env bash
set -euo pipefail

# Run from the repository root. Pass a verified upstream commit or tag explicitly.
if [ "$#" -ne 1 ]; then
  echo "Usage: $0 <ip2region-commit-or-tag>" >&2
  exit 64
fi

repo_root=$(cd "$(dirname "$0")/.." && pwd)
work_dir=$(mktemp -d)
trap 'rm -rf "$work_dir"' EXIT

git clone --depth 1 --branch "$1" https://github.com/lionsoul2014/ip2region.git "$work_dir/ip2region"
data_dir="$repo_root/vue3/src-tauri/resources/geoip"
mkdir -p "$data_dir"
install -m 0644 "$work_dir/ip2region/data/ip2region_v4.xdb" "$data_dir/ip2region_v4.xdb"
install -m 0644 "$work_dir/ip2region/data/ip2region_v6.xdb" "$data_dir/ip2region_v6.xdb"
git -C "$work_dir/ip2region" rev-parse HEAD
shasum -a 256 "$data_dir/ip2region_v4.xdb" "$data_dir/ip2region_v6.xdb"
