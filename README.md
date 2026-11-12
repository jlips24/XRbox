# XRbox

Standalone Xbox **console Remote Play** for XR headsets. Godot provides the
OpenXR frontend; a native Rust core handles Microsoft authentication, console
discovery, WebRTC, video/audio and controller input. No Electron, embedded browser
runtime, cloud gaming or relay PC is used.

Targets are **Steam Frame / Linux ARM64** and **Android ARM64**, starting with
Meta Quest 3S. Android build **0.1.10-xr.2** implements live streaming but still
needs real console/headset testing. Linux Godot previews remain synthetic;
hardware decoding/import and Flatpak deployment are pending. No battery,
latency or zero-copy improvements have been measured.

## Repository layout

| Location | Responsibility |
| --- | --- |
| `crates/xrbox-core/` | Engine-independent auth, configuration, session ownership, media and controller routing |
| `crates/xrbox-godot/` | In-process GDExtension, typed input mapping and Android GPU presentation |
| `xr/` | Godot scenes, controller UI, OpenXR presentation and Android platform glue |
| `vendor/openxcloud/` | Pinned reusable protocol/media libraries and documented narrow patches |
| `scripts/` | Reproducible build, packaging and validation helpers |

Keep these boundaries in one repository: interface changes, dependency pins and
regression tests can evolve atomically. The core never depends on Godot. The
standalone 2D desktop client has been retired; it remains in Git history.
macOS remains supported as a **development host**, not a shipping app target.

## Develop

Use the pinned Rust toolchain and Cargo.lock. Godot **4.7.2**, native prerequisites
and command-local tool overrides are described in [DEVELOPMENT.md](DEVELOPMENT.md).
No command installs system tools or changes PATH/defaults.

```sh
make build       # build/stage host extension
make xr-check    # native bridge and headless Godot checks, no account access
make xr-preview  # synthetic desktop preview
make test
make check
make xr-apk      # build/sign/verify Android APK, no device install
```

`make help` lists all tasks. Host previews never restore credentials or start a
console session. Live Android mode supports device-code sign-in, remembered or
session-only credentials, console selection, video/audio and a Bluetooth Xbox
controller. Touch/hand controls and full XR settings are not implemented.

## Test on Quest

Follow [xr/ANDROID.md](xr/ANDROID.md) for prerequisites, installation, sign-in and
exact video/audio/controller/lifecycle acceptance steps. Do not infer playback
from network or decode counters. Physical Guide remains host-reserved; Select +
Start sends virtual Guide, and Select + Start + LB + RB opens XRbox's menu. The
shortcut resolves when the first Select/Start pressed is released.

Only refresh credentials persist in the OS secure store; temporary tokens remain
in memory. Never share tokens, sign-in codes, raw SDP or personal account details.

See [ARCHITECTURE.md](ARCHITECTURE.md), [XR_PLAN.md](XR_PLAN.md),
[VALIDATION.md](VALIDATION.md) and [ROADMAP.md](ROADMAP.md). GPL-3.0-only;
provenance and patches are recorded in [vendor/README.md](vendor/README.md), with
[third-party notices](THIRD_PARTY_NOTICES.md).
