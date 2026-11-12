.DEFAULT_GOAL := help

CARGO ?= cargo
PYTHON ?= python3
ARGS ?=
VIDEO_FIXTURE := vendor/openxcloud/fixtures/testsrc_1080p60.h264

.PHONY: help build release run test test-all test-rust-all test-python check check-all fmt fixture clean

help:
	@printf '%s\n' \
	  'make build           Build/stage the host Godot extension' \
	  'make release         Build/stage an optimized host Godot extension' \
	  'make run             Run the host synthetic Godot preview' \
	  'make test            Test core/session, bridge and protocol libraries' \
	  'make check           Check formatting and lint core and bridge' \
	  'make test-all        Run Rust, Python, Godot frontend and GPU smoke tests' \
	  'make test-rust-all   Generate fixture and test every Rust crate/feature' \
	  'make test-python     Run Python helper regression tests' \
	  'make check-all       Check formatting and lint every crate/feature' \
	  'make fmt             Format workspace Rust sources' \
	  'make fixture         Generate synthetic H.264 fixture (FFmpeg/libx264)' \
	  'make clean           Remove project Rust artifacts; retain dependency caches' \
	  'make xr-build        Build/stage extension (ARGS for target/profile)' \
	  'make xr-check        Build extension and run native/headless Godot tests' \
	  'make xr-render       Run synthetic GPU smoke (opens a short-lived window)' \
	  'make xr-run          Run host synthetic scene with OpenXR' \
	  'make xr-preview      Run host synthetic scene without OpenXR' \
	  'make xr-apk          Build/sign/verify Quest development APK' \
	  'make xr-apk-check    Verify existing APK (no device access)'

build: xr-build

release:
	$(PYTHON) scripts/godot.py build --release $(ARGS)

run: xr-preview

test:
	$(CARGO) test --locked -p xrbox-core -p xrbox-godot -p xcloud-api -p xcloud-stream -p xcloud-input --features xrbox-core/session -- --test-threads=1

# Keep stages sequential, including under make -j: Godot import/staging share files.
test-all:
	$(MAKE) test-rust-all
	$(MAKE) test-python
	$(MAKE) xr-check
	$(MAKE) xr-render

test-python:
	$(PYTHON) -m unittest discover -s scripts -p 'test_*.py'

test-rust-all: fixture
	$(CARGO) test --locked --workspace --all-targets --all-features -- --test-threads=1

check:
	$(CARGO) fmt --all -- --check
	$(CARGO) clippy --locked -p xrbox-core -p xrbox-godot --all-targets --features xrbox-core/session -- -D warnings

check-all: fixture
	$(CARGO) fmt --all -- --check
	$(CARGO) clippy --locked --workspace --all-targets --all-features -- -D warnings

fmt:
	$(CARGO) fmt --all

fixture: $(VIDEO_FIXTURE)

$(VIDEO_FIXTURE): scripts/generate_fixture.sh
	bash scripts/generate_fixture.sh

clean:
	$(CARGO) clean -p xrbox-core -p xrbox-godot

# Optional XR tools remain command-local; GODOT can be an absolute executable path.
.PHONY: xr-build xr-check xr-render xr-run xr-preview
xr-build:
	$(PYTHON) scripts/godot.py build $(ARGS)

xr-check: xr-build
	$(CARGO) test --locked -p xrbox-godot --lib
	$(PYTHON) scripts/godot.py check

xr-render: xr-build
	$(PYTHON) scripts/godot.py render

xr-run: xr-build
	$(PYTHON) scripts/godot.py run

xr-preview: xr-build
	$(PYTHON) scripts/godot.py run --desktop

.PHONY: xr-apk xr-apk-check
xr-apk:
	$(PYTHON) -m unittest discover -s scripts -p test_package_android.py
	$(PYTHON) scripts/package_android.py

xr-apk-check:
	$(PYTHON) scripts/package_android.py --verify-only
