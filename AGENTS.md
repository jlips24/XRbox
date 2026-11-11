# XRBox agent instructions

## Product and current scope

XRBox is an Xbox **console Remote Play** client with a native Rust core. Headset
product targets are Steam Frame/Linux ARM64 and Android ARM64 APKs (Meta Quest
first, then other validated Android OpenXR headsets). It must ship as one app
without a relay PC, an embedded browser runtime, or separately configured services. Electron and
cloud gaming are excluded.

The current milestone is reliable desktop streaming on macOS Apple Silicon.
The user has reported desktop playback working. Recent authentication, controller
shortcut and UI changes still need the manual checks recorded in VALIDATION.md.
Standalone Steam Frame, Linux ARM64 hardware decoding, XR presentation and battery
performance remain unvalidated; full XR work is a separate milestone.

Read README.md, ARCHITECTURE.md, VALIDATION.md and the relevant parts of Plan.md
before architectural changes. Godot is the selected future XR frontend, with the
Rust streaming core embedded through an in-process GDExtension. Do not revert to a
handwritten native OpenXR product frontend by default. The Godot project/extension,
Android platform adapters and headset exports are not implemented yet; follow the
prototype and hardware gates in Plan.md before a full UI migration. Prefer
implemented behavior and current validation evidence over older planning assumptions. Read the application version from
crates/xrbox/Cargo.toml; do not infer it from old examples or an existing app bundle.

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

- `crates/xrbox/src/main.rs`: native egui/eframe UI and GPU presentation.
- `auth.rs`: credential-store selection and console-only token acquisition.
- `session.rs`: bounded commands, session ownership, cancellation, input and recovery.
- `controller.rs` and `navigation.rs`: shared physical sampling, menu/game input
  routing, spatial focus and controller repeat. After stream-menu resume, only
  View/Menu wait for release; other gameplay input must resume immediately.
  Select/View and Start/Menu are deferred bindings: never forward their raw
  button-down events. Resolve the most specific completed binding when the first
  Select/Start pressed is released. Default shortcuts are the pair for Guide and
  the pair+LB+RB for XRBox; preserve validated user-configured mappings across
  cancellation. Singles use the same deferred recognizer and protocol pulse path. Physical Guide
  stays reserved to the host and is not forwarded. Preserve bounded input queues
  and cancellation on focus loss/disconnect; do not enable system-gesture capture.
  Reveal a selected tile only on focus change; idle hover must not recenter the
  page. Keep the version footer outside scrollable content.
- `media.rs`: upstream RTP/decoder/audio wiring and stage counters.
- `config.rs`: validated non-secret settings and atomic persistence; credentials
  never belong in settings.json. Apply live input/audio/hotkeys without cancelling
  auth jobs; resolution changes apply on the next connection.
- `settings_ui.rs`: controller-navigable draft editor, chord capture and background
  saves. Apply only after a successful save; preserve cancellation/error handling.
- `toolbar.rs`: compact vector-glyph header actions. Reserve fixed hit rectangles,
  keep controller focus/accessibility labels, and wrap rather than overlap at
  narrow widths. Do not reuse large content-tile sizing for header buttons.
- `presentation.rs`: native fullscreen state and video fit geometry. Fullscreen
  gameplay hides all informational panels/margins, but must retain menu/exit paths.
- `diagnostics.rs`: sanitized logging.
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

`make smoke` opens the UI briefly without credentials. For isolated non-secret
logs, prefix that command with an explicit temporary XRBOX_DATA_DIR.
`make auth-probe` contacts Microsoft and discards the code; it does not authorize an
account. When packaging is requested or needed, use `make package` for the ad-hoc
signed macOS development bundle and `make package-release` for an optimized bundle.
Neither installs the app. The packager can use an existing signing identity via
`--sign-identity`; do not create certificates or modify Keychain trust automatically.
Ad-hoc rebuilds may require renewed credential-access approval.

## Dependencies and repository changes

Dependabot configuration lives in .github/dependabot.yml: Cargo updates are checked
daily at 09:00 America/New_York, with related graphics/WebRTC groups and at most
five open version-update PRs. Keep schedule documentation consistent with the YAML.
Do not auto-merge updates. Review API compatibility, enabled features (including
native clipboard support), hardware decode/import paths, native runtime libraries
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
