# Meta Quest 3S Remote Play development APK

Build **0.1.10-xr.2**, package **org.xrbox.prototype**, version code **11002**.
The APK now includes Xbox console Remote Play: supported Microsoft device-code
sign-in, console discovery, native WebRTC, MediaCodec video, Opus/CPAL audio and
Bluetooth gamepad input. These paths build successfully but **have not been run
on a headset or against a console**. No PC or relay service is part of runtime.

## Install and test

1. Enable Xbox Remote Features through the console's supported settings, keep the
   console awake for this first test, and put both devices on the same LAN.
   Start a game or moving console screen so a frozen image is easy to recognize.
2. Enable Quest developer mode and USB debugging using Meta's supported setup.
   Pair a Bluetooth Xbox controller. Touch controllers/hand interaction are not
   implemented. Install manually from the repository root:

   ```sh
   adb install -r dist/xr/android/XRBox-0.1.10-xr.2-quest-debug.apk
   ```

3. Launch **XRBox XR Prototype** from **Unknown Sources**. Confirm build xr.2,
   android/aarch64 and **OpenXR active**; confirm stereo and head tracking.
4. Select **Sign in**, copy the displayed code or enter it on another device at
   the displayed Microsoft verification URL. The browser action is optional;
   return to XRBox after approval. Never share codes or tokens in a bug report.
   Remember sign-in defaults on; choose session-only before starting if desired.
5. Select your discovered console, then connect. Console lists paginate when
   needed. Confirm the picture actually moves, audio is audible, and stick,
   D-pad, face-button and trigger input reaches the console.
6. Select/View and Start/Menu are deferred until the first is released. Their
   pair sends virtual Guide; pair + LB + RB opens XRBox's menu. Check that the
   pair does not also press the individual buttons. Physical Guide stays reserved
   to the headset. B remains gameplay input while streaming; use the chord for
   the local menu. Resume should restore held gameplay input immediately while
   only View/Menu wait for release.
7. Test screen width/distance, recenter and vertical flip if necessary. Screen
   controls are session-only. Inspect status counters for assembly, decoding,
   decoder errors, GPU imports, audio and drops, plus codec/status and route.
   Network traffic, decoded counts and GPU imports alone do not prove presentation.
8. Disconnect, reconnect and quit. Opening a system menu, losing XR focus or
   removing the headset disconnects active media; reconnect manually on return.
   Browser sign-in polling can continue when no media session is active. Unplug
   the controller during gameplay and verify input neutralizes before reconnect.
9. Relaunch to check remembered sign-in and console discovery, then confirm
   sign-out removes remembered credentials. Android Keystore persistence and
   rotation still need this device test; no agent has read the credential store.

Report build, Quest OS version, XR startup, whether video moves, audio/controller
results, stage counters and any visible error. For app-only logs:

```sh
adb shell pidof org.xrbox.prototype
# Substitute that PID; do not clear all device logs:
adb logcat --pid=12345 -v threadtime
```

Review logs before sharing; do not include auth codes, tokens, raw SDP or addresses.
If the process crashes before capture, report the symptom for targeted follow-up.
No headset is polled, installed or launched by our build commands.

## Rebuild on this Mac

```sh
make xr-apk
make xr-apk-check
```

make xr-apk runs packaging regression tests, builds/stages host and Android native
extensions, stages the pinned upstream vendor addon, includes notices, exports a
debug-signed APK, then verifies signature, zip/native 16 KiB alignment, ARM64-only
libraries, one OpenXR loader, required Quest manifest entries and excluded keys/
tests/desktop libraries. It does not install or operate a device. make xr-apk-check
only verifies the existing APK. The debug key lives under ignored .tools/android;
retain it to update this package without a signature mismatch. No release key is
created. The default debug password is the public Android development convention.

Tool paths and overrides are process-local. The helper edits **only** the
self-contained editor settings under .tools/godot/editor_data. It rejects a global
Godot executable/configuration. Do not run two editors/exports against that local
configuration simultaneously. The export preset has runnable=false to disable
editor device polling; manual installation remains available.

## Prerequisites on another development machine

Nothing installs SDKs or changes PATH/defaults when make xr-apk runs. Supply:

