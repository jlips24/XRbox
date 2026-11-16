# Roadmap and acceptance gates

1. Accept Android console Remote Play on Quest 3S using [Android guide](android.md).
2. Fix observed runtime problems before adding settings, passthrough or environments.
3. Measure native presentation, power and thermals; counters alone prove no efficiency claim.
4. Implement/qualify Linux ARM64 media and Frame deployment, then Flatpak packaging.
5. Port settings/custom hotkey editing and qualify additional Android headsets.

The sole frontend is Godot/OpenXR with the Rust core embedded in-process. Android
ARM64 and Steam Frame/Linux ARM64 are product targets; macOS is a development host.
The standalone desktop frontend is retired. No cloud gaming or relay PC is planned.

[Product plan](plan.md) · [Engineering gates](xr-plan.md) ·
[Observed evidence](validation.md)
