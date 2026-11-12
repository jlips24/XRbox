# Android live implementation validation — November 12, 2026

## Desktop retirement checks — November 12, 2026

- `make test`: **221 passed, zero failed, one ignored** (credential-store test).
  Initial sandbox execution blocked six fake-HTTP localhost tests; rerun with
  localhost access passed. No live account/console operations occurred.
- `make check`: formatting and strict core/bridge Clippy passed.
- `make xr-check`: host extension built/staged; four native bridge tests and
  headless Godot scene/UI checks passed without credentials.
- Four Android packaging helper tests passed; locked workspace metadata resolves
  without the deleted desktop member. `make help` and diff whitespace checks pass.
- No APK rebuild or device test was needed for removing the separate executable;
  the previously verified xr.2 APK remains the hardware-test artifact.

## Product scope update — November 12, 2026

The standalone 2D desktop client and macOS .app packager are retired by user
request. macOS remains a development host. Keep the engine-independent Rust core,
Godot adapter and XR project in separate folders in this repository, with atomic
interface/test changes. Earlier desktop harness instructions/results below are
historical; current commands are in README.md and DEVELOPMENT.md. Headset targets
remain Linux ARM64 and Android ARM64. Hardware acceptance is still pending.


The following results concern build 0.1.10-xr.2 on macOS Apple Silicon. No real
account authorization, stored credentials, Xbox, headset, physical controller or
physical audio device was accessed. Host Godot tests suppress live startup.

- Full workspace/all-feature serial tests: **564 passed, zero failed, two ignored**
  (OS credential access and physical audio). Includes opaque-surface output policy,
  input mapping/reconnect neutralization and GPU lease retirement ordering.
- Full workspace/all-target/all-feature strict Clippy and formatting: passed.
- Android-specific strict Clippy for the bridge, core and affected upstream
  API/input/media crates: passed with the NDK ARM64 toolchain.
- `make xr-check`: passed, including four Rust bridge tests and fake-state Godot
  menus, pagination, long diagnostics/code layout, gameplay B and shutdown.
- Four Python packaging regression tests and Python compilation: passed.
- `make xr-apk`: passed; signature, ARM64-only libraries, 16 KiB alignment,
  one XR loader, Java/JNI bootstrap, Internet permission and key exclusion verified.
  APK size **108,496,044 bytes**; SHA256
  `6e9bd35d001f78224331d313335d68283d39ddc1a2b29e1bf76eb79a767bcd5a`.
  These checks establish build correctness, not successful device playback.

Manual acceptance is in [xr/ANDROID.md](xr/ANDROID.md). Confirm each stage separately:
auth/discovery, media assembly, actual hardware decode, visible moving XR video,
audible audio, game controls and lifecycle/persistence. Direct LAN behavior,
zero-copy, frame pacing, latency and battery remain unverified.

# Executed validation — November 11, 2026

Environment: macOS ARM64 (Apple Silicon), project-pinned Rust 1.99.1.
No real account authorization, stored Xbox credentials, Xbox console, or Steam
Frame was used. No PATH, shell defaults, default applications, global Git
configuration, or default Rust toolchain was changed. Git was initialized only
inside the project. Toolchain/dependency downloads used normal build caches;
a command-local CARGO_TARGET_DIR reused the temporary audit build directory.

## Results

- Native XRBox desktop build: passed.
- Full workspace, all targets/features, serial tests: **516 passed, zero failed,
  two ignored**. Ignored tests require OS-keychain access or physical audio output.
- Targeted WebRTC/application tests after extending selected-route observation to
  media and SCTP transports: **13 passed, zero failed** (plus empty doctest suite).
- Full workspace strict Clippy (`-D warnings`): passed.
- Formatting check: passed.
- Packaged executable UI smoke: ten native UI frames rendered and clean exit.
- Live Microsoft device-code initiation probe: succeeded. Code was discarded;
  no browser sign-in, refresh token, or persistent credential was acquired.
- macOS development .app packaging: built, external dylib check passed, ad-hoc
  code signature verified. No Homebrew/non-system dynamic library is needed by
  the default Mac executable. Clean-machine and notarized distribution testing
  remain outstanding.

## Notable regression coverage

