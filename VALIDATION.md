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
