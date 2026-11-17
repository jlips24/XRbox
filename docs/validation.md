# Validation evidence

## v0.0.1 release preparation — November 16, 2026

- APK display version is **0.0.1**, version code **11003**; internal core remains
  **0.1.10**. The numeric code preserves upgrade ordering from prototype 11002.
- `make xr-apk` passed: ARM64 build, debug signature, native/ZIP 16 KiB alignment,
  Java/JNI bootstrap, Internet/Quest manifest requirements and sensor/key exclusion.
- Local artifact: `dist/xr/android/XRbox-0.0.1-quest-debug.apk`, **108,516,142 bytes**.
  SHA256: `25a0f3728e46f0f28f6b9faa656c7baf792a254f9ffe0b98a1a482b3d41a020e`.
  This identifies the locally signed APK, not a future CI-signed release asset.
- `make xr-check` passed, including four native bridge tests and the fake-state
  headless UI/Meta-input/material/focus/shutdown scenarios. Thirteen Python helper
  tests passed. Local documentation link targets and whitespace checks passed.
- APK manifest verification now checks the configured version rather than a
  historical hardcoded version. CI artifact metadata rejects a tag/app mismatch.
- Historical snapshots were removed; Git retains earlier records. Guides moved
  under `docs/`, with root/component entrypoints and legal/provenance files kept.
- No account authorization, stored credentials, console or headset was accessed.
  No GitHub tag/release was created or branch pushed; the user will create v0.0.1.