- Home-only authentication makes no cloud offering requests.
- OAuth slow_down increases polling interval and continues.
- Home sessions perform transfer-token authentication when ReadyToConnect is
  returned, then wait for Provisioned before negotiation.
- Home start request uses /home/play and serverId, with empty titleId.
- Peer connectivity and newly decoded video are both required for app readiness.
- Authentication error summaries and application errors do not expose raw tokens.
- Invalid console selection cannot create media; disconnect/shutdown clear local
  state. Upstream tests cover negotiation failure, cancellation, cleanup, media
  queues, controller mappings, neutral input, and native WebRTC loopback.
- Nominated-pair access is exercised over localhost; route classification treats
  relay candidates as relayed and handles private addresses without logging them.

An upstream decoder test initially raced its own fake-decoder log against frame
publication. Its wait now synchronizes on actual mailbox publication; the patch
is recorded in vendor/README.md. Local transport tests initially needed execution
outside the restricted sandbox to bind localhost. These were test environment /
test synchronization issues, not successful real-console validation.

## Hardware acceptance still required

Follow README.md's console procedure. Verify account sign-in, console discovery,
video/audio/input, nominated route classification, ten-minute playback, focus
loss, controller reconnection, disconnect/reconnect, application exit and signout.
Frame ARM64 execution, decoder access, passthrough, custom XR, battery life,
input-to-display latency and CPU/memory instrumentation remain unverified or
unimplemented. No benchmark or real Xbox streaming success is claimed.

## Freeze investigation — XRBox 0.1.1

A user-run console session produced sustained video traffic while the upstream
submitted-frame rate fell to zero. Controller detection eventually became true.
These logs establish partial real-session activity, not stable playback or working
controls. No account credentials were read during the investigation.

Reproduced an upstream jitter-buffer defect using a burst of 32 incomplete frame
timestamps followed by a new keyframe after the expiration interval. Without the
fix, the full queue rejects the keyframe before draining expired entries and
never advances. The regression test failed (zero emitted frames) without the
fix and passed with it, including delivery of the subsequent inter-frame.
This is consistent with the reported freeze but is not a confirmed hardware
root cause until the user retests.

The full workspace passed 517 tests (zero failures, two ignored); after the final
capacity-guard refinement, 189 protocol/application tests passed. Strict Clippy
and package/signature checks passed. Version 0.1.1 adds per-stage frame counters,
decoder-error/queue-drop/keyframe diagnostics, rate-limited stall recovery, and a
15-second decoded-video stall stop. The user must quit the old app and reopen
dist/XRBox.app; the running console session was not terminated automatically.

## 0.1.2 — secure restoration and connection UI

The user reported desktop playback working after 0.1.1. This is user hardware
feedback, not a new automated streaming or performance measurement.

Executed for 0.1.2:

- `make check`: formatting and application/all-target Clippy passed with warnings denied.
- `cargo test --locked -p xrbox`: 5 tests passed. New tests use fake HTTP and
  in-memory credentials to verify empty-store restoration makes no requests and
  revoked saved credentials attempt refresh and produce sanitized errors.
- `make package`: built and ad-hoc signed `dist/XRBox.app`, version 0.1.2.
- Packaged app `--smoke`: rendered 10 native GPU UI frames and exited successfully.
  It exercises a synthetic device-code screen (`0O1I-TEST`) and the welcome screen;
  no real credentials, clipboard writes, Microsoft requests or Xbox session occur.

Manual follow-up: sign in with Remember sign-in securely checked, use Copy code
and verify its paste matches the displayed code, complete authentication, and Play.
Close and reopen the same app bundle: consoles should load without another code.
Sign out, reopen and confirm sign-in is required. Test session-only sign-in with
Remember unchecked. Keychain access may prompt; storage failures should be visible.
Actual Keychain restoration, clipboard interaction and the redesigned layout's
usability await this user test. Steam Frame/battery performance remains untested.

## 0.1.3 — enable native clipboard backend

Root cause of the reported Copy code failure: eframe default features were disabled
without explicitly opting into `clipboard`. egui-winit therefore kept copied text
only in its internal fallback buffer. Enabled the native backend and updated the
lockfile (including Linux Wayland clipboard dependencies).

