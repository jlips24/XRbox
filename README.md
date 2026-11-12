# XRbox

Play your Xbox console on a virtual screen in a standalone XR headset. XRbox uses
Godot, OpenXR and a native Rust streaming core. It connects to your own console
through Xbox Remote Play, without a relay PC or an embedded browser runtime.

**Early development:** the Android streaming implementation builds, but playback
on a real console and headset still needs validation. Meta Quest 3S is the first
Android test target. Steam Frame / Linux ARM64 support is in development; there
is no ready-to-install Frame or Flatpak release yet. Xbox Cloud Gaming is not
supported.

## Install on Meta Quest

### What you need

- A Quest headset with developer mode and USB debugging enabled. Quest 3S is the
  initial target; other Android headsets have not been qualified.
- An Xbox Series X/S with Remote Features enabled and your Microsoft account
  configured for console Remote Play.
- A Bluetooth Xbox controller paired with the headset. Touch controllers and
  hand tracking are not supported.
- Both the console and headset on the same local network.
- A computer with Android Platform Tools (`adb`) for installation only.

### Install the APK

The current development package is **0.1.10-xr.2**. APKs are build outputs, not
included in this repository. If you do not already have one, follow the
[Android build instructions](xr/ANDROID.md#rebuild-on-this-mac) before continuing.
No public download is assumed by these instructions.

Connect the Quest by USB, accept the debugging prompt inside the headset, then
run these commands from the repository root:

```sh
adb devices
adb install -r dist/xr/android/XRbox-0.1.10-xr.2-quest-debug.apk
```

If `adb devices` shows `unauthorized`, check the headset for its permission prompt.
Open **XRbox XR Prototype** from the Quest's **Unknown Sources** app library.
Confirm the displayed version and **OpenXR active** status. The computer can be
disconnected after installation; it is not part of the streaming pipeline.

## Connect and play

1. Leave your Xbox awake for the first test and open a game or moving dashboard
   screen so you can tell whether video is updating.
2. Use the controller's **D-pad** to navigate and **A** to select **Sign in**.
3. Enter the displayed code at the Microsoft verification URL on your phone or
   computer. Complete sign-in, then return to XRbox.
4. Select your discovered Xbox to connect.
5. Confirm the picture moves, audio plays and controller input reaches the console.

Remembered sign-in is enabled by default. You can choose session-only sign-in
before starting. Only refresh credentials are stored securely; temporary tokens
stay in memory. Sign out to remove remembered credentials.

### Controls

| Input | Action |
| --- | --- |
| D-pad / A | Navigate / select in XRbox menus |
| Select (View) | Send Select when released |
| Start (Menu) | Send Start when released |
| Select + Start | Send the virtual Xbox Guide button |
| Select + Start + LB + RB | Open XRbox's menu |

For shortcuts, hold the combination and release the **first Select/Start button
pressed** to trigger it. The physical Xbox/Guide button stays reserved to the
headset. B remains game input during streaming; use the menu shortcut to return
to XRbox. The stream menu includes resume, disconnect and screen adjustments.

## Current limits and troubleshooting

- Opening the headset system menu, removing the headset or losing XR focus
  disconnects an active stream. Reconnect manually on return.
- Screen size, distance, recenter and vertical-flip controls are available;
  screen adjustments currently last only for the session. Full settings and
  custom hotkey editing have not been ported to XR.
- Steam Frame media integration, passthrough and other-headset compatibility
  remain unfinished. Host/Linux Godot previews show synthetic content.
- Hardware decoding, XR output, secure sign-in restoration, audio and physical
  controls still need device acceptance. Latency, battery life and zero-copy
  behavior have not been measured.

If connection or playback fails, record the app version, headset OS, visible
error and media-stage counters. Counters alone do not prove visible playback.
Never share sign-in codes, tokens, raw SDP or account details. See the
[Quest testing and diagnostic guide](xr/ANDROID.md#install-and-test) for lifecycle
checks and app-only log capture.

## Build and contribute

The project uses pinned Rust **1.99.1**, Godot **4.7.2** and Cargo.lock. macOS is
supported as a development host for synthetic previews and Android builds; the
standalone 2D desktop app is retired. Linux and Android remain the product targets.
No helper installs system tools or changes PATH/defaults.

Start with [DEVELOPMENT.md](DEVELOPMENT.md) for prerequisites and checks,
[xr/ANDROID.md](xr/ANDROID.md) for APK tooling, and [xr/README.md](xr/README.md)
for Godot preview and Linux export instructions.

```sh
make build       # build/stage the host Godot extension
make xr-preview  # synthetic preview, no account or console access
make test        # Rust core, bridge and selected protocol tests
make check       # formatting and core/bridge lint
make test-all    # all Rust, Python, headless Godot and GPU smoke tests
make xr-apk      # build/sign/verify Android APK; does not install it
```

`make test-all` saves full logs under ignored `.tools/test-results/` and prints a
compact final summary. It needs the full native toolchain, pinned Godot and a functioning
GPU/display session. Its render smoke briefly opens a window. Hardware-only and
explicitly ignored credential/audio-device checks remain manual. Run `make help`
for individual test and build targets.

### Source layout

| Location | Responsibility |
| --- | --- |
| `crates/xrbox-core/` | Engine-independent authentication, session, media and controller logic |
| `crates/xrbox-godot/` | In-process Godot bridge and GPU presentation adapters |
| `xr/` | Godot scenes, controller UI, OpenXR and Android platform glue |
| `vendor/openxcloud/` | Pinned reusable protocol/media libraries |
| `scripts/` | Build, packaging and validation helpers |

The core has no Godot dependency. These components share a repository so interface
changes and tests can land together. Read [AGENTS.md](AGENTS.md) before automated
contributions.

For design and project status, see [ARCHITECTURE.md](ARCHITECTURE.md),
[XR_PLAN.md](XR_PLAN.md), [ROADMAP.md](ROADMAP.md) and
[VALIDATION.md](VALIDATION.md).

## License

XRbox is licensed under **GPL-3.0-only**. See [LICENSE](LICENSE),
[third-party notices](THIRD_PARTY_NOTICES.md) and the
[OpenXCloud provenance and patch notes](vendor/README.md).
