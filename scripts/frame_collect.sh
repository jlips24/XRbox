#!/bin/sh
# Manual read-only capability collection on Frame. No sign-in, streaming, installs,
# device configuration, camera capture, network-address or credential collection.
set -u
printf '%s\n' 'XRbox Frame capability report — advertised capabilities, not validation'
printf '\nArchitecture: '; uname -m
printf 'Kernel: '; uname -r
if command -v getconf >/dev/null 2>&1; then getconf GNU_LIBC_VERSION; fi
if [ -r /etc/os-release ]; then
    sed -n '/^PRETTY_NAME=/p; /^VERSION_ID=/p; /^BUILD_ID=/p' /etc/os-release
fi
printf '\nVulkan summary (physical GPU, not extensions enabled in Godot):\n'
if command -v vulkaninfo >/dev/null 2>&1; then
    vulkaninfo --summary 2>/dev/null | awk '/Vulkan Instance Version|deviceName|deviceType|apiVersion|driverVersion|driverName|driverInfo/ { print }'
else
    printf '%s\n' 'NOT AVAILABLE: vulkaninfo. Do not install packages just for this collection.'
fi
printf '\nVideo nodes (sysfs names only; cameras are not opened):\n'
for entry in /sys/class/video4linux/video*/name; do
    [ -r "$entry" ] || continue
    card=$(cat "$entry")
    node=$(basename "$(dirname "$entry")")
    printf '%s: %s\n' "$node" "$card"
    case "$card" in
        *[Dd]ecod*|*iris*|*venus*)
            if command -v v4l2-ctl >/dev/null 2>&1; then
                v4l2-ctl --device "/dev/$node" --list-formats-out --list-formats 2>&1 || true
            else
                printf '%s\n' 'NOT AVAILABLE: v4l2-ctl. No decoder was configured or started.'
            fi
            ;;
    esac
done
printf '\nFFmpeg build capabilities (not proof of a usable hardware decoder):\n'
if command -v ffmpeg >/dev/null 2>&1; then
    ffmpeg -hide_banner -decoders 2>/dev/null | awk '/h264|v4l2/ { print }'
    ffmpeg -hide_banner -hwaccels 2>/dev/null
else
    printf '%s\n' 'NOT AVAILABLE: ffmpeg CLI. The app libraries and system CLI are separate.'
fi
printf '\nNo account, console, decoder stream or camera capture was started.\n'
