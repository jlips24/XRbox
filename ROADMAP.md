# Roadmap and acceptance gates

## Product scope update — November 12, 2026

The standalone 2D desktop client and macOS .app packager are retired by user
request. macOS remains a development host. Keep the engine-independent Rust core,
Godot adapter and XR project in separate folders in this repository, with atomic
interface/test changes. Earlier desktop harness instructions/results below are
historical; current commands are in README.md and DEVELOPMENT.md. Headset targets
remain Linux ARM64 and Android ARM64. Hardware acceptance is still pending.


## Immediate gate — November 12, 2026

1. Test **0.1.10-xr.2** on Quest 3S against a real Xbox using [xr/ANDROID.md](xr/ANDROID.md):
   startup, sign-in/persistence, moving video, audio, controller shortcuts and lifecycle.
2. Fix observed device issues before adding settings, passthrough or environments.
3. Measure decoder/import/presentation timing, power and thermal behavior; avoid
   claiming latency or zero-copy from counters alone.
4. Implement/qualify Linux ARM64 hardware decoding/import on Frame, then Flatpak
   runtime permissions and packaging. Preserve the working desktop harness.
5. Port validated settings/custom hotkey editing and qualify other Android headsets.

Android live paths now build but have no hardware acceptance. Earlier items below
remain subject to those device gates.


The detailed current plan is [Plan.md](Plan.md). The selected XR frontend is
**Godot with the existing Rust streaming core through an in-process GDExtension**.
Two headset deliverables are required: **Steam Frame Linux ARM64** and an
**Android ARM64 APK for Meta Quest and other validated Android OpenXR headsets**.

1. **Desktop regression acceptance:** native Remote Play is implemented and the
   user reports playback working. Current source is 0.1.10; recent auth, fullscreen,
   settings, hotkey and compact-header changes still need the manual checks in
   VALIDATION.md. Preserve the working desktop client as a regression harness.
2. **Godot/native-extension bring-up (in progress):** shared core extraction,
   engine-neutral notifications and the synthetic Godot/native bridge are tested
   on macOS. Linux/Android ARM64 cargo checks pass; a debug Quest 3S APK now builds and
   passes static package checks. Test Quest 3S while Frame is shipping, then deploy on Steam Frame
   Linux ARM64 and a Meta Quest Android debug APK. Either device can be brought up
   first; Linux is the current priority, and APK remains a first-class deliverable. No full UI
   rewrite before this works on actual devices.
3. **Hardware video presentation:** prove local synthetic H.264 decoding into
   Godot-native GPU textures with explicit synchronization/lifetime management.
   Evaluate an accessible Linux decoder and Android MediaCodec separately; the
   current non-macOS FFmpeg path does not establish efficient headset playback.
4. **Real headset Remote Play:** connect the Rust core to Godot sign-in, console,
   settings, diagnostics and cinema-screen controls. Preserve controller navigation,
   deferred/custom hotkeys, secure credentials and cleanup. Verify live audio/input,
   route classification, reconnect, app focus and headset suspend/resume per target.
5. **Packaging, compatibility and power:** ship reproducible Linux packages and
   signed Android APKs, validate Meta first and each additional Android device
   before claiming support, and add only supported passthrough extensions. Measure
   matched idle/local-video/Xbox-streaming trials and report decoder, pacing,
   latency and power evidence. Godot is selected; renderer/decoder/import choices
   still depend on measurements. System-screen/native-renderer runs are optional
   diagnostic baselines, not a second default XR frontend.

No cloud gaming, relay PC, social features, catalogs, motion-controller gameplay,
complex environments, frame generation or upscaling in the first XR milestone.
Mac Godot checks and cross-target type checks do not establish Android APK,
Frame hardware compatibility, native media presentation or battery behavior.
