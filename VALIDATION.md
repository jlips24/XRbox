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