- A project-local self-contained **Godot 4.7.2 stable** editor and matching Android
  export templates. The current Mac binary is
  .tools/godot/Godot.app/Contents/MacOS/Godot, with _sc_ beside that executable.
  Start/import once so its local editor_settings-4.7.tres exists. Templates are
  under .tools/godot/editor_data/export_templates/4.7.2.stable. The exact template
  archive URL/SHA512 and dependency checksums are in toolchain.json. Extract
  android_debug.apk, android_release.apk and android_source.zip from templates/.
- **JDK 17**. The helper respects a command-local JAVA_HOME; on macOS it otherwise
  asks java_home for the existing JDK 17. It never changes the default Java.
- Android SDK command-line tools **19.0**, Build-Tools **36.1.0**, Platform **36**,
  NDK **29.0.14206865**, and platform-tools (tested **37.0.1**) under
  .tools/android/sdk, or a command-local XRBOX_ANDROID_SDK pointing at that SDK.
- Project-pinned Rust **1.99.1** and the aarch64-linux-android standard library:

  ```sh
  rustup target add --toolchain 1.99.1 aarch64-linux-android
  ```

The older stable web documentation lists different Android tools. The selected
versions come from the actual **4.7.2 android_source.zip/config.gradle**. They
are build-tested here; changing engine/template versions requires requalification.
The extension links against Android API 29. The exported APK currently declares
min SDK 29 and target SDK 36; these are verified artifact values, not Quest OS
compatibility claims. The renderer has an **Android-only Compatibility/OpenGL**
override; Linux retains its Mobile renderer selection.

For an explicit SDK setup, obtain Google's command-line tools archive for your
host, verify its checksum, and place it at SDK/cmdline-tools/19.0. Then run the
following manually, accepting only the required SDK licenses if you agree:

```sh
JAVA_HOME=/absolute/path/to/jdk17 .tools/android/sdk/cmdline-tools/19.0/bin/sdkmanager \
  --sdk_root="$PWD/.tools/android/sdk" \
  'build-tools;36.1.0' 'platforms;android-36' 'ndk;29.0.14206865' 'platform-tools'
```

The Android helper currently supports macOS and Linux build hosts with a matching
NDK host toolchain. Only macOS Apple Silicon has actually built this APK. Supply
GODOT as a command-local path for a Linux portable editor inside .tools/godot.
The upstream plugin dependency may be downloaded automatically to the ignored
cache, but is checksum-verified; unmanaged addon directories are not overwritten.
No source changes to the vendor plugin are made.

## Implemented pipeline and remaining boundary

The existing console-only Rust session owner runs off the UI thread. Android
uses a named hardware H.264 MediaCodec, PRIVATE ImageReader buffers and EGLImage
import into a Godot external OES texture on the render thread. Frame leases are
retained through GPU fences; the bounded retirement queue waits if exhausted.
There is no deliberate CPU pixel readback. Driver copies, color/crop correctness,
frame pacing, latency, thermal behavior and battery consumption remain unmeasured.
No software-decoder fallback silently replaces unavailable hardware support.

Only refresh credentials are AES-GCM sealed with an Android Keystore key; encrypted
blobs/IVs are in app-private preferences. Access/stream/transfer tokens stay in
memory. Storage errors are visible. Android HTTPS uses pinned Mozilla trust roots
with normal TLS verification, not a Java verifier adapter or a trust bypass.
Audio reuses upstream Opus and CPAL's Android backend; synchronization, routing and
physical playback are unverified. No microphone/camera/location permission is used.

Static package checks and host tests do not establish native startup, successful
sign-in, Xbox playback, hardware decoding, zero-copy or XR output. Full settings,
custom hotkey editing in XR, haptics, passthrough, release signing/store submission
and other-headset qualification are later work. Linux Godot streaming is not yet
enabled; the standalone desktop harness is retired. Linux decoding/import and
Flatpak permissions need independent implementation and validation.

Primary references: [Godot ExternalTexture](https://docs.godotengine.org/en/stable/classes/class_externaltexture.html),
[Android MediaCodec](https://developer.android.com/reference/android/media/MediaCodec),
[Android Keystore](https://developer.android.com/privacy-and-security/keystore),
[Godot Android OpenXR](https://docs.godotengine.org/en/stable/tutorials/xr/deploying_to_android.html).
