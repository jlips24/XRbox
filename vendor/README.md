# OpenXCloud source provenance

Source: https://github.com/shyim/openxcloud
Revision: 982d2a058c51b5f1285bba90f485bd599a3c2bc0 (2026-11-03)
License: GPL-3.0-only; see the root LICENSE and THIRD_PARTY_NOTICES.md.

Seven complete library crates are vendored, preserving their original layout and
workspace dependencies. The cloud-oriented xcloud-app crate is excluded. This is
a deliberate adjustment from the plan's Git-dependency approach: one local source
snapshot avoids split package identities and preserves reproducible, reviewable
patches without a remote private fork. Versions and inherited dependency versions
remain in the root workspace manifest; Cargo.lock pins the resolved graph.

Changes from the pinned source:

- xcloud-api/auth: add exchange_for_home_tokens, which requests only the xhome
  offering; honor OAuth slow_down by increasing polling intervals; redact raw
  authentication response bodies. Add/update tests for these behaviors.
- xcloud-stream/orchestrator: handle ReadyToConnect transfer-token authentication
  for home as well as cloud sessions; accept Provisioned directly; fail the
  connection wait instead of reporting Connected without peer connectivity.
  Add a home-session regression test asserting /home/play and serverId.
- xcloud-stream/decode_thread test: wait for mailbox publication instead of the
  fake decoder's earlier log write, fixing a race observed in the upstream test.
- xcloud-webrtc/native: expose a sanitized nominated-pair route classification
  using the existing ICE transport API. Add unit and loopback assertions. Raw
  addresses are not exposed to app logs.
- xcloud-proto/video_jitter: drain expired frames when a burst fills the queue
  before rejecting a new timestamp. The old order can wedge reassembly forever.
  Regression test reproduces the old failure and verifies continued decoding.
- cargo fmt applied to patched Rust code.

No transport, packet codec, decoder, renderer, or controller protocol was rewritten.
The root workspace's xcloud-* paths point here. The original repository URL in
workspace package metadata identifies upstream, not an XRbox upstream repository.

Generate the ignored synthetic H.264 fixture with `bash scripts/generate_fixture.sh`
from the XRbox root before running all vendored media tests. The tests' relative
include paths resolve to vendor/openxcloud/fixtures. The original audit checkout
under /private/tmp is not needed to build this repository.

## Android Remote Play integration (November 12, 2026)

Additional narrow platform patches against the same pinned upstream revision:

- xcloud-input/native and its manifest exclude gilrs on Android, just as iOS uses
  an external sampler. XRbox supplies Godot's controller state to the existing
  GamepadInputSource/packet queue; shortcut and packet implementations are reused.
- xcloud-media adds an Android NDK MediaCodec/PRIVATE ImageReader decoder and a
  retained AndroidImage native frame variant. Hardware codec selection comes from
  Android MediaCodecInfo.isHardwareAccelerated, excluding secure/software codecs.
  The existing DecodeThread, bounded AU queue, RTP/jitter/pipeline and Opus/CPAL
  output remain unchanged. Images retain their reader until all frame leases drop.
  Native decoder failure/reset and GPU image import still require device testing.
- xcloud-api's Android HTTPS builder uses the pinned Mozilla webpki-roots with a
  standard Rustls verifier/provider. This avoids assuming the Java components of
  rustls-platform-verifier are hosted in Godot. Certificate and hostname checks
  stay enabled. Other platforms retain their original trust configuration.

New GPU imports/lease retirement and Android Keystore/VM bootstrap live in XRbox's
own core/extension, not the upstream transport or protocol code. No Xbox signaling,
SDP/ICE, auth flow, packet codec or Xbox security control is replaced. Host fake
service/regression tests, Android compile/lint and APK checks are recorded in
VALIDATION.md; they do not establish hardware playback, native color/crop correctness
or zero-copy. The source snapshot revision above remains unchanged.
