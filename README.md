# XRBox

A native Rust **Xbox console Remote Play** desktop proof of concept, intended as
the first step toward efficient standalone playback on Steam Frame. No Electron,
browser media runtime, relay PC, or separately enabled service is used.

**Desktop playback has been reported working by the user.** Version 0.1.2 adds
automatic secure sign-in restoration and a revised connection UI. Steam Frame,
battery life, and these new sign-in changes still need hardware testing.

## Run on an Apple Silicon Mac

Requires Rust/rustup and CMake for the bundled native Opus build. The repository's
rust-toolchain.toml selects Rust 1.99.1 for this project without changing your
default toolchain. The default Mac app uses VideoToolbox/Metal, with no FFmpeg
runtime dependency. All dependencies are pinned by Cargo.lock.

```sh
make build
make run
```

To create a self-contained development app:

```sh
make package
open dist/XRBox.app
```

The script builds inside the project, checks for external dynamic libraries and
ad-hoc signs the bundle. It does not install anything or alter PATH/defaults.
The package is not Developer ID signed or notarized.

## Developer commands

Run `make help` to list all targets. GNU Make or the macOS developer-tools Make is
sufficient; Python 3 is needed for packaging.

| Command | Purpose |
| --- | --- |
| `make build` / `make release` | Debug / optimized desktop executable |
| `make run` | Launch the native client |
| `make smoke` | Render ten UI frames and exit without signing in |
| `make auth-probe` | Request/discard a Microsoft device code; requires internet |
| `make test` / `make check` | Core tests / formatting and application Clippy |
| `make test-all` / `make check-all` | Every vendored crate and feature |
| `make fmt` | Format Rust sources |
| `make package` / `make package-release` | Debug / optimized macOS .app |

Full checks generate a synthetic H.264 fixture and require developer FFmpeg with
libx264 and headers, CMake and pkg-config. Network tests bind localhost; media/GPU
tests need the corresponding platform facilities. Ignored keychain and physical
audio tests are not run automatically. See DEVELOPMENT.md for direct Cargo commands.

No target installs system tools or changes PATH, shell settings, services or system
defaults. Cargo may download the pinned toolchain and build dependencies to its
normal caches. Existing environment overrides are inherited. For example, use a
command-local build cache with `CARGO_TARGET_DIR=/your/cache make build`; for an
isolated UI test use `XRBOX_DATA_DIR=/private/tmp/xrbox-smoke make smoke`.
Agent/contributor constraints are recorded in AGENTS.md.

## First console test

1. On Xbox, enable Remote Features and run its Remote Play connection checks.
   Put the Xbox and Mac on the same LAN; console Ethernet is preferable.
2. Pair the Bluetooth Xbox controller with the Mac, then open XRBox.
3. Select **Sign in with Microsoft**. **Remember sign-in: On** is the default;
   select that tile to turn it Off for session-only credentials.
4. Open Microsoft sign-in, enter the displayed device code, and finish in your
   browser. XRBox never asks for your password.
5. Consoles appear automatically after sign-in. Select your Xbox tile to play.
6. Verify moving video, game audio, sticks/triggers/buttons, D-pad, Menu and View
   on the dashboard or an installed game. GTA VI is not required for this test.
7. Play for at least ten minutes. Test focus loss, controller unplug/reconnect,
   disconnect/reconnect, close/relaunch, and sign-out. Confirm controls release
   when focus is lost and no old frame remains after disconnect.
8. Report the visible connection stage, observations, and sanitized diagnostics.
   Never share credentials, device codes, or tokens.

After relaunch, XRBox automatically refreshes saved credentials and discovers
consoles; it does not start a stream until you select a console tile. On macOS, credentials
use Keychain (service `xrbox`), never a plaintext token file. If storage is locked
or unavailable, unlock it or uncheck Remember sign-in and sign in for this session.
Revoked or expired refresh credentials require signing in again. Sign out clears
in-memory credentials and attempts to remove saved credentials; failures are shown.
The device code uses the bundled Hack monospace font with a distinct zero. **Copy
code** copies only the code when clicked; codes are never copied automatically.

The Diagnostics window separates assembled, decoded and presented frames and shows
dimensions, source FPS, incomplete-frame drops, decoder errors, queue drops,
keyframe gating, frame age, decoder callback timing, GPU import mode, controller
detection and audio-device state. Stalls trigger recovery requests and a bounded
timeout instead of leaving a frozen image labeled as streaming.
Connection status is gated on peer connectivity, input handshake and decoded media.
A connected session is not proof that its selected network route is direct/LAN.
The nominated ICE pair is sampled every five seconds. Relay routes are rejected;
private/link-local peers are labeled as LAN candidates, not proof of network
topology (VPNs also use private addresses). An unavailable pair stays unknown.

## Current limits

- The user reports desktop playback working. New controller shortcuts still need
  real-console validation; automated tests do not establish physical behavior.
- No XR renderer, passthrough, Frame package or Frame battery measurements yet.
- Linux ARM64 hardware decoding and GPU surface import are unresolved. Portable
  non-macOS compilation uses FFmpeg software decoding; it is not a Frame efficiency solution.
