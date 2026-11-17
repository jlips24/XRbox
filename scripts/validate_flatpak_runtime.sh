#!/bin/sh
# Run inside the Platform runtime, never against host Ubuntu's libraries.
set -eu
app_root=${1:-/app}
ldd_command=${2:-ldd}
test -s "$app_root/lib/xrbox/xrbox.arm64"
test -s "$app_root/lib/xrbox/xrbox.pck"
list=$(mktemp)
trap 'rm -f "$list"' EXIT
# Debug companions are not loadable ELF objects. Inspect real versioned files,
# rather than symlink aliases, and keep failures out of find's predicate status.
find "$app_root/lib" -path "$app_root/lib/debug" -prune -o \
    -type f ! -name '*.debug' \( -name '*.so' -o -name '*.so.[0-9]*' \) -print > "$list"
check_binary() {
    printf '\nChecking runtime dependencies: %s\n' "$1"
    if output=$("$ldd_command" "$1" 2>&1); then
        printf '%s\n' "$output"
        case "$output" in
            *'not found'*) return 1 ;;
        esac
    else
        code=$?
        printf '%s\nldd failed with exit %s: %s\n' "$output" "$code" "$1" >&2
        return 1
    fi
}
status=0
check_binary "$app_root/lib/xrbox/xrbox.arm64" || status=1
while IFS= read -r library; do
    check_binary "$library" || status=1
done < "$list"
exit "$status"