Executed: `make check`, `cargo test --locked -p xrbox` (5 passed), `make package`,
and packaged-app `--smoke` (10 rendered UI frames, successful exit). Inspected
`cargo tree --locked -p xrbox -e features -i egui-winit`: clipboard and arboard are
enabled through eframe. No real credentials or existing clipboard contents were
read, and smoke testing does not write to the clipboard.

Manual verification: reopen version 0.1.3, begin sign-in, click Copy code and paste
into Microsoft's device-code field. End-to-end clipboard interaction still needs
this manual check; the smoke test does not establish successful OS clipboard writes.

## 0.1.4 — reduce Keychain operations

Startup previously persisted access-token, refresh-token and home-stream-token
updates to the same secure bundle. The new app-owned store keeps temporary tokens
in memory and persists refresh-token updates only. Sign-out avoids clearing the
current persistent store twice. Existing bundle contents remain readable.

Executed: `make check`, `cargo test --locked -p xrbox` (6 passed), Python packager
syntax compilation, `make package`, and packaged-app `--smoke` (10 UI frames).
New in-memory test covers temporary-token isolation, refresh restoration/rotation,
and durable sign-out. The optional existing-identity signing path was not tested;
the bundle was built with the default ad-hoc signature. No saved credentials were
read and no actual Keychain prompt count was measured.

Manual retest: open 0.1.4 and approve the XRBox credential request using Always
Allow if desired. Discover/Play, close the app, then reopen the same bundle without
rebuilding. Record whether prompts recur and their exact action/item labels (do
not share credentials). OS unlock requirements and permissions can still prompt;
a rebuilt ad-hoc app can require approval again. Stable-identity signing and real
refresh-token rotation remain hardware/account-dependent checks.

## 0.1.5 — controller-first UI and local stream menu

Implemented Xbox-inspired tile styling, spatial focus, controller action edges,
D-pad/stick repeat, sign-out confirmation, and View+Menu stream-menu access. Menu
input is local. On resume, only View/Menu wait for release; other controls pass
immediately, per the user's explicit preference. A first hotkey button may reach
the console before both buttons are down. Browser authentication remains external.

Executed:

- `cargo test --locked -p xrbox -p xcloud-input`: 12 application and 38 upstream
  input tests passed. New tests cover stick thresholds, repeat timing, action
  edges, focus/reconnect gating, directional tile selection, activation within a
  real headless egui layout, menu-chord focus/edges, and View/Menu-only release.
- `make check`: formatting and Clippy passed with warnings denied.
- `make package`: built and verified the ad-hoc signed macOS 0.1.5 bundle.
- Packaged `--smoke`: 10 GPU UI frames, successful exit. Synthetic screens cover
  device code, welcome, console tiles, sign-out confirmation and Diagnostics.
  Smoke uses a stub sampler and does not read credentials or connect to Xbox.

Manual retest with 0.1.5:

1. Pair the Xbox controller, open XRBox, use D-pad/left stick and A to select a
   console tile. Check the green focus outline, held-direction repeat, controller
   reconnect, and focus loss/return. Use B to cancel provisioning if needed.
2. During gameplay, press View+Menu. Check that the stream menu opens and gameplay
   receives neutral controls while you navigate. Check Diagnostics and close with B.
3. Hold a stick or trigger while resuming with A/B. It should immediately reach
   the console; only View/Menu should remain masked until both are released.
4. Open the menu again and Disconnect. Test sign-out confirmation and B to dismiss.
5. If signing in again, test selecting Copy and Open Microsoft sign-in using the
   controller; browser authentication itself requires the browser's own input.

Physical controller routing, console behavior, final visual usability and battery
impact still require this user hardware test. No latency/battery results claimed.

## 0.1.6 — virtual Xbox Guide and overlapping shortcuts

Implemented the user's clarified mapping: Select/View+Start/Menu sends remote
Xbox/Guide; adding LB+RB opens XRBox instead. Guide commits on chord release as a
120 ms protocol-button pulse. Physical Guide input is not forwarded and no host
system gesture capture, input synthesis or OS settings changes are made. This
supersedes 0.1.5's two-button app-menu shortcut.

Executed:

