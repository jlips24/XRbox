# Validation evidence

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
