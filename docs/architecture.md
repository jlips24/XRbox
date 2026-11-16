# XRbox architecture

The sole product frontend is Godot/OpenXR, embedding Rust in one process. Keep
one repository with explicit boundaries; no separate core release/versioning or
IPC service is needed at this stage.

```mermaid
flowchart LR
    UI[Godot scenes and UI: xr] --> BRIDGE[GDExtension: xrbox-godot]
    BRIDGE -->|bounded commands and snapshots| CORE[Session owner: xrbox-core]
    CORE --> AUTH[Microsoft auth and secure refresh store]
    CORE <-->|native WebRTC and Xbox channels| XBOX[Xbox console]
    CORE --> DECODE[Native media backend]
    DECODE -->|latest retained GPU frame| BRIDGE
    BRIDGE --> SCREEN[Godot XR screen]
    CORE --> AUDIO[Opus and CPAL audio]
    UI -->|gamepad samples| BRIDGE
```

- `xrbox-core` owns authentication, discovery, cancellation, bounded queues,
  controller routing and media wiring. It has no Godot dependency.
- `xrbox-godot` translates commands/input/state and owns presentation adapters.
  Workers never receive engine objects; Android GPU imports run on the render thread.
- `xr/` owns scenes, navigation, XR lifecycle and narrow Java platform services.
- `vendor/openxcloud` contains seven pinned libraries. Existing Mac backends and
  upstream GPU utilities stay for provenance/tests; they are not a desktop product.

The standalone egui/eframe 2D client and macOS .app packager were removed by user
request. Git history preserves the validated desktop baseline. macOS development
previews/build tools remain supported; the headset targets are Linux and Android.
Splitting repositories is deferred until an independent consumer or release cycle
justifies versioned public APIs and separate compatibility management.

## Android live implementation — November 12, 2026

Build **0.1.10-xr.2** now includes the first live Android Godot Remote Play path.
This is implemented/build-tested, **not headset or console validated**. The user
requested streaming before the first Quest test, so Android adapters were brought
forward; Linux remains a product target and needs its own decoder/import work.

Godot calls the in-process Rust `Session` through bounded commands. Microsoft
device-code sign-in, secure refresh persistence, discovery and console-only
negotiation reuse the existing core. Godot UI exposes sign-in/copy/browser,
console selection, disconnect/resume and screen controls. No relay or browser
runtime is introduced. Host/Linux Godot previews remain synthetic.

Android startup initializes JNI/CPAL using application context, selects a named
hardware AVC codec, and seals refresh credentials with Android Keystore AES-GCM.
Temporary tokens stay in memory. Android HTTPS uses standard validated TLS with
pinned Mozilla roots. Storage/decoder/import failures surface instead of silently
switching to plaintext credentials or CPU decoding.

MediaCodec outputs PRIVATE ImageReader images. Latest-frame native leases pass to
the render thread for AHardwareBuffer → EGLImage → external OES texture import.
Leases remain alive through GPU fences; bounded retirement may wait when full.
There is no deliberate CPU pixel readback, but actual driver copies, color/crop,
latency, pacing and energy cost require device measurements. Audio uses upstream
Opus/CPAL. Godot gamepad samples feed the existing deferred shortcut recognizer;
physical Guide remains host-reserved. Focus loss disconnects active streaming.

Next gate: follow [Android guide](android.md) on Quest 3S with a real Xbox.
Validate actual moving video, audio, controls, storage, lifecycle and XR separately.
Do not infer successful presentation from packet, decode or GPU import counters.
Full XR settings/hotkey editor, passthrough, Linux hardware media and
battery optimization remain later work. Flatpak packaging is now implemented
(see [Linux guide](linux.md)); its Linux build and runtime acceptance remain pending. No hardware performance claims are made.

## Shared session guarantees

The session owner runs Tokio on a dedicated thread; UI commands and input queues
are bounded. Auth jobs are cancellable and stale results cannot activate a new
session. Only refresh credentials persist; temporary credentials stay in memory.
No credentials are written to settings. Console-only acquisition and signaling
reuse upstream code; no cloud application crate is included.

Streaming readiness requires peer connectivity, input readiness and newly decoded
video. Latest-frame mailboxes prevent an accumulating presentation queue. Audio
uses bounded PCM output. Focus loss/disconnect neutralizes input and tears down
media; teardown never blocks the Godot UI thread.

Select/View and Start/Menu are deferred bindings, including singles. Resolve the
most specific completed chord when the initiating button is released. Physical
Guide stays host-reserved. After menu resume, only Select/Start wait for release;
other held gameplay inputs resume immediately.

Diagnostics expose sanitized categories/counters, never tokens, raw responses,
SDP or addresses. Relay routes are rejected; private ICE addresses alone do not
prove direct LAN routing. Linux hardware decoding/import, release packaging and
battery behavior remain independent hardware gates.

## Source ownership map

Within `crates/xrbox-core/src/session/`, `mod.rs` defines the public handle and
bounded command interface, `worker.rs` coordinates commands/jobs/timers,
`auth_jobs.rs` prepares credentials and discovery, and `active.rs` owns stream
polling, readiness, route inspection, recovery and stop. `tests.rs` exercises the
session boundary. Controller routing stays in `controller.rs`; its unit tests
live in `controller_tests.rs`.

The bridge entrypoint only registers the extension. `runtime.rs` keeps the Godot
class and command API, `snapshot.rs` owns its dictionary contract, `video.rs`
coordinates engine textures, and `android_bootstrap.rs` exports JNI startup.
`android_video.rs` remains the render-thread EGL/native-lease implementation;
`retirement.rs` owns bounded fenced retirement. Neither worker calls engine APIs.

In `xr/scripts/`, `main.gd` composes the scene and coordinates XR lifecycle,
`menu_ui.gd` owns menu nodes/buttons/focus, and `live_ui.gd` chooses pages and
renders session state. Helpers receive explicit objects and callables.
`controller_input.gd` supplies physical samples; `video_surface.gd` assigns and
restores screen materials using `xr/shaders/video.gdshader`. `screen_settings.gd`
contains bounded, session-only screen geometry settings. The floating screen
has a separate frame and a once-rendered idle card; live video retains its existing
native texture path. Menu pages share a navigation rail, inline value controls
and a fixed version footer. Opaque menu pixels use alpha-scissor depth protection;
transparent corners reveal the room, and the dimmer has the lowest render priority.
Android uses 4x MSAA for geometry edges and explicit linear/clamped sampling of
the native external video texture. Snapshots expose submitted decoded-frame
dimensions, clearing them when video is released. Size and placement are separate settings pages. Headless smoke scenarios remain
behind `xr/tests/smoke.gd`; `xr/tests/render.gd` is the synthetic GPU check.

Build helpers remain under `scripts/`. APK byte/ELF inspection lives in
`android_validation.py`; `package_android.py` owns build/export/signature tooling.
Current guidance is organized under `docs/`; superseded narratives remain in
Git history. Vendored source layout is preserved.
