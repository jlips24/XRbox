# XR engineering gates

The Android live implementation exists; hardware acceptance remains pending.
Host previews are synthetic. Keep the core, bridge and Godot project in one repository.

| Gate | Remaining evidence or implementation |
| --- | --- |
| Android runtime | Quest startup, Keystore refresh/rotation/sign-out, actual moving video, audio and controller behavior |
| Native presentation | Correct color/crop, bounded surface lifetime, reconnect and teardown with frames in flight |
| Lifecycle | XR focus loss, browser return, headset removal, disconnect/reconnect and process recreation |
| Linux ARM64 | Linked native extension, qualified hardware decoder/import, Frame OpenXR and audio/input |
| Flatpak | Release-only packaging implemented; first Linux build, narrower device permissions and Frame runtime qualification pending |
| Compatibility/power | Per-headset results and matched idle/local-video/streaming measurements |

Preserve deferred shortcuts, bounded queues, refresh-only secure persistence and
nonblocking UI teardown. Do not infer presentation from network or decode counters.
Full settings/hotkey UI and passthrough follow basic runtime acceptance.

Build commands and prerequisites belong in [development guide](development.md).
Android packaging and exact manual checks belong in [Android guide](android.md).
Record observations in [validation evidence](validation.md); architecture belongs in
[architecture](architecture.md).
