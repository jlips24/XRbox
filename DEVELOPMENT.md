# Development and validation

## Android live build — November 12, 2026

`make xr-apk` builds/signs/verifies **0.1.10-xr.2** using local tools; it never
installs or launches on a headset. `make xr-apk-check` verifies the existing APK.
See [xr/ANDROID.md](xr/ANDROID.md) for prerequisites, manual console testing and
known limits. `make xr-check` runs native bridge tests plus fake-state headless
Godot scene/UI checks without credentials. Host previews stay synthetic; live
streaming is Android-only in Godot for now. The standalone desktop streaming frontend is retired.

Android requires a linked NDK toolchain, not just a Rust target. The packager sets
`ANDROID_NDK_HOME`, target compiler/linker and CMake ABI locally so Opus builds for
ARM64. It patches only the pinned generated Godot Activity bootstrap and stages
tracked `xr/android-src` platform services. Never change global editor settings,
install tools automatically, or use production keys. Device acceptance is pending.


## Host development and checks

Use project-pinned Rust 1.99.1, Cargo.lock and Godot 4.7.2. macOS is a development
host for the Godot preview and APK build; the standalone desktop app and its .app
packager have been removed. No global toolchain, PATH or editor defaults change.

```sh
make build
make xr-check
make xr-preview
make test
make check
```

`make build` stages the host extension; `make release` stages its optimized
counterpart. Neither exports a finished application. `make run` aliases the
synthetic preview. `make smoke`, `make auth-probe`, `make package` and
`make package-release` were retired with the desktop executable.

The core's `session` feature enables its native stack. Tests explicitly enable
it; no Godot engine objects or UI dependencies belong in the core. Bridge and
scene changes should also run `make xr-check`. Host tests use synthetic state;
do not run ignored OS-credential or physical-audio tests without authorization.

`make test-all` runs every automated suite sequentially: all Rust workspace
crates/targets/features, Python `test_*.py` helpers, headless Godot frontend tests,
and the synthetic GPU render smoke. It requires pinned Godot, a working desktop
GPU/windowing session, FFmpeg/libx264 with headers, CMake and pkg-config. The render
smoke briefly opens a window and writes ignored `.tools/godot-preview.png`; it
uses dummy audio and disables XR/live sessions. On a headless build server, provide
a suitable GPU/display environment rather than silently skipping this stage.

Use `make test-rust-all`, `make test-python`, `make xr-check` or `make xr-render`
for individual suites. Ignored credential/physical-audio tests stay opt-in; real
Xbox, Quest/Frame, Android runtime and battery checks remain manual. `test-all`
does not build/install an APK or operate devices.

Full checks:

```sh
make test-all
make check-all
```

Network tests bind localhost; some GPU tests need an adapter. Report sandbox
restrictions separately from failures. Never authorize an account or operate a
console merely to test a refactor. Existing environment overrides are inherited,
including command-local `CARGO_TARGET_DIR`. See [xr/README.md](xr/README.md) for
Linux export prerequisites and [xr/ANDROID.md](xr/ANDROID.md) for Android tooling.

## Dependency updates on GitHub

`.github/dependabot.yml` schedules Cargo workspace updates daily at 09:00
America/New_York, with at most five open version-update PRs. Related graphics and
WebRTC crates are grouped; other minor/patch updates share a routine group and
other major updates remain separate. Groups are reviewed manually; no auto-merge
is configured. In particular, 0.x minor updates may require API changes.

Push the configuration to the GitHub repository's **default branch** to activate
version updates. In repository Settings, enable the dependency graph, Dependabot
alerts and Dependabot security updates if they are not already enabled. Alerts
and security updates are separate repository settings, not enabled by this YAML.
Security updates are not restricted to the daily version-update schedule.
See [GitHub's configuration documentation](https://docs.github.com/en/code-security/concepts/supply-chain-security/about-the-dependabot-yml-file).

The root entry covers Cargo manifests and the shared lockfile. The pinned Rust
toolchain and vendored OpenXCloud source revision still need manual updates;
Dependabot does not replace that source snapshot. There are currently no GitHub
Actions workflows to maintain. Add a `github-actions` entry if workflows are added.

For dependency PRs, run `make check` and `make test`; use the broader checks from
AGENTS.md when media/protocol dependencies change. Review default features and
native library requirements, especially Godot/JNI compatibility, hardware decoder and GPU import
support. No hosted CI exists yet, so GitHub will not automatically run these local
checks just because Dependabot is configured. Actual Dependabot execution must be
verified on GitHub after upload; local YAML validation alone does not establish it.
