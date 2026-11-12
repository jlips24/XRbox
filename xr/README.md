# XRBox Godot foundation

Linux first; one shared Godot project with separate native Linux and Android ABIs.
Android now has an implemented live Remote Play path; device acceptance is pending.
Host/Linux previews remain synthetic and never start authentication. The existing
desktop client remains the validated streaming harness.

Implemented: in-process Rust GDExtension version/target diagnostics, optional OpenXR
initialization with labeled desktop fallback, an unshaded test-pattern screen,
controller/keyboard menu navigation, session-only width/distance controls,
level recentering, focus recovery and hidden-menu redraw suppression.

## Tools and local preview

Pinned tools: Godot **4.7.2 stable**, godot-rust **0.5.5** using API **4.7**,
project Rust **1.99.1**. See toolchain.json and Cargo.lock. Godot/export templates
must match. The Android debug export now uses OpenXR Vendors 5.1.0-stable
and the toolchain recorded in toolchain.json; Quest device testing is pending.

On this development Mac the authorized download is project-local:
`.tools/godot/Godot.app/Contents/MacOS/Godot`. The download was verified against the
official release SHA512; self-contained editor mode keeps editor settings in the
local tool directory. Nothing is installed into Applications or added to PATH.
On another machine, obtain Godot from its official archive and supply its path:

```sh
GODOT=/absolute/path/to/godot make xr-check
GODOT=/absolute/path/to/godot make xr-preview
```

With the local Mac download, simply run:

```sh
make xr-check       # native build, headless import, scene/input/lifecycle tests
make xr-preview     # desktop synthetic screen; no OpenXR initialization
make xr-run         # attempts the host OpenXR runtime, otherwise labels preview
```

Use arrows/D-pad to select, Enter/A to activate, Escape/B to hide/show the menu.
Select + Start + LB + RB also opens the prototype menu. Physical Guide is unused.
The prototype does not send game input; live streaming will use the extracted
core's deferred shortcut recognizer, not a second gameplay recognizer in GDScript.
Screen adjustments currently last only for this scene/session. The 3D menu
uses keyboard/controller navigation; pointer picking is not implemented.

make xr-build honors CARGO_TARGET_DIR and stages only the selected target/profile
under ignored native/ directories. Desktop make build retains its existing meaning.
Restart Godot after a native rebuild; extension hot reload is disabled.

## Linux first: build and headset procedure

On a Linux ARM64 development environment with the pinned Rust toolchain and
Godot 4.7.2 ARM64 executable:

```sh
GODOT=/absolute/path/to/godot make xr-check
GODOT=/absolute/path/to/godot make xr-preview
GODOT=/absolute/path/to/godot make xr-run
```

For cross-building a native library, explicitly configure a compatible Linux
ARM64 linker/sysroot and install the Rust target into the pinned toolchain cache.
Adding the Rust target alone does not provide a linker/sysroot:

```sh
make xr-build ARGS='--target aarch64-unknown-linux-gnu'
make xr-build ARGS='--target aarch64-unknown-linux-gnu --release'
```

The Linux ARM64 export preset expects both relevant native builds staged, plus
matching Godot export templates. Once those prerequisites are present:

```sh
mkdir -p dist/xr/linux-arm64
/absolute/path/to/godot --headless --path xr --export-debug 'Linux ARM64'
# Optimized export after building/staging the release extension:
/absolute/path/to/godot --headless --path xr --export-release 'Linux ARM64'
```

The Linux live decoder/import adapter is not enabled for this gate. Deployment must
include the exported executable, PCK and generated native-library dependencies;
do not copy only the executable. The export has not been built/verified here.
Use Valve's supported developer deployment workflow; no automatic headset install.

On Frame, record build/revision, OS/runtime, renderer and exposed XR extensions.
Verify the headset shows stereo output and head tracking, the screen remains
world-locked, A/B/D-pad navigation works, width/distance/recenter/reset work, and
headset removal/focus loss/return/disconnect/quit recover without stale controls.
Check the menu in the headset itself, not just a spectator window. Stop before
real account/console testing: hardware decoder/import integration is the next gate.

Flatpak remains the preferred Linux release package. Its manifest/bundle and
host OpenXR IPC/driver/decoder/controller/credential permissions are not yet
implemented or qualified. Compare unpackaged and sandboxed bring-up early.

## Android / Quest 3S debug APK

A debug-signed ARM64 APK is now built and statically verified. See
[ANDROID.md](ANDROID.md) for exact Quest 3S installation/retest steps and build
prerequisites. Android uses the shared scene with a Compatibility renderer
override; Linux retains its Mobile renderer setting.

```sh
make xr-apk         # build/sign/verify, no device install
make xr-apk-check   # verify the existing APK
```

The Android build includes native console authentication/discovery, WebRTC,
MediaCodec-to-EGL video, Opus/CPAL audio and shared deferred controller shortcuts.
See ANDROID.md for the exact hardware test. The host remains synthetic. No real
account, console or headset was accessed while implementing this build.

## Validation scope

Headless tests cover the native class, scene creation/recreation, bounded geometry,
16:9 dimensions, level recentering, default D-pad/A/B/key mappings, menu focus,
focus loss/recovery and hidden-menu redraw behavior. They do not verify XR or GPU
surfaces. tests/render.gd is an optional short GPU smoke that saves an ignored
.tools/godot-preview.png when run with --audio-driver Dummy, --xr-mode off and
user argument --test. It is excluded from release exports.

The Android debug APK builds and passes package checks. No Linux/Android
executable launch, Flatpak, headset XR presentation, hardware video, real
controller, audio or battery result is established by these tests.
See ../XR_PLAN.md and ../VALIDATION.md for progress and evidence.
