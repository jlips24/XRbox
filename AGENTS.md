# XRbox agent instructions

## Product and current scope

XRbox is an Xbox **console Remote Play** client with a native Rust core. Headset
product targets are Steam Frame/Linux ARM64 and Android ARM64 APKs (Meta Quest
first, then other validated Android OpenXR headsets). It must ship as one app
without a relay PC, an embedded browser runtime, or separately configured services. Electron and
cloud gaming are excluded.

The standalone 2D desktop frontend has been retired by user decision. Do not
reintroduce it by default. macOS remains a development host for Godot previews,
headless tests and Android builds, not a product target. Existing upstream Mac
backends are retained as library code, not a shipping desktop application.

Read README.md, ARCHITECTURE.md, VALIDATION.md and relevant Plan.md sections before
architectural changes. Keep one repository with separate components:
`crates/xrbox-core` is engine-independent; `crates/xrbox-godot` embeds it through
an in-process GDExtension; `xr/` owns Godot scene/UI and Android platform glue.
The core must never depend on Godot. Shared session services use the `session`
feature. Host previews remain synthetic and do not start auth or streaming.
Android build 0.1.10-xr.2 implements live streaming, but actual sign-in, Keystore,
video/audio/controller and XR behavior still need hardware acceptance.
Linux ARM64 decoding/import, Flatpak and battery performance remain unvalidated.
Follow XR_PLAN.md and xr/ANDROID.md; do not infer hardware results from builds.
Read the core version from crates/xrbox-core/Cargo.toml and APK version from the
Godot configuration; old desktop documentation is historical.

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
  with automatic restoration and a session-only option. Persist refresh credentials
  in the OS credential store; keep temporary access, stream and transfer tokens in
  memory. Preserve refresh rotation, sign-out cleanup and visible storage errors.
  Never introduce plaintext token files or log tokens, device codes, raw auth
  responses, SDP, ICE addresses or account IDs.

## Code boundaries

- `xr/`: Godot scene/input prototype and synthetic engine tests.
- `crates/xrbox-godot`: in-process extension; no engine calls from session/decoder workers. Android video uses a documented
  render-thread callback, external OES texture and fenced AImage leases. Keep Android
  free of FFmpeg/gilrs/desktop credential dependencies. `android_video.rs` owns EGL
  import; `input.rs` maps Godot samples with sticky disconnect neutralization.
- `crates/xrbox-core/src/android.rs` and `xr/android-src/`: JNI bootstrap, Android
  Keystore sealed refresh persistence and hardware codec selection. Never read
  live stored credentials during development checks.
- `vendor/openxcloud/crates/xcloud-media/src/android.rs`: bounded MediaCodec /
  PRIVATE ImageReader hardware buffers; opaque output size is not a CPU pixel count.
- `crates/xrbox-core/src/auth.rs`: credential-store selection and console-only token acquisition.
- `crates/xrbox-core/src/session.rs`: bounded commands, session ownership, cancellation, input and recovery.
- `crates/xrbox-core/src/controller.rs` and `xr/scripts/`: shared physical sampling, menu/game input
  routing, spatial focus and controller repeat. After stream-menu resume, only
  View/Menu wait for release; other gameplay input must resume immediately.
  Select/View and Start/Menu are deferred bindings: never forward their raw
  button-down events. Resolve the most specific completed binding when the first
  Select/Start pressed is released. Default shortcuts are the pair for Guide and
  the pair+LB+RB for XRbox; preserve validated user-configured mappings across
  cancellation. Singles use the same deferred recognizer and protocol pulse path. Physical Guide
  stays reserved to the host and is not forwarded. Preserve bounded input queues
  and cancellation on focus loss/disconnect; do not enable system-gesture capture.
  Reveal a selected tile only on focus change; idle hover must not recenter the
  page. Keep the version footer outside scrollable content.
- `crates/xrbox-core/src/media.rs`: upstream RTP/decoder/audio wiring and stage counters.
- `crates/xrbox-core/src/config.rs`: validated non-secret settings and atomic persistence; credentials
  never belong in settings.json. Apply live input/audio/hotkeys without cancelling
  auth jobs; resolution changes apply on the next connection.
