# Validation evidence

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
