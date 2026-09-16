#!/bin/sh
set -eu
cd "$(dirname "$0")/../vue3"
npm ci
npm run package
