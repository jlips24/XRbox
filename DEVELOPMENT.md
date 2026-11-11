# Development and validation

Builds use the project-pinned Rust 1.99.1. Do not change PATH, global toolchain,
shell settings, default browser, system services, or OS defaults for this project.
Build caches can be selected per command using CARGO_TARGET_DIR; no export or
persistent shell edit is needed. The Mac app needs CMake and the platform SDK.

```sh
cargo build --locked -p xrbox
cargo run --locked -p xrbox -- --smoke
cargo run --locked -p xrbox -- --auth-probe
cargo test --locked -p xrbox -p xcloud-api -p xcloud-stream -p xcloud-input -- --test-threads=1
cargo clippy --locked -p xrbox --all-targets -- -D warnings
cargo fmt --all -- --check
```

`--smoke` renders ten UI frames and exits without reading credentials or signing
in. `--auth-probe` requests a Microsoft device code in memory, discards it and
exits without authorizing an account. The latter needs internet access.

To run every vendored media/GPU test, install developer FFmpeg with libx264,
CMake/pkg-config and FFmpeg headers (these are development prerequisites, not
macOS end-user runtime dependencies), then:

```sh
bash scripts/generate_fixture.sh
cargo test --locked --workspace --all-targets --all-features -- --test-threads=1
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
```

Do not run ignored keychain or physical audio tests automatically. Network tests
bind localhost and may need execution outside a restricted sandbox. GPU tests
need an available adapter. The fixture is synthetic and ignored; tests do not
access a real Xbox or substitute cloud streaming.

```sh
python3 scripts/package_macos.py --debug
# For an optimized package:
python3 scripts/package_macos.py
```

The packager places XRBox.app under dist, rejects non-system dylib references,
and ad-hoc signs it. A distributable release needs clean-machine testing,
Developer ID signing/notarization, and dependency/license review. Package the
corresponding source and notices when distributing GPL-derived binaries.

For isolated checks, prefix an individual command with
`XRBOX_DATA_DIR=/private/tmp/xrbox-test-data`; this only redirects that process's
non-secret log directory and does not modify shell settings.

Controlled app logs are diagnostics.log in the platform's XRBox application-data
directory (printed on startup); no raw upstream logs are enabled. Diagnostics
currently show partial stream measurements; RTT defaults are not presented as
measured values. Route labels are nominated-pair observations, not proof of LAN
topology; no selected route is claimed until a real session is tested.

Manual console procedure and current limitations are in README.md. Stop for the
user's Xbox results before XR or Steam Frame implementation. Windows and Linux
ARM64 builds have not been executed for XRBox; do not infer success from Rust
portability or an upstream CI workflow.