- `make test`: application (19 tests), API/auth, session and controller suites
  passed. The OS-Keychain integration test remains ignored by default.
- `make check`: formatting and Clippy passed with warnings denied.
- `make package`: built and ad-hoc signed 0.1.6 with pinned dependencies.
- Packaged `--smoke`: rendered 10 synthetic GPU UI frames and exited successfully,
  without credentials, controller sampling, host shortcut changes or Xbox access.

New regression coverage: all 24 press orders for the four-button chord, slow
bumper additions, no smaller-chord action when opening XRBox, held/repeated
shortcuts, exact Guide pulse expiration, partial release, focus loss, disconnect,
idle/menu cancellation, physical Guide filtering, immediate normal inputs and
View/Menu-only release gating. A shared-controller test feeds the real upstream
GamepadInputSource and checks encoded input packets for Guide down and up. Added
headless egui tests for keyboard Enter/Escape and disabled modal-background actions.
Existing auth persistence, redaction, session cleanup and controller tests still pass.

Manual console retest:

1. Launch the packaged 0.1.6, connect and press Select+Start together, then release.
   The Xbox Guide should open once; no host Guide button is pressed by this action.
2. Hold Select+Start for several seconds: no Guide until release, then a short
   press (not remote long-press behavior). Try releasing either button first.
3. Hold Select+Start and add LB/RB, also trying bumpers first. XRBox's menu should
   open without opening the console's Guide. Release the shortcut and resume.
4. While resuming, hold another button, stick or trigger. It should pass through
   immediately; only Select/Start wait for release. LB/RB are not gated on resume.
5. Test focus loss or controller disconnect during both a pending shortcut and
   Guide pulse; returning must not replay it. Check that single Select/Start and
   bumper presses still act normally in games.

Real console response to the Guide pulse and platform-specific host interception
still require hardware validation. Pressing the physical Xbox button may still
open the host overlay; ignoring it in XRBox cannot prevent that OS behavior.

## 0.1.7 — defer single Select/Start through the shortcut recognizer

The reported leakage in 0.1.6 was real: it forwarded the first Select/Start press
before recognizing a combination. Singles are now entries in the same ordered
binding table as Guide and XRBox menu. Raw Select/Start are always masked. The
first Select/Start pressed owns the gesture; its release commits the most specific
completed binding observed while held. Releasing other chord members first does
not dispatch or downgrade it. The app-menu shortcut now also fires on that release.
Protocol actions are bounded 120 ms presses, including singles (no long-press
semantics). Repeated taps use a bounded queue and an 8 ms release gap. Bumpers and
all unrelated controls remain immediate; only Select/Start are deferred.

Executed:

- `make test`: 213 passed (108 API/auth, 38 input, 44 session, 23 app); one actual
  OS-Keychain integration test intentionally ignored. No real Xbox or credentials.
- `make check`: formatting and Clippy passed with warnings denied.
- `make package`: built and ad-hoc signed version 0.1.7.
- Packaged `--smoke`: 10 synthetic GPU UI frames, successful exit.

Tests cover deferred singles, no output while held, first-button release for both
Guide press/release orders, all 24 x 24 press/release permutations of the app
shortcut, priority retention after partial release, distinct rapid taps, queue
bounds, queued-action cancellation, focus/disconnect/idle/menu reset, ordinary
controls passing immediately and physical Guide filtering. Shared-controller tests
inspect actual upstream-encoded packets for singles, Guide and XRBox menu. Bumper
passthrough is explicitly asserted; Select/Start must be absent from combined
shortcut packets. These checks would reject 0.1.6's raw single-button behavior.

Manual retest with 0.1.7:

1. Connect to the Xbox. Hold Select alone: it must not act until released, then
   act once. Repeat for Start, including long holds and quick repeated taps.
2. Press Select then Start, release Start first: no Guide until Select releases.
   Reverse the press order and verify that Start's release owns that gesture.
   Neither case should send individual Select or Start presses.
3. Add LB/RB while holding the pair, release the initiating Select/Start: XRBox's
   menu should open once without console Guide or individual Select/Start actions.
   Try releasing bumpers/secondary button first; it must still choose XRBox.
