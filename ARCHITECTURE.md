# XRBox architecture

One native process, one window, one active Xbox home session. `crates/xrbox`
contains the application; seven pinned OpenXCloud library crates supply the
protocol and media stack. No upstream cloud application UI is included.

```mermaid
flowchart LR
    UI[egui / eframe UI] -->|bounded commands| SESSION[Session owner]
    SESSION --> AUTH[MicrosoftAuthService / OS credential store]
    AUTH <-->|HTTPS auth and signaling| MS[Microsoft services]
    SESSION --> RTC[Native WebRTC and Xbox channels]
    XBOX[Xbox console] <-->|negotiated media and input| RTC
    RTC --> PIPE[RTP jitter / H.264 reassembly]
    PIPE --> VT[DecodeThread / VideoToolbox]
    VT --> MAIL[Latest frame mailbox]
    MAIL --> GPU[Metal plane import / wgpu color conversion]
    GPU --> UI
    PIPE --> OPUS[Opus / PCM ring / cpal]
    PAD[OS gamepad / gilrs] --> INPUT[Xbox input queue]
    INPUT --> RTC
```

## Ownership and flow

`app` (main.rs) owns egui and GPU objects on the UI thread. `session` owns a
Tokio runtime on a dedicated owner thread, accepts bounded typed commands and
publishes a small shared status snapshot. Long auth requests run in cancellable
jobs. Cancelling waits for the job to stop before changing credentials or accepting
another attempt, so an old result cannot activate a later session. There is one
active transport/orchestrator and one per-session decoded-frame mailbox.

`auth` chooses xhome credentials explicitly. Its home-only upstream extension
never requests cloud offerings. Device-code polling supports pending, expiry,
denial, cancellation, and slow_down. `KeyringTokenStore` uses service `xrbox`;
secure persistence is enabled by default, with refresh/discovery at startup on the
session worker. The user can choose session-only `MemoryTokenStore` before signing
in. An empty store stays at sign-in without network requests; refresh/store errors
are shown and never erase saved credentials silently. Successful device-code sign-in
also discovers consoles automatically. Smoke tests bypass restoration entirely.
No credential file exists.

The patched session orchestrator accepts either Provisioned directly or
ReadyToConnect -> transfer token /connect -> Provisioned. It then performs the
existing SDP/ICE exchange and channel handshakes. XRBox reports Streaming only
with peer connectivity, input readiness, and newly decoded video. Retry-start
records a decoded-frame baseline so old decoded frames cannot satisfy readiness.

`media` installs NativeMediaPipeline on the transport worker. H.264 access units
feed the bounded upstream DecodeThread; its latest-frame mailbox avoids an
accumulating presentation queue. macOS VideoToolbox outputs IOSurface-backed
CVPixelBuffers; xcloud-gpu imports their Metal planes, converts color, and presents
without a deliberate CPU readback. GPU completion retains frame resources through
upstream VideoRenderer. All upscalers are bypassed.

Opus decodes into upstream AudioOutput's bounded PCM ring and cpal output callback.
The owner polls device replies; output counters report drops. Input samples every
8 ms only during an active session; upstream channel scheduling sends the queue.
Focus loss and disconnect enqueue neutral state. Input routing is in session.rs
rather than a separate input module because it has no XRBox-specific mappings.

Disconnect aborts outstanding auth work, neutralizes input, stops keepalive,
closes the peer, resets media, clears the frame source, and permits a short bounded
server cleanup window. Upstream close operations have their own timeout. Process
exit joins the owner off the async runtime and shuts down Tokio with a timeout.

## Diagnostics and platform boundary

Only the `xrbox` tracing target is enabled. Upstream raw logs may contain SDP,
addresses, response bodies or account identifiers, so they are not enabled by an
environment override. App error formatting emits categories/status codes, not
raw upstream bodies. Controlled stage logs rotate at startup after 1 MiB.
The nominated ICE pair is sampled off the input/UI thread every five seconds.
Only its route classification reaches the app; addresses never do. Relay routes
cause teardown. Private addresses suggest LAN but do not prove network topology.
Unavailable route information remains unknown.

Desktop rendering consumes upstream Frame/FrameMailbox; the Xbox/session layer
has no egui/OpenXR dependency. A future Frame backend must extend the Linux
native frame representation and import/synchronization path for an actually
accessible hardware decoder. Linux CPU NV12 upload is not zero-copy. First test
the ordinary window on Frame's system screen; add a minimal OpenXR quad presenter
only after desktop and headset streaming acceptance. Do not build XR now.

## Decisions and differences from Plan.md

- Vendor all seven required crates together instead of mixing Git and patched
  package sources. See vendor/README.md and Cargo.lock.
- Use the existing thread-safe snapshot/frame mailbox rather than a watch channel;
  command queue is bounded, media never travels through UI messages.
- One owner loop samples input while active; no additional sampling thread is
  needed. Upstream WebRTC/decoder/audio threads are retained.
- No FFmpeg macOS fallback in the default app, keeping its .app self-contained
  with only system dynamic libraries. Unsupported decoding is a visible failure.
- Secure persistence defaults on; no automatically configured Linux secret service.
- CPU/memory sampling and richer diagnostic exports remain follow-up work.
  Selected-route observation is implemented, but console/LAN and battery behavior
  remain unverified until hardware testing.

## Video stall handling (0.1.1)

The media adapter publishes reassembly, decoder-output, error, queue-drop and
keyframe-gating counters once per second. The owner compares decoder progress
with monotonic time. After three seconds without a decoded frame it requests
recovery with a three-second minimum interval; after fifteen seconds it tears
down the stalled session instead of leaving the last image labeled Streaming.
The upstream jitter buffer now drains expired entries before rejecting a new
timestamp when its bounded queue is full. UI diagnostics distinguish assembled,
decoded and presented frames. The upstream drop counter counts incomplete frames,
so XRBox labels it accordingly instead of claiming it is lost network packets.

## Native clipboard integration (0.1.3)

The eframe dependency disables default features, so native `clipboard` support
must be explicitly enabled. Without it, egui-winit accepts CopyText commands but
only updates an internal string; the OS clipboard is unchanged. Enable eframe's
clipboard feature to route the existing user-initiated copy command through its
native backend (arboard on macOS, with Wayland support resolved for Linux).

## Reduce persistent credential operations (0.1.4)

An application-owned TokenStore adapter delegates refresh-token storage to the
existing cached KeyringTokenStore while keeping MSA access tokens, stream tokens
and LPT/expiry in MemoryTokenStore. Upstream token exchange APIs are unchanged.
Refresh rotation is saved synchronously before the exchange reports success;
storage errors remain visible. Legacy bundles remain readable, without a migration
that deletes credentials. Previously persisted temporary fields are untouched
until sign-out. Sign-out clears transient and durable state, once for the current
persistent service. No token is moved into a plaintext file.

The packager optionally accepts an existing signing identity, retaining ad-hoc
signing by default. No certificates, trust policy or OS defaults are modified.
