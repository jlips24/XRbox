# Validation evidence

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
- [Wednesday test plan](xr/FRAME_TEST_PLAN.md) and
  [results template](xr/FRAME_RESULTS_TEMPLATE.md) explicitly gate live testing on
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
Follow the [reorganization retest](xr/ANDROID.md#reorganization-retest) manually.

## Baseline recorded November 12, 2026

These are prior recorded results, not a fresh run of the reorganized source:

- Desktop-retirement checks: `make test` recorded 221 passed, zero failed,
  one ignored; `make check` and `make xr-check` passed.
- Android live implementation: workspace/all-feature tests recorded 564 passed,
  zero failed, two ignored; strict host and Android lint passed.
- APK 0.1.10-xr.2 was built/signature/ABI/alignment checked. Its recorded SHA256 is
  `6e9bd35d001f78224331d313335d68283d39ddc1a2b29e1bf76eb79a767bcd5a`.

The exact environments, suite scope and older results are preserved in the
[validation history](docs/history/2026-11-12-VALIDATION.md). Historical desktop
observations do not establish headset playback.

## Open acceptance

Quest/console sign-in, Keystore, moving XR video, audio, physical controls and
lifecycle remain unaccepted. Linux ARM64 decoding/import, Frame, Flatpak, direct
LAN routing, zero-copy, latency and battery remain independently unvalidated.
Follow [xr/ANDROID.md](xr/ANDROID.md) for manual device checks and
[DEVELOPMENT.md](DEVELOPMENT.md) for automated commands.

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