- `xr/scripts/live_ui.gd`: controller-navigable sign-in, console selection and stream
  menu. Full desktop settings/hotkey editing is not yet ported; do not claim parity.
- `crates/xrbox-core/src/diagnostics.rs`: sanitized logging.
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
For runtime changes, use the relevant tests and checks:

```sh
make check
make test
```

Build with `make build` when needed for the task. Documentation/configuration-only
changes do not need an app rebuild or launch; validate the changed files instead.
Honor a request to skip rebuilds, packaging or app launches. Do not run live auth,
credential or console tests merely to validate unrelated changes.

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

`make xr-preview` opens only synthetic content on a development host. The retired
standalone app's smoke/auth-probe/macOS package tasks no longer exist. Use
`make xr-apk` only when Android packaging is needed; it never operates a device.

## Dependencies and repository changes

Dependabot configuration lives in .github/dependabot.yml: Cargo updates are checked
daily at 09:00 America/New_York, with related graphics/WebRTC groups and at most
five open version-update PRs. Keep schedule documentation consistent with the YAML.
Do not auto-merge updates. Review API compatibility, enabled features (including
Godot/JNI compatibility), hardware decode/import paths, native runtime libraries
and license compatibility; 0.x minor upgrades may be breaking changes.

The pinned Rust toolchain and vendored OpenXCloud source revision require manual
updates. Dependabot does not update that source snapshot. There are currently no
GitHub Actions checks; do not claim that a dependency PR has passed CI just because
it was opened by Dependabot. Add action dependency coverage if workflows are added.

Keep commits grouped by purpose and avoid unrelated user changes. Do not commit
build output, generated video fixtures, local logs or credentials; preserve the
.gitignore rules. Do not add a remote, push, publish or create releases unless the
user asks. Documentation and config changes should receive appropriate lightweight
validation rather than unrelated runtime tests.

## Reporting and hardware boundary

Run checks appropriate to each change and report what actually executed. Add
meaningful regression coverage for behavioral changes where practical, especially
shortcut priority, focus/disconnect cleanup and input suppression. Use headless UI
and fake-service tests where possible; document remaining hardware-only checks. Update
documentation when behavior, commands or limitations change. For streaming fixes,
separate packet receipt, frame assembly, decoding and presentation evidence; never
equate network traffic or signaling state with successful video playback.

Only report real Xbox streaming, physical controls/audio, Frame compatibility,
direct LAN routing, zero-copy or battery improvements when supported by relevant
hardware observations. Private ICE addresses alone do not prove LAN topology.
After preparing a hardware-dependent fix, provide an identifiable build and exact
retest steps, then stop for the user's results. Do not fabricate benchmarks.

## Godot checks

`make xr-check` builds/stages the host extension and runs headless Godot tests.
`make xr-preview` runs the synthetic scene without XR; `make xr-run` attempts XR.
Use GODOT as a command-local executable path; an authorized project-local Mac
download may live under ignored .tools/. Never install tools automatically.
Godot 4.7.2 and godot-rust 0.5.5/API 4.7 are pinned. A cross-target cargo check
is not a linked library, exported app or device test. Host Godot previews never start auth/sessions; Android live mode does. Headless
UI tests use fake state and suppress preference restoration. Test the shared core through `make test` / `make check` when touching shared logic.

`make xr-apk` builds/signs/verifies the Quest 3S prototype using a project-local
SDK/NDK and self-contained Godot. `make xr-apk-check` verifies an existing artifact.
The helper must never edit global Godot settings, install SDK tools automatically,
poll/install/launch on headsets or use a production signing key. The preset is not
runnable in the editor to disable device polling. Preserve local debug keys in
ignored .tools; never commit them or APKs. Read xr/ANDROID.md for tool pins and the
manual device gate. Android has its own Compatibility renderer override; do not
change Linux rendering defaults to accommodate the Android test.
