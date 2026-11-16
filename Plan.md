# XRbox product plan

XRbox ships one Godot/OpenXR app with an in-process, engine-independent Rust core.
Console Remote Play targets Android ARM64 (Quest first) and Steam Frame/Linux ARM64.
The standalone desktop frontend is retired; macOS is a synthetic development host.

## Remaining work

1. Accept Android sign-in, secure restoration, moving video, audio, controls and
   lifecycle on Quest 3S with a real Xbox; fix observed failures first.
2. Implement and qualify Linux ARM64 native decoding/import and Frame deployment.
3. Qualify Flatpak runtime permissions and packaging independently of native builds.
4. Port settings/custom hotkey editing and qualify additional Android headsets.
5. Measure presentation timing, power and thermals before optimization claims.

See [XR_PLAN.md](XR_PLAN.md) for engineering gates, [ROADMAP.md](ROADMAP.md) for
priority, [ARCHITECTURE.md](ARCHITECTURE.md) for ownership and
[VALIDATION.md](VALIDATION.md) for evidence. No cloud gaming, relay PC or second frontend.

## History

[Earlier implementation plan](docs/history/2026-11-12-Plan.md) preserves desktop-era
work and superseded proposals; it does not authorize restoring that product.