4. Resume while holding other controls: those pass immediately; Select/Start
   remain suppressed until released. Focus loss/disconnect must cancel pending
   actions without replay on return.

Console response, physical timing and final UX require this user hardware test.
Single Select/Start long-press behavior intentionally changes to a short press on
release, and bumper input is not buffered during shortcut construction.

## 0.1.8 — stop repeated focus scrolling and pin version footer

Reproduced the old navigation behavior in a headless egui test: stationary hover
caused unchanged selected tiles to recenter the scroll area repeatedly, overriding
the requested user scroll position (200 points reset to zero). The regression
passes with one reveal per actual focus change. A companion test verifies that
newly selected offscreen tiles still scroll into view and that assigning the same
focus repeatedly does not fight scrolling. The version label was moved into the
fixed bottom panel so it is no longer clipped by page scrolling.

Executed: `cargo test --locked -p xrbox` (25 passed) and `make check` (formatting
and Clippy with warnings denied). Packaging was already started before the user
asked to skip rebuilding; no further build or app launch was requested. No
packaged smoke test was run for this change, and no credentials or console session
were accessed. The headless tests establish the scrolling fix, not direct
observation of the user's GPU flicker.

Manual retest: reopen the 0.1.8 bundle and hover the sign-in tile and a discovered
console tile before connecting. Confirm the bottom version text stays visible and
steady. Resize the window, scroll the page and navigate between offscreen tiles
with the controller; only changed focus should request a reveal. Confirm the app
shows 0.1.8 so an older running app is not mistaken for the updated bundle.

## 0.1.9 — fullscreen, settings and configurable shortcuts

Implemented native fullscreen with informational panels/footer/cursor hidden and a
zero-margin video area during gameplay. F11 toggles, Escape leaves fullscreen (or
opens the stream menu when windowed), right-click opens the menu, and the configured
controller menu shortcut stays available. Fit/stretch/zoom use GPU draw geometry.

Added controller-navigable settings categories for display, requested resolution,
audio mute, controller deadzone/Y inversion/A-B swap and hotkey capture/reset. The
settings editor saves non-secret preferences atomically on a background thread,
then applies them. Resolution changes wait for the next connection. Recording and
menu use pause gameplay; changing hotkeys cancels old pending actions. Custom
bindings must include Select/Start, reject physical Guide and duplicates, and use
specificity/menu priority while preserving deferred singles.

Executed:

- `cargo test --locked -p xrbox`: final 42 app tests passed.
- `make test`: broader API/auth (108 passed, 1 Keychain test ignored), input
  (38 passed) and session (44 passed) suites also passed, alongside the then-current
  40 app tests. Two additional settings failure-path tests passed in the final run.
- `make check`: final formatting and Clippy passed with warnings denied.
- No app bundle rebuild, GPU smoke launch, credential read or Xbox session, honoring
  the user's earlier request to skip rebuilding/launching the app. The existing
  dist bundle is not this source version.

Coverage includes native Fullscreen command output, full-viewport zero-margin
layout, async fullscreen state transitions, fit/stretch/zoom geometry, controller
activation in every settings category, recorder arming/first-release/cancellation,
custom binding specificity/ties/reset, atomic save/load/replacement/defaults/invalid
files and write failures, save completion/error behavior, upstream input settings,
mute/unmute through a fake audio backend, and applying settings without clearing
existing authentication busy/code state. Previous 576 chord-order regressions and
hover/focus scrolling tests remain passing.

Manual test after the user chooses to build (for example, `make run`):

1. Open Settings using the controller. Navigate all categories using D-pad/stick,
   A to select/cycle and B to return. Change/apply preferences, then relaunch to
   confirm persistence. Invalid custom bindings should show a useful error.
2. Connect to Xbox, open the stream menu and select Fullscreen. Confirm the native
   window fills the display and no informational border, footer or pointer covers
   video. Fit may retain aspect-ratio letterboxing. Verify F11/Escape, right-click,
   and the controller menu chord can recover controls and leave fullscreen.
3. Toggle fullscreen while in Settings and test fullscreen-on-connect on a new
   connection. Menus must remain controller navigable and must pause game input.
