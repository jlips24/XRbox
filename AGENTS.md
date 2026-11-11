# XRBox agent instructions

## Product and current scope

XRBox is a native Rust Xbox **console Remote Play** client, with standalone Steam
Frame/Linux ARM64 as the eventual target. It must ship as one app without a relay
PC, an embedded browser runtime, or separately configured services. Electron and
cloud gaming are excluded.

The current milestone is reliable desktop streaming on macOS Apple Silicon.
The user reports desktop playback working after the 0.1.1 video fix.
New secure sign-in restoration in 0.1.2 still needs user validation. Full XR work
is a separate milestone. Read README.md, ARCHITECTURE.md, VALIDATION.md and the relevant parts
of Plan.md before making architectural changes. Prefer implemented behavior and
current validation evidence over older planning assumptions.

## Respect the user's machine

- Never change PATH, shell startup files, global/default Rust toolchains, default
  apps, global Git configuration, system defaults, or enabled services.
- Keep environment overrides command-local. Honor an existing CARGO_TARGET_DIR;
  do not hardcode the temporary audit checkout or another developer's paths.
- Do not install system tools or packages automatically. Document missing prerequisites.
  Normal Cargo dependency/toolchain cache downloads for an authorized build are
  acceptable; do not change the default toolchain.
- Do not read saved credentials, authorize an account, operate the user's console,
  terminate their running app, or run ignored keychain/audio tests without an
  explicit request. Synthetic tests and in-memory UI smoke tests are appropriate.
- Keep secure credential persistence enabled by default, as requested by the user,
  with automatic restoration and a session-only option. Never introduce plaintext token files
  or log tokens, device codes, raw auth responses, SDP, ICE addresses or account IDs.

## Code boundaries

- `crates/xrbox/src/main.rs`: native egui/eframe UI and GPU presentation.
- `auth.rs`: credential-store selection and console-only token acquisition.
- `session.rs`: bounded commands, session ownership, cancellation, input and recovery.
- `controller.rs` and `navigation.rs`: shared physical sampling, menu/game input
  routing, spatial focus and controller repeat. After stream-menu resume, only
  View/Menu wait for release; other gameplay input must resume immediately.
- `media.rs`: upstream RTP/decoder/audio wiring and stage counters.
- `config.rs` and `diagnostics.rs`: non-secret app data and sanitized logging.
- `vendor/openxcloud`: seven library crates pinned to the revision in
  vendor/README.md. Do not add the upstream cloud-oriented application crate.

Reuse upstream authentication, SDP/ICE, WebRTC, packet codecs, decoding and input.
Keep vendored changes narrow; document their provenance and rationale in
vendor/README.md and add meaningful regression coverage. Preserve GPL-3.0-only
licensing and third-party notices. No speculative XR scaffolding or protocol rewrite.

Do not block the UI thread on network, decoder or session teardown. Keep media
queues bounded, preserve native GPU buffers through presentation, and disable
optional image-processing passes by default. An available hardware decoder does
not establish that a particular pipeline can use it efficiently on Frame.

## Build and checks

Use the project-pinned toolchain and Cargo.lock. `make help` lists supported tasks.

```sh
make build
make check
make test
```

For changes to vendored protocol/media code, run the affected regression tests
and the applicable broader checks:

```sh
make test-all
make check-all
```

The full targets generate an ignored synthetic video fixture and require developer
FFmpeg with libx264/headers, CMake and pkg-config. Some tests bind localhost or
require GPU access; distinguish sandbox restrictions from application failures.
Do not disable assertions or weaken tests just to obtain passing results.

`make smoke` opens the UI briefly without credentials. For isolated non-secret
logs, prefix that command with an explicit temporary XRBOX_DATA_DIR.
`make auth-probe` contacts Microsoft and discards the code; it does not authorize an
account. Use `make package` for the ad-hoc signed macOS development bundle and
`make package-release` for an optimized bundle. Neither installs the app.

## Reporting and hardware boundary

Run checks appropriate to each change and report what actually executed. Update
documentation when behavior, commands or limitations change. For streaming fixes,
separate packet receipt, frame assembly, decoding and presentation evidence; never
equate network traffic or signaling state with successful video playback.

Only report real Xbox streaming, physical controls/audio, Frame compatibility,
direct LAN routing, zero-copy or battery improvements when supported by relevant
hardware observations. Private ICE addresses alone do not prove LAN topology.
After preparing a hardware-dependent fix, provide an identifiable build and exact
retest steps, then stop for the user's results. Do not fabricate benchmarks.