Follow [the release guide](releases.md) and [Quest test procedure](android.md#install-and-test).

## GitHub Actions configuration — November 16, 2026

- Test workflow targets PRs against master and invokes full lint/synthetic suites.
  Build workflow gates new version tags/published releases and produces a Linux
  ARM64 extension plus an Android development APK as Actions artifacts.
- Local validation: actionlint 1.7.12 passed; workflow/composite/Dependabot YAML
  parsed; ten Python helper regressions passed, including exact version-tag
  filtering, output-injection rejection and download-checksum rejection.
- Action SHAs and the Linux Godot editor checksum were checked against their
  public release metadata. `git diff --check` passed. GitHub Actions itself has
  **not run**: no branch was pushed and no PR/tag/release was created.
- Hosted software-Vulkan tests will remain synthetic. Building artifacts cannot
  establish headset playback, Frame compatibility or hardware acceptance.

## Shared UI and Wednesday test preparation — November 16, 2026

- Source xr.3/code 11003 adds styled controller-first menus, a fixed footer,
  diagnostics page and explicit Meta Touch menu action bindings. Gameplay still
  requires a physical Xbox controller; hand tracking is not implemented.
- `make xr-check` passed with fake Meta input/repeat/focus recovery and idle-scroll
  regression coverage. Seven Python helper tests and shell syntax checks passed.
- A synthetic GPU UI render was inspected. No new APK was built and no actual
  Meta/Frame input, headset, account or console behavior was validated.
- Experimental Vulkan probe, Docker build/packaging helpers, downloaded Linux
  source/cache and staged artifacts were discarded. Stock Godot remains unchanged.
- [Wednesday test plan](frame-test-plan.md) and
  [results template](frame-results-template.md) explicitly gate live testing on
  GPU-native local video. Linux live streaming remains unimplemented/disabled.

## Maintainability reorganization — November 16, 2026

- `make check`: formatting and strict core/bridge Clippy passed.
- `make test`: **222 passed, zero failed, one ignored** (credential-store test).
  Includes a new in-flight job cancellation regression; existing shortcut,
  focus/disconnect, restoration and configuration coverage remains enabled.
- `make xr-check`: four native bridge tests and headless Godot scenarios passed,
  including navigation, placement, live pages, fake-video material cleanup and
  repeated shutdown/recreation. No preference restoration or live sessions.
- `make xr-render`: passed; the synthetic preview image was inspected. Godot
  emitted host shader-cache casing warnings; no source asset casing errors were
  reported. This is a host GPU smoke, not headset presentation evidence.
- `make test-python`: seven helper tests passed. APK-specific discovery still runs
  all four packaging/native-inspection tests after their separation.
- Android `cargo clippy --locked --target aarch64-linux-android -p xrbox-core
  -p xrbox-godot --all-targets --features xrbox-core/session -- -D warnings` passed
  with the project-local NDK compiler/linker/CMake environment.
- `make xr-apk`: built, debug-signed and verified the artifact below, including
  five ARM64 libraries with 16 KiB load alignment, one XR loader and required
  manifest/bootstrap entries. Keys and test resources are excluded.
- Local Markdown link targets, Godot resource references, Make dry runs and
  `git diff --check` passed. Exported Godot method signatures and snapshot keys
  match the pre-refactor source. Vendor source and dependency pins are unchanged.

Initial sandbox runs blocked six fake-HTTP localhost tests and Godot's system
certificate initialization. Scoped reruns passed; assertions were not weakened.
No saved XRbox credentials, account authorization, console or headset was accessed.

### Rebuilt artifact

Version **0.1.10-xr.2**, version code **11002** (unchanged):
`dist/xr/android/XRbox-0.1.10-xr.2-quest-debug.apk`, **108,508,208 bytes**.
SHA256: `010da3cafde1b24a1a879dfc385582406883e26a3e50d0e47f2740f11b987bad`.
This checksum distinguishes the reorganization build from the earlier xr.2 APK.
Follow the [reorganization retest](android.md#reorganization-retest) manually.

## Baseline recorded November 12, 2026

These are prior recorded results, not a fresh run of the reorganized source:

- Desktop-retirement checks: `make test` recorded 221 passed, zero failed,
  one ignored; `make check` and `make xr-check` passed.
- Android live implementation: workspace/all-feature tests recorded 564 passed,
  zero failed, two ignored; strict host and Android lint passed.
- APK 0.1.10-xr.2 was built/signature/ABI/alignment checked. Its recorded SHA256 is
  `6e9bd35d001f78224331d313335d68283d39ddc1a2b29e1bf76eb79a767bcd5a`.

Earlier detailed environments, suite scope and results remain in Git history.
Historical desktop observations do not establish headset playback.

## Open acceptance

Quest/console sign-in, Keystore, moving XR video, audio, physical controls and
lifecycle remain unaccepted. Linux ARM64 decoding/import, Frame, Flatpak, direct
LAN routing, zero-copy, latency and battery remain independently unvalidated.
Follow [Android guide](android.md) for manual device checks and
[development guide](development.md) for automated commands.

## CI compile reuse — November 16, 2026

Hosted [run 43821987015](https://github.com/jlips24/XRbox/actions/runs/43821987015)
passed Rust, Python and GPU smoke, but failed the Godot frontend stage during
headless editor import. Rust took 539 seconds, including 520 seconds compiling;
Godot built a separate feature graph for another 97 seconds. The captured import
log did not establish the cause of the abnormal exit. The successful render
screenshot does not establish that the headless UI suite ran.

The CI runner now retains build caches after failures, stages Godot with the full
workspace feature graph, and avoids repeating bridge tests already run by Rust.
It retains all four suites and separate import/UI/render logs. Local macOS
`make test-ci` passed all four suites, including 15 Python helper regressions.
A warm local run recorded Rust 17.7s, Python 0.1s, Godot frontend 3.1s and GPU
smoke 1.2s; Godot staging reused the build in 0.26s. These are macOS timings,
not a GitHub benchmark. Workflow actionlint and diff whitespace checks passed.
Hosted cache timings and the Linux import failure still require a GitHub rerun;
these changes do not establish that the import failure is fixed.

## Cold-import crash investigation — November 16, 2026

Run 43829070998 captured SIGSEGV on the original import process's main thread,
after `EditorSettings: Save OK!`. Engine frames lack function symbols; this does
not establish an exact failing function. The evidence closely matches Godot's
confirmed [GDExtension documentation shutdown race](https://github.com/godotengine/godot/issues/111048),
including earlier clean diagnostic reruns after a failed cold import. The helper
now applies the upstream frame-delay workaround (1000ms) to editor import only.
Crash diagnostics, nonzero exit checks and UI/render assertions remain enabled.

Local `make xr-check` passed the four bridge regressions and headless UI smoke.
A fresh-cache import in a temporary project copy also passed on macOS.
Python helpers passed 17 tests; one Linux-only GDB integration check was skipped
on macOS. Diff whitespace checks passed. Linux CI must validate the timing
workaround; this is not a symbolicated root-cause confirmation or engine fix.

## Automatic release attachments — November 16, 2026

The Build workflow now runs on release publication only. It checks numeric tag/APK
version agreement and master ancestry, pins both builds to the validated commit,
and attaches verified outputs after both jobs succeed. The release guide describes
the complete GitHub UI process. The final job alone has repository write access.

Local Python regressions cover source ancestry, version mismatch, artifact source
and tag mismatch, tampering rejection, archive contents and final checksums.
Workflow actionlint and whitespace checks passed. No release was published, assets
uploaded or devices accessed during implementation; the actual hosted build and
attachment path require the first published release to validate end to end.

## Single release-version config — November 16, 2026

`release.json` now owns public app version and Android version code. Build/preview/
check and APK helpers synchronize derived Godot settings and filenames; CI tag
validation and provenance read the config directly. Rust core version is separate.
Config-only edits through GitHub require no manual edits to derived version fields.

Local Python tests passed 25 tests with one Linux-only GDB skip, including version
bump synchronization, unrelated-setting preservation, idempotence, invalid config
and missing-setting rejection. `make xr-check` passed bridge and fake-state UI
checks. No new APK was built and no device was accessed; the configured version
and code remain 0.0.1/11003.

## Quest feedback and retest build — November 16, 2026

User observation: Quest 3S boots and connects to the console. An 8BitDo Ultimate 2
can navigate XRbox menus but does not control the Xbox. This does not establish
moving video, audio, secure persistence, latency or battery acceptance.

Branch `bug/quest-input-background-recenter` adds active-gamepad selection,
non-secret sampled/routed activity counters and focus/menu/controller diagnostics.
The root cause of the reported gameplay failure remains unconfirmed. A regression
checks that menu suppression ends on resume and that focus loss neutralizes input.
Passthrough is optional with 10% darkness adjustments, and the OpenXR recenter
signal requests placement from the next valid tracked pose. Neither XR behavior
has been accepted on a headset.

- `make check` passed strict core/bridge lint and formatting.
- `make test` passed: 223 tests, one ignored credential test. Initial sandbox
  fake-HTTP socket failures passed on a scoped rerun with localhost access.
- `make xr-check` passed four bridge tests and headless UI regressions, including
  tracked recenter placement, darkness limits and active-pad selection/disconnect.
  The scoped rerun avoided macOS certificate/socket sandbox restrictions.
- `make xr-apk` passed six packaging regressions, ARM64 build, export, debug
  signature and 16 KiB ZIP/native alignment validation. Gradle required a scoped
  rerun outside the socket-restricted sandbox. Diff whitespace checks passed.
- Artifact: `dist/xr/android/XRbox-0.0.4-quest-debug.apk`, version code 11006,
  108,519,698 bytes. SHA256:
  `db09632d86eb7543e24176990ba0a22ee9cbcd89495f7a25cc411982c0e730f9`.

No device, console, saved credentials or live account was accessed by the agent.
Follow [the targeted Quest retest](android.md#quest-controller-background-and-recenter-retest).

### User retest follow-up

After manually installing this APK on Quest 3S, the user reported “Works great”
and requested commit/push. The installed package had a different signing key,
so an in-place update was rejected; installation required removing the old app.
This is positive user retest feedback for the build above, without separate
measurements or itemized acceptance results. The precise cause of the earlier
8BitDo routing failure remains unisolated.
## Linux ARM64 Flatpak packaging — November 16, 2026

Implemented the complete Godot export/SDK-native compilation/Flatpak bundle path,
launcher and notices. Full Flatpak packaging runs only on release publication;
PRs and master pushes retain lightweight packaging regressions in the normal suite. Release uploads now
contain only APK and Flatpak; GitHub supplies both source archives. Checksummed
provenance remains in CI artifacts. Existing releases were not edited.

Local Python regressions passed 32 tests with one Linux-only skip; manifest JSON,
AppStream XML, source staging isolation, package-only release output, Make dry run
and whitespace checks passed. This Mac lacks Flatpak/SDK tooling, so no actual
Flatpak build, installation, Linux launch or hardware acceptance was performed.
The first published-release Linux job must pass before a Flatpak is attached. See [Linux guide](linux.md).

## Spatial UI overhaul — November 16, 2026

Branch `feat/spatial-ui-overhaul`, app 0.0.5/code 11007. Implements a wider rounded
control panel with persistent navigation, separate settings pages, inline numeric
adjustments, three screen presets, bounded height/horizontal placement, a screen
frame and a once-rendered idle card. Existing native media and deferred input paths
are unchanged. Screen/room adjustments remain session-only; direct grabbing, hand
tracking and full settings/custom hotkey editing are outside this change.

`make check` passed. `make test` passed 223 tests with one ignored credential test
on a scoped localhost-enabled run; the initial restricted run could not bind fake
HTTP fixtures. Four bridge unit tests passed during `make xr-check`; its first
Godot phase was blocked by macOS sandbox certificate/socket restrictions. The
scoped Godot check passed after script/layout corrections. Synthetic GPU captures
were inspected for home, screen, placement, environment and account. Additional
captures cover settings, consoles and the idle screen. No live credentials,
console, headset or account were accessed. Final headless regressions passed, including supported-runtime room controls,
preset resizing, placement reset and page navigation. The synthetic GPU run
captured eight pages successfully. `make xr-apk` passed six packaging regressions,
ARM64 compilation, export, debug-signature verification and 16 KiB native/ZIP
alignment verification after rerunning with Gradle socket access.

Artifact: `dist/xr/android/XRbox-0.0.5-quest-debug.apk`, 108,526,266 bytes. SHA256:
`f00cd4de120966ba9f02266c94a204c656bd486a1953a355d74425e6f0656b74`. Follow the [spatial interface retest](android.md#spatial-interface-retest--005--11007).

## Startup UI placement fix — November 16, 2026

User reported that the redesigned UI appears behind them at app launch.
`bug/startup-ui-placement` preserves the uncommitted UI overhaul and adds a
focused-session/valid-head-pose startup gate. Placement uses the tracker's adjusted
transform in the XR origin's world space, and projects forward onto the floor
instead of extracting Euler yaw. Initial surfaces remain hidden until placement;
ordinary head motion preserves the anchor. This addresses an identified startup
readiness gap; the device root cause and fix acceptance remain unconfirmed.

`make xr-check` passed four bridge tests and headless UI regressions. New synthetic
coverage includes missing/stale head poses, a valid pose before session focus,
a backward-facing tilted headset with a deliberately stale scene camera, initial
placement completion, level placement, vertical-gaze deferral and world anchoring.
Implementation follows Godot 4.7's [adjusted-pose contract](https://docs.godotengine.org/en/4.7/classes/class_xrpose.html#class-xrpose-method-get-adjusted-transform)
and the pinned [camera pose integration](https://github.com/godotengine/godot/blob/4.7.2-stable/scene/3d/xr/xr_nodes.cpp).

`make xr-apk` passed packaging regressions, ARM64 build/export, debug signature
and 16 KiB alignment verification. Artifact:
`dist/xr/android/XRbox-0.0.6-quest-debug.apk`, code 11008, 108,527,222 bytes.
SHA256: `51e181e6bdd0c066d6430f973f94de305378a36925a17de2f385d49a950062dc`.
No headset, console, account or stored credentials were accessed. Follow the
[startup placement retest](android.md#startup-placement-retest--006--11008).

## Room darkness and picture quality — November 16, 2026

User reports room darkness affecting settings and a low-resolution/jagged Xbox
picture. Branch `bug/room-darkness-picture-quality` keeps the existing UI/startup
work. The menu now renders opaque pixels with alpha-scissor depth protection and
transparent rounded corners; the room sphere has the lowest render priority.
Video explicitly requests linear/clamped external sampling. Android alone enables
4x MSAA for geometry edges. The existing 1080p console request is unchanged.
Diagnostics report decoded-frame dimensions from the latest submitted frame,
not the placeholder texture; release clears these dimensions.

`make check` passed. `make test` passed 224 tests with one ignored credential test.
`make xr-check` passed five bridge tests and headless UI regressions, including
resolution reporting/reset and material/Android AA policy checks. Synthetic GPU
regressions passed in both host Mobile/Metal and OpenGL Compatibility renderers:
50%/100% darkness changes the synthetic room while preserving menu and idle-screen
pixels. Samples include menu portions beyond the screen silhouette. Tests force
the dimmer to draw last, exposing the original alpha-blended menu's vulnerability;
a scoped mutation removing depth protection was rejected by the regression.
The initial normal-order host simulation did not reproduce the reported device
symptom. The stricter test caught insufficient depth-prepass protection before
switching to opaque alpha-scissor rendering. No claim of device acceptance follows.

`make xr-apk` passed six packaging regressions, host/Android native compilation,
export, debug-signature and 16 KiB ZIP/native alignment verification.
Artifact: `dist/xr/android/XRbox-0.0.7-quest-debug.apk`, code 11009,
108,528,050 bytes. SHA256:
`935c7574f84b636f43a80dfcf4ec33793c2a33457538a4bd60f518dfa354d345`.
No account, stored credentials, console or headset was accessed. Actual decoded
source resolution, visible sharpness, stereo behavior and performance remain
hardware checks. Follow the [targeted retest](android.md#room-darkness-and-picture-retest--007--11009).

Godot documents [opaque alpha-scissor rendering](https://docs.godotengine.org/en/4.7/classes/class_basematerial3d.html)
and [MSAA's geometry-only scope](https://docs.godotengine.org/en/4.7/classes/class_projectsettings.html#class-projectsettings-property-rendering-anti-aliasing-quality-msaa-3d).
No image postprocessing, supersampling or native media/protocol change was added.

## Flatpak release runner prerequisites — November 16, 2026

[Release job 133287581833](https://github.com/jlips24/XRbox/actions/runs/43863911609/job/133287581833)
failed while flatpak-builder processed the staged Rust toolchain, before compiling
XRbox. It reported missing `eu-strip` and warned that `eu-elfcompress` was absent.
Both tools come from elfutils. The `make xr-flatpak | tee` pipeline masked the
builder's exit status, so the visible failing step instead tried to copy a
nonexistent Flatpak. This run does not establish Linux app or hardware behavior.

Branch `bug/flatpak-ci-prerequisites` installs elfutils on the ephemeral CI runner,
checks both tools before staging/downloads on developer hosts, and uses explicit
Bash for the logged build step so GitHub enables pipefail. No host tools were
installed on this Mac and no global settings or release/tag were changed.

Seven Flatpak helper tests and seven release-helper tests passed, including missing
elfutils rejection and execution of the actual workflow build body with a failing
synthetic make command: exit 23 survives tee and the log remains available. Diff
whitespace validation passed. No actual Flatpak build or CI rerun was performed;
this Mac cannot validate Linux ARM64 SDK packaging. Rerunning the existing tagged
release uses its old source; these changes require a build from the corrected
revision under the normal release/version gate.

## CI lint failure propagation — November 16, 2026

The Test workflow's logged `make check-all` pipeline used the default shell,
allowing tee's success to hide a failing formatter or linter. The lint step now
explicitly selects Bash, so GitHub runs it with pipefail. Existing always-run
log upload remains intact. Branch `bug/ci-lint-failure-propagation` preserves the
pending Flatpak fixes; app version 0.0.5 and Android code 11007 are unchanged.

A regression initially rejected the missing shell setting. After the fix, the
actual workflow body was run under GitHub's Bash flags with synthetic make exits
0 and 23: each status propagated and output remained both visible and logged.
Eight CI helper tests and seven Flatpak helper tests passed; diff whitespace
checks passed. No app rebuild or remote workflow run was needed/performed.

## Flatpak SDK libclang — November 16, 2026

[Release job 133301426942](https://github.com/jlips24/XRbox/actions/runs/43867973680/job/133301426942)
passed toolchain staging and FFmpeg compilation, then failed in ffmpeg-sys-next's
bindgen because no libclang shared library was available inside the SDK sandbox.
Host Ubuntu's libclang package does not supply sandbox build dependencies. The
Rust DWARF/debugedit warnings were nonfatal in this run. Pipeline failure now
correctly fails the build step and preserves diagnostics.

Branch `bug/flatpak-sdk-libclang` adds build-only
`org.freedesktop.Sdk.Extension.llvm20` to the manifest and CI SDK installation,
using the pinned runtime branch. The build environment points LIBCLANG_PATH and
its library search path at `/usr/lib/sdk/llvm20/lib`, and exposes the extension's
compiler tools. A preflight loads libclang and checks its API before Cargo starts.
No LLVM runtime extension or host-library bind mount is added to the app.
This follows the [LLVM SDK extension's build configuration](https://github.com/flathub/org.freedesktop.Sdk.Extension.llvm20#usage)
and [Flatpak SDK-extension model](https://docs.flatpak.org/en/latest/extension.html).

Ten Flatpak helper tests and eight CI helper tests passed. Coverage includes
manifest/search-path consistency, CI extension installation, missing/unloadable
libclang, required API presence and failure before version synchronization/Cargo.
Diff whitespace checks passed. No tools were installed on this Mac, no full Linux
ARM64 build/CI rerun occurred, and release version 0.0.6/code 11008 is unchanged.
SDK compilation, export and runtime checks still require a corrected Linux run.

## Manual diagnostic packaging — November 17, 2026

Branch `feat/manual-diagnostic-builds` adds workflow_dispatch to Build with Linux
(default), Android or both as targets. The selected ref resolves once to a commit;
manual runs do not require a release tag or master ancestry. Release events retain
the numeric tag/version/master-history gate. Builds reuse the existing jobs and
read-only permissions. The attachment job is explicitly release-event-only.

Manual artifacts have unique run/attempt labels, record their exact source commit,
mark trigger=workflow_dispatch and tag=null, and retain version 0.0.6/code 11008.
Release verification also rejects manual provenance. Flatpak diagnostic log names
include attempt numbers so failed-job retries can retain fresh logs without a
same-run artifact-name collision. Package artifact retention remains 30 days;
Flatpak logs remain seven days. No release/tag is created or changed.

The complete Python helper suite passed 40 tests with one Linux-only skip. CI
helper regressions also passed under a simulated pull_request environment.
Coverage includes unmerged manual source selection, preserved release gates,
manual metadata, refusal to publish manual artifacts, platform selection and
publish-job guard. Workflow YAML parsed successfully and diff whitespace checks
passed. No hosted workflow was dispatched and no package/app was built locally.
The manual trigger must first land on the default branch before GitHub exposes
Run workflow; see the [manual build guide](releases.md#manual-diagnostic-builds-no-release-needed).

## Pre-Frame packaging checks and warning audit — November 17, 2026

[Manual Linux job 134013303939](https://github.com/jlips24/XRbox/actions/runs/44075762930/job/134013303939)
compiled Rust/FFmpeg, exported Godot, completed a headless synthetic launch and
uploaded `XRbox-0.0.6-linux-arm64.flatpak`. Android and release attachment were
correctly skipped. The old dependency scan included debug companions and did not
propagate every ldd status; therefore its green result was incomplete validation.
The font message matches Flatpak's [missing build-time host font include](https://github.com/flatpak/flatpak/issues/6137).
No headset or Linux streaming acceptance follows from this successful package.

Branch `bug/flatpak-preflight-validation` replaces the broad scan with a labeled
runtime check of engine/shared libraries that excludes debug files and fails on
any ldd error (including 135) or missing dependency. Tests execute the real shell
checker against synthetic libraries in paths with spaces; debug companions are
excluded and both loader failures and missing-library output reject the build.

A CI-only Fontconfig XML lists SDK/app font directories without importing host
configuration. It is scoped to Godot import/export and passed via a temporary file
for runtime synthetic smoke. Both success and failure remove that file and preserve
the app's exit code. No host or installed-app font defaults are changed. Fontconfig
errors now reject Godot checks. A local fc-match check of the same configuration
structure with the bundled-font directory resolved Hack successfully; it does not
establish the SDK's system-font inventory. The synthetic render now captures a
fake sign-in page, and CI retains all page captures. Its default text and bundled
monospaced code were visually inspected on the host preview.

The exact checksum-pinned FFmpeg source was reviewed. The specific val16[3–5]
warning conflicts with the three-symbol recursion bound; the only callers of the
multi-symbol initializer are disabled Ut Video/MagicYUV decoders. An unchanged
function harness passed 78 ASan/UBSan cases at both maximum symbol depths on the
Mac. No FFmpeg patch or warning suppression was added. See [the scoped audit](ffmpeg-vlc-audit.md)
for provenance, reasoning, commands and limitations.

Validation: 44 Python helper tests passed with one Linux-only skip; `make xr-check`
passed five bridge tests and headless Godot checks; synthetic GPU capture passed
nine pages plus room-darkness isolation. Shell syntax and diff whitespace checks
passed. Source version remains 0.0.6/code 11008. No account, stored credentials,
console or headset was accessed. No actual Flatpak build or hosted workflow was
run for this change; the next Linux-only manual run must validate strict dependency
checks and font configuration inside the real SDK/Platform. On Frame arrival,
manually install that identified artifact and perform native launch, stereo,
placement and text-readability gates before considering further hardware tests.