4. Verify Fit/Stretch/Zoom, live mute/unmute, deadzone, inversion and gameplay A/B
   swap. Menu A/B must stay unchanged. Request 720p then reconnect; verify actual
   dimensions in Diagnostics rather than assuming the request was honored.
5. Record distinct new Guide and XRBox shortcuts, Apply and test both in gameplay
   and fullscreen. Select/Start must stay deferred; other controls remain immediate.
   Test focus loss/disconnect during recording and restore defaults with Apply.
6. Check remember-sign-in Off skips automatic credential restoration on the next
   launch; it must not delete Keychain credentials silently. Sign out still removes
   them. Test saving failures if practical without modifying credential storage.

Actual macOS fullscreen/Space behavior, physical controller capture, live audio,
console-requested quality and user-visible layout remain manual hardware checks.
No new Frame, battery, zero-copy or performance measurements are claimed.

## 0.1.10 — compact non-overlapping header actions

Replaced oversized header tiles and the 160-point container constraint with fixed
76x44 actions using native vector glyphs and 11-point labels. Settings uses a gear,
Diagnostics uses bars, Sign out uses an exit arrow, and the stream menu uses lines.
Responsive layout stacks/wraps on narrow windows. Controller focus, activation,
modal disabling and accessibility labels are preserved through shared navigation.

Executed `cargo test --locked -p xrbox` (44 passed) and `make check` (formatting and
Clippy passed with warnings denied). Headless layout tests use the app's heading
size/spacing and assert all three action rectangles stay inside 240, 320, 480,
520, 800 and 1280-point windows without overlap. Tests also verify one controller
activation per press and that Diagnostics prevents background sign-out activation.

The full app-test run exposed a pre-existing test temp-path collision: clock-only
names were reused across parallel settings tests. Added a process-local atomic
counter to those test paths; runtime settings persistence is unchanged. The final
parallel test run passes. No app bundle rebuild or launch, credentials, or console
session were used. Manual follow-up after the user rebuilds: hover and navigate
all header actions at normal/narrow window sizes and confirm icon/label appearance.

## Godot foundation — November 11, 2026, source 0.1.10

Extracted auth/settings/controller/session/media/diagnostics into xrbox-core. The
unchanged desktop flow consumes its desktop-session feature; Notify replaces egui
contexts in workers. Added a native godot-rust extension and synthetic cinema scene.
No saved credentials, live authentication, physical audio test or Xbox was used.
The existing desktop .app was not rebuilt/replaced/launched.

Executed on macOS Apple Silicon:

- Rust regression command: cargo test --locked -q -p xrbox -p xrbox-core
  -p xcloud-api -p xcloud-stream -p xcloud-input -- --test-threads=1.
  **235 passed, zero failed, one ignored** (Keychain integration). Counts are
  18 desktop, 27 core, 108 API, 38 input and 44 stream tests.
- Strict Clippy for xrbox, xrbox-core and xrbox-godot, all targets: passed.
  Workspace formatting check: passed.
- Project-local Godot 4.7.2 stable archive: SHA512 checked against official release
  sums. Version reports 4.7.2.stable.official.ed1daf0bf. godot-rust 0.5.5/API 4.7
  builds and loads with default safeguards. No PATH/default-toolchain changes.
- make xr-check: passed native build, engine import and headless scene/input tests.
  Tests exercise the real extension class, native version/target calls, scene
  recreation, geometry bounds/aspect/recenter, menu/focus and hidden redraw state.
  Actual InputEventJoypadButton A/B/D-pad and keyboard events verify engine action
  mappings. Added explicit application-local A/B events after these tests exposed
  that Godot's current defaults omit them; final tests pass.
- Optional tests/render.gd GPU smoke: passed with audio Dummy and XR disabled;
  captured/inspected the synthetic scene. This Mac run used Metal / Forward+ on
  Apple M1 Max, not Linux Mobile/Vulkan. No headset output is inferred.
- cargo check --locked -p xrbox-godot --target aarch64-unknown-linux-gnu: passed.
- cargo check --locked -p xrbox-godot --target aarch64-linux-android: passed.
  Android normal dependency graph contains no xcloud media/session libraries,
  FFmpeg, cpal, gilrs, Keyring, egui or wgpu. Desktop services remain feature-gated.

