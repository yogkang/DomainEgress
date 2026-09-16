#!/bin/sh
set -eu
cd "$(dirname "$0")/../vue3"
if [ ! -d node_modules ]; then npm ci; fi
exec npm run desktop
