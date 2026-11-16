# Linux ARM64 Flatpak

The release build is configured to produce `XRbox-VERSION-linux-arm64.flatpak`,
containing the Godot executable, exported project/PCK, Rust GDExtension and
FFmpeg shared libraries. It uses the Freedesktop 25.08 runtime, with a desktop
launcher and bundled license/third-party notices. No separately configured XRbox
service or relay computer is needed.

**Status:** the packaging implementation is awaiting its first Linux CI build.
This is an installable-package target for the existing synthetic Linux XR
prototype, not working Linux console Remote Play. Linux decoding/import and
Frame OpenXR, controller and audio behavior still require implementation or
hardware acceptance. The standalone 2D frontend remains retired.

## Install a successfully built release

On an ARM64 Linux system with Flatpak already installed, download the `.flatpak`
from a release whose Linux build passed. Install per user and launch:

```sh
flatpak install --user ./XRbox-VERSION-linux-arm64.flatpak
flatpak run io.github.jlips24.XRbox
```

Replace VERSION with the release version. Flatpak may offer to install the required
Freedesktop runtime from Flathub. The bundle is not a Flathub store submission or
an automatic-update repository; download/install a later bundle to update it.
It does not run on x86_64 Linux. For a synthetic window without attempting XR:

```sh
flatpak run io.github.jlips24.XRbox -- --desktop
```

A successful install or headless smoke does not establish Frame compatibility.
Do not authorize an account or connect a console for this synthetic Linux gate.

## Build

Use a Linux ARM64 development host with the project-pinned Rust toolchain,
Cargo.lock, `flatpak`, `flatpak-builder` and `elfutils` (`eu-strip` and
`eu-elfcompress`). Install these prerequisites yourself;
`make xr-flatpak` never installs host tools/runtimes or the application. Prepare
the build runtime and SDK explicitly if they are not already available:

```sh
flatpak --user remote-add --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak --user install flathub org.freedesktop.Platform/aarch64/25.08 org.freedesktop.Sdk/aarch64/25.08
make xr-flatpak
```

The helper copies nonignored repository sources, vendors the locked Cargo graph,
and stages the existing pinned compiler into an ignored build input directory.
Godot/editor/templates, FFmpeg and Godot legal notices are downloaded from their
pinned URLs and verified against checksums in `xr/toolchain.json`. The native core
and bundled FFmpeg compile **inside the SDK**, not against host Ubuntu libraries.
The SDK compiler is removed from the finished app. SDK/runtime commits may advance
within 25.08; this is not a claim of bit-for-bit reproducibility.

Generated inputs, the builder cache and OSTree repository live in
`.tools/flatpak/`. The app output lives in `dist/xr/linux-arm64/`. Rust builds use
the isolated SDK workspace and honor CARGO_TARGET_DIR if explicitly set inside
that environment; the host Cargo target/cache is not copied into the app.
Only versioned/nonignored source files enter staging: no `.git`, ignored keys,
credentials, native output, Android templates or editor caches.

Before creating the bundle the helper checks exported executable/PCK presence,
transitive ELF dependencies under the **Platform** runtime, and a bounded
headless synthetic startup with `--test`, dummy audio and XR disabled. Error
markers and nonzero exits fail the build. It never opens credentials or starts
live auth/streaming. Only the release workflow runs this full packaging command automatically;
PRs and pushes to master run the normal synthetic checks without building a
Flatpak. Packaging logs are retained in Actions.

## Sandbox and hardware boundary

Wayland/X11, IPC and PulseAudio support the current Godot frontend. Device access
uses Flatpak's broad `--device=all` for gamepads/XR devices, compatible with the
Ubuntu runner's Flatpak version; it needs narrowing/qualification before a stable
release. No system-gesture capture is enabled. There is no general host/home
filesystem access, network permission or Secret Service D-Bus permission in this
synthetic build. Linux live services must add and validate their own permissions
when implemented; secure refresh persistence must remain the default for live mode.

Read-only `xdg-config/openxr` and `xdg-run/monado_comp_ipc` access are declared for
OpenXR configuration/IPC. Host runtime library availability and Steam Frame's
actual OpenXR deployment are **not established** by these declarations; an
external runtime may need different integration. Do not grant blanket filesystem
or session-bus access to hide missing runtime dependencies.

On Frame, record the exact Flatpak filename/revision, OS/OpenXR runtime and visible
results. Test install/launch, stereo/head tracking, synthetic screen placement,
controller navigation, focus loss/return and quit using
[the Frame plan](frame-test-plan.md). Stop and report errors before proceeding to
console streaming. GPU/decoder efficiency, direct LAN routing and battery behavior
remain separate gates.
