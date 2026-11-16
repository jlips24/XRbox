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
