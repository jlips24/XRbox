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
Dependabot does not replace that source snapshot. GitHub Actions updates use a separate daily 09:00 America/New_York schedule,
with at most two open update PRs and one grouped action update.

For dependency PRs, run `make check` and `make test`; use the broader checks from
AGENTS.md when media/protocol dependencies change. Review default features and
native library requirements, especially Godot/JNI compatibility, hardware decoder and GPU import
support. The Test workflow runs checks for PRs targeting master once these workflows are
on GitHub. Inspect the actual run before claiming success. Actual Dependabot
execution must also be verified on GitHub; local YAML validation establishes neither.

## Test output

`make test-all` captures each suite's full stdout/stderr in a unique ignored
`.tools/test-results/` directory and prints progress plus a final PASS/FAIL table
with durations and failure-log paths. Other suites still run after a failure;
the command exits nonzero if any suite fails. Ctrl-C reports the interrupted and
unrun suites. Individual suite targets retain their normal verbose output.
Logs are kept across runs; remove old result directories manually when no longer
needed. The runner does not authorize accounts or operate devices. The synthetic
GPU smoke still briefly opens a window. Exit status, not log text, determines success.

## Finding code and evidence

[ARCHITECTURE.md](ARCHITECTURE.md#source-ownership-map) maps responsibilities to
modules. Keep private Rust regressions beside the owning module (large test bodies
may use sibling files); Python helper tests stay beside scripts. Godot's smoke
entrypoint groups navigation, placement, live pages, video material lifetime and
shutdown scenarios. Every new scenario must remain reachable from `make xr-check`.

[Plan.md](Plan.md) and [XR_PLAN.md](XR_PLAN.md) track unfinished product/engineering
work; [VALIDATION.md](VALIDATION.md) indexes observed evidence. Superseded desktop
plans and results are preserved under `docs/history/`, not current instructions.

## GitHub Actions

Inspect the workflow run for the tested commit before claiming CI passed.
Local validation does not establish hosted runner or release build results.

- **Test**: `pull_request` targeting `master`, plus pushes to `master` to refresh
  caches for future PRs; formatting, full workspace Clippy
  (`make check-all`) and `make test-ci`. GPU smoke uses Xvfb/software Vulkan as
  a synthetic fixture, not Frame compatibility evidence. Logs are retained for
  seven days. Ignored credential/audio tests and real account/device tests remain off.
- **Build**: a new pushed tag or published release whose tag exactly matches
  `vX.X` or `vX.X.X` (numeric components). Other names, including prerelease
  suffixes, skip builds. Moving an existing tag does not trigger a new build.
- Build outputs are the optimized **Linux ARM64 native extension** with runtime
  dependency information, and a verified **Android development APK**. Artifacts
  include source/version/checksum metadata and are retained for 30 days. A native
  extension is not an exported Linux app or proof of streaming compatibility.
- Both a tag push and later publishing its release can each produce a build run.
  These workflows do not create releases, attach release assets, deploy or operate
  devices. Source versions remain authoritative; the tag identifies the build.
- Actions use pinned commit SHAs and read-only repository permissions. Godot,
  export templates, SDK/NDK and Rust stay pinned; downloads are verified or managed
  by the SDK manager. Tool installation happens only on ephemeral hosted runners.
- Android uses the existing self-contained project-local editor and an ephemeral
  debug signing key, never production keys. Separate runs can have different debug
  signatures and may not update an existing locally signed installation in place.

The shared runner setup is `.github/actions/setup/action.yml`; `scripts/ci.py`
handles checksum-verified editor/templates, isolated SDK setup, exact tag gating
and artifact provenance. Dependabot covers action pins without auto-merging them.

### CI compile reuse and failure logs

`make test-ci` retains all four suites and all Rust targets/features/assertions.
Its Godot stage uses `xr-ui-check` because bridge unit tests already ran in the
full Rust suite. Godot staging uses `--workspace-features` to match that suite's
Cargo feature graph rather than compiling a second, narrower dependency graph.
Standalone `make xr-check` still runs the bridge unit tests; `make test-all`
retains its existing local behavior. Cross-target/Android builds cannot use the
host workspace mode.

The Test workflow caches Cargo dependencies and workspace artifacts using the
pinned Rust/compiler/manifests/lockfile/environment identity. Failed test runs
also retain the cache; this never changes their failure status. Pushes to master
seed a base cache that new PRs can restore. The first cache miss still compiles.
Actual warm-run timings must be measured on GitHub; no speedup is assumed from
local tests.

Godot import, headless UI and GPU smoke write separate `godot-import.log`,
`godot-check.log` and `godot-render.log` files under ignored `.tools/`; later
rendering no longer overwrites failed-import diagnostics. Signal exits are named
explicitly and still fail the suite. Verbose import logging helps investigate
Linux editor aborts without weakening any assertions or suppressing errors.

The Linux Test workflow installs GDB on its ephemeral runner and enables
`XRBOX_GODOT_CRASH_BACKTRACE=1`. If a captured Godot process exits by signal,
the helper reruns that phase under GDB (up to 120 seconds) and records all-thread
backtraces and loaded libraries in `.tools/godot-<phase>-backtrace.log`. The
debugger rerun has a separate engine log and never replaces the original failure.
These logs are included in the existing validation artifact. A non-reproducing
rerun or stripped engine symbols may limit the evidence; no crash fix is implied.
Local commands do not enable the debugger or install it.
