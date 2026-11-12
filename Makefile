.DEFAULT_GOAL := help

CARGO ?= cargo
PYTHON ?= python3
ARGS ?=
VIDEO_FIXTURE := vendor/openxcloud/fixtures/testsrc_1080p60.h264

.PHONY: help build release run smoke auth-probe test test-all check check-all fmt fixture package package-release clean

help:
	@printf '%s\n' \
	  'make build           Build the desktop client' \
	  'make release         Build an optimized desktop client' \
	  'make run             Run the client (optional ARGS="...")' \
	  'make smoke           Render the native UI briefly and exit' \
	  'make auth-probe      Request/discard a Microsoft device code (internet required)' \
	  'make test            Run app, API, session and controller tests' \
	  'make test-all        Generate the fixture and test every crate/feature' \
	  'make check           Check formatting and lint the application' \
	  'make check-all       Check formatting and lint every crate/feature' \
	  'make fmt             Format workspace Rust sources' \
	  'make fixture         Generate the synthetic video fixture (FFmpeg/libx264)' \
	  'make package         Build dist/XRBox.app in debug mode (macOS)' \
	  'make package-release Build an optimized dist/XRBox.app (macOS)' \
	  'make clean           Remove XRBox Cargo artifacts; retain dependency caches'

build:
	$(CARGO) build --locked -p xrbox

release:
	$(CARGO) build --locked --release -p xrbox

run:
	$(CARGO) run --locked -p xrbox -- $(ARGS)

smoke:
	$(CARGO) run --locked -p xrbox -- --smoke

auth-probe:
	$(CARGO) run --locked -p xrbox -- --auth-probe

test:
	$(CARGO) test --locked -p xrbox -p xrbox-core -p xcloud-api -p xcloud-stream -p xcloud-input -- --test-threads=1

test-all: fixture
	$(CARGO) test --locked --workspace --all-targets --all-features -- --test-threads=1

check:
	$(CARGO) fmt --all -- --check
	$(CARGO) clippy --locked -p xrbox -p xrbox-core --all-targets -- -D warnings

check-all: fixture
	$(CARGO) fmt --all -- --check
	$(CARGO) clippy --locked --workspace --all-targets --all-features -- -D warnings

fmt:
	$(CARGO) fmt --all

fixture: $(VIDEO_FIXTURE)

$(VIDEO_FIXTURE): scripts/generate_fixture.sh
	bash scripts/generate_fixture.sh

package:
	$(PYTHON) scripts/package_macos.py --debug

package-release:
	$(PYTHON) scripts/package_macos.py

clean:
	$(CARGO) clean -p xrbox
