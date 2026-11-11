# Roadmap and acceptance gates

The detailed current plan is [Plan.md](Plan.md). The selected XR frontend is
**Godot with the existing Rust streaming core through an in-process GDExtension**.
Two headset deliverables are required: **Steam Frame Linux ARM64** and an
**Android ARM64 APK for Meta Quest and other validated Android OpenXR headsets**.

1. **Desktop regression acceptance:** native Remote Play is implemented and the
   user reports playback working. Current source is 0.1.10; recent auth, fullscreen,
   settings, hotkey and compact-header changes still need the manual checks in
   VALIDATION.md. Preserve the working desktop client as a regression harness.
2. **Godot/native-extension bring-up:** extract only the reusable Rust core needed
   by the prototype, remove egui notification coupling, pin compatible engine /
   binding / plugin / SDK versions, and load a minimal XR screen on Steam Frame
   Linux ARM64 and a Meta Quest Android debug APK. Either device can be brought up
   first; an APK is a first-class deliverable, not just a fallback. No full UI
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
No Godot, Android APK, Frame hardware compatibility or battery results are claimed
by the desktop tests. This planning update does not implement those components.
