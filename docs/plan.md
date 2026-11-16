# XRbox product plan

XRbox ships one Godot/OpenXR app with an in-process, engine-independent Rust core.
Console Remote Play targets Android ARM64 (Quest first) and Steam Frame/Linux ARM64.
The standalone desktop frontend is retired; macOS is a synthetic development host.

## Remaining work

1. Accept Android sign-in, secure restoration, moving video, audio, controls and
   lifecycle on Quest 3S with a real Xbox; fix observed failures first.
2. Implement and qualify Linux ARM64 native decoding/import and Frame deployment.
3. Pass the new Flatpak PR build gate, then qualify runtime permissions and
   headset installation independently of native compilation. See [Linux guide](linux.md).
4. Port settings/custom hotkey editing and qualify additional Android headsets.
5. Measure presentation timing, power and thermals before optimization claims.

See [XR engineering gates](xr-plan.md) for engineering gates, [roadmap](roadmap.md) for
priority, [architecture](architecture.md) for ownership and
[validation evidence](validation.md) for evidence. No cloud gaming, relay PC or second frontend.
