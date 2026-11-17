#!/bin/sh
# Receive the CI-only font configuration on stdin; do not install it in /app.
set -eu
config=$(mktemp)
trap 'rm -f "$config"' EXIT
cat > "$config"
FONTCONFIG_FILE="$config" "$@"