- macOS requires VideoToolbox hardware decode; there is no silent software fallback.
- Rumble is not validated. Physical Xbox/Guide is reserved for the host and ignored
  by console input; the virtual shortcut still needs console validation. Gameplay
  button/axis mappings reuse upstream; controller identification depends on OS access.
- CPU/memory, end-to-end latency and power are not yet
  measured. Decode-to-callback time is not input-to-display latency.
- Server session deletion is best effort within shutdown; abrupt termination may
  leave a session until the server expires it or the next connect cleans it up.

See [DEVELOPMENT.md](DEVELOPMENT.md), [ARCHITECTURE.md](ARCHITECTURE.md),
[ROADMAP.md](ROADMAP.md), [VALIDATION.md](VALIDATION.md) and [Plan.md](Plan.md). Derived from GPL-3.0-only
OpenXCloud; provenance and modifications are listed in [vendor/README.md](vendor/README.md).

## macOS Keychain password prompts

XRBox 0.1.4 keeps short-lived access, console-streaming and transfer tokens in
memory. Only refresh-token changes are sent to the secure persistent store, and
sign-out no longer clears the same Keychain item twice. Existing saved sign-ins
remain compatible; old temporary values may remain in the bundle until sign-out.

When the macOS prompt identifies XRBox accessing its `xrbox` credentials, choosing
**Always Allow** grants lasting access for that app identity; **Allow** grants only
one access. A locked login Keychain may still need unlocking. See
[Apple's explanation](https://support.apple.com/en-euro/guide/keychain-access/kyca1243/mac).
Our default development bundle is ad-hoc signed, so rebuilt versions can require
approval again. Use the same app bundle when testing relaunch. No global Keychain
permissions or security settings are changed by XRBox.

If you already have a suitable code-signing identity, the packager supports it:

```sh
python3 scripts/package_macos.py --debug --sign-identity 'Your existing signing identity'
```

This does not create certificates or notarize the app. Consistent certificate
signing supports stable app identity across updates; it does not guarantee an
unlocked Keychain or eliminate all OS prompts. Session-only sign-in remains
available by setting Remember sign-in to Off.

## Controller navigation

The home and connection screens use large Xbox-inspired tiles and a green focus
outline. Use **D-pad / left stick** to move, **A** to select, and **B** to go back,
close Diagnostics, dismiss sign-out confirmation, or cancel a pending connection.
Sign-out has a confirmation screen. Keyboard arrows/Enter/Escape and mouse remain
available. Microsoft authentication itself takes place in the system browser.

During gameplay:

| Shortcut | Action |
| --- | --- |
| **Select** (View) | Send one Select press on release |
| **Start** (Menu) | Send one Start press on release |
| **Select + Start** | Send one Xbox/Guide press |
| **Select + Start + LB + RB** | Open XRBox's stream menu instead |

Pressing either Select or Start begins a gesture; neither is sent to the console
on button-down. The most specific completed shortcut is remembered, and it fires
when the **first Select/Start button pressed is released**. Releasing the other
button or bumpers first does not downgrade or trigger it. Hold Select+Start while
adding LB/RB, then release the initiating button to open XRBox without sending Guide.
If both Select and Start first appear in the same sample, Select is treated as first.

Single Select, single Start and virtual Guide use the same 120 ms protocol-button
press followed by release. Long holds produce one short press on release; held
Select/Start or Guide long-press behavior is not implemented. Rapid taps are queued
with a release gap so repeated presses remain distinct (bounded to 16 queued taps;
overflow is discarded with a sanitized warning). No OS key events are generated.

Menu navigation stays local and sends neutral gameplay input while open. Resume
with A or B. All other buttons (including LB/RB), sticks and triggers pass through
immediately on resume; **only Select/View and Start/Menu** remain suppressed until
both are released. Bumpers and other controls remain live while constructing a
shortcut; **Select and Start are always buffered**, so their individual presses
cannot leak into a combined shortcut. Ordinary gameplay input is not buffered.

The **physical Xbox/Guide button is ignored by XRBox's console-input mapping**.
It may still open the host OS/Steam overlay. XRBox does not alter system shortcuts
or enable upstream system-gesture capture. Steam documents its
[global Guide chords](https://partner.steamgames.com/doc/features/steam_controller/getting_started_for_players?l=english),
and Windows uses the [Xbox button for Game Bar](https://support.microsoft.com/en-us/servicing/os/windows/docs/2025/10/windows-gaming-full-screen-experience).
Apple offers a [system-gesture preference](https://developer.apple.com/documentation/gamecontroller/gccontrollerelement/preferredsystemgesturestate)
but explicitly does not guarantee that it is respected. These shortcuts avoid
needing the reserved physical button; OS-specific behavior still needs testing.

The app samples one controller source for both menu and gameplay. Idle controller
polling is 30 Hz; active streaming retains the 125 Hz sampling rate. Idle menus
repaint on input changes and navigation repeat rather than continuously. These are
implementation settings, not measured latency or battery improvements.