Sandbox-only failures were kept separate: six API tests could not bind localhost;
the full suite passed with localhost allowed. Initial Godot import could not write
editor/cache files or access system CA facilities in the sandbox; the final
self-contained local engine run passed with platform access allowed. No assertions
were weakened. A godot-rust derive-generated Clippy warning was resolved with an
explicit class constructor rather than disabling the lint.

Only Rust target standard-library caches were added for cross-target checks.
There is no Linux ARM64 cross-linker/sysroot here. Docker was unavailable and was
not started. Cross-target checks do not link a library, export an application,
verify native ABI loading or prove device compatibility. No Linux executable/PCK,
Flatpak, APK or headset-native decoder/GPU path was produced or tested.

Next manual gate: follow xr/README.md on a Linux ARM64 development environment and
Steam Frame. Confirm native extension load, stereo/head tracking, world-locked
placement, screen controls, controller menu/focus and clean exit. Linux is first;
Android APK/device bring-up remains required later. Native synthetic hardware
H.264 decode/import must pass before connecting real Xbox media to Godot.

## Quest 3S development APK — 0.1.10-xr.1

Built the shared Godot synthetic prototype as an ARM64 Android debug APK on macOS
Apple Silicon. Native extension links with Android NDK 29.0.14206865/API 29; Godot
4.7.2 uses OpenXR Vendors 5.1.0-stable with the Meta exporter enabled for quest3 and
quest3s. The engine's actual android_source.zip specifies SDK/target 36, Build-Tools
36.1.0, Gradle 8.11.1, AGP 8.6.1 and Java 17. These supersede older stable-doc
examples for this build. Linux keeps Mobile; Android selects Compatibility.

Executed:

- `make xr-apk`: passed 4 Python packaging regression tests, host/Android native
  builds, Godot export, local debug signing and artifact verification.
- `make xr-apk-check`: verifies the existing artifact without building/deploying.
- Godot headless scene/input/lifecycle tests: passed with the staged upstream addon.
  Added tracked-startup placement and menu bounds tests. Initial recenter waits
  for tracking, then remains world-locked as later head poses change.
- APK signature v2: verified, one signer. ZIP 16 KiB alignment: verified.
- All five packaged shared libraries are AArch64 ELF64 with 16 KiB PT_LOAD alignment.
  There is one libopenxr_loader.so, plus Godot, vendor addon, Rust bridge and libc++.
  Native DT_NEEDED entries resolve to packaged libc++ or Android system libraries;
  no developer desktop-library dependency is present. This does not prove runtime
  loading on Quest.
- Manifest: org.xrbox.prototype, version code 11001, version 0.1.10-xr.1,
  min SDK 29 / target SDK 36, quest3|quest3s metadata, immersive OpenXR/Meta VR
  launch categories. No Internet, microphone or camera permission is requested.
- Packaging checks reject malformed/wrong-ABI/unaligned ELF, missing libraries,
  signing keys/tests/desktop binaries, and tampered/escaping vendor archives.
- Godot/vendor/MPL/project notices are included under assets/legal; public release
  license/source-bundle review is still outstanding.

Artifact: dist/xr/android/XRBox-0.1.10-xr.1-quest-debug.apk, **93,519,098 bytes**.
SHA256: **4b0b61378f13c88b8ad3185dbf62abfc1e71d9986d0385c53e53cf7a69be7b48**.
The sidecar records the checksum of the actual local artifact; rebuilding can
change ZIP/signing bytes. APKs, downloaded tools/addons, signing keys, SDK paths
and generated Gradle output remain ignored. No global PATH, Java/Rust default,
SDK install location or global Godot editor preference was changed. Godot's device
polling is disabled for this preset; no headset was installed or launched.

Manual boundary: [xr/ANDROID.md](xr/ANDROID.md). User can test on Meta Quest 3S
while Steam Frame is shipping. Pair a Bluetooth Xbox controller; Touch-controller
UI is not implemented. Check headset/native initialization, stereo/head tracking,
initial placement/recenter, controller navigation, focus/suspend/return and quit.
No real headset, Xbox account/console, audio, MediaCodec, passthrough, battery or
stream latency result is claimed. This APK is a synthetic frontend prototype.
