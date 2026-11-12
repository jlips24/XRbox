# Meta Quest 3S development APK

Build **0.1.10-xr.1**, package **org.xrbox.prototype**, version code **11001**.
This is the Godot **synthetic XR screen/menu prototype**. It does not sign in,
connect to an Xbox, stream games or play audio. Linux remains a product target;
Quest 3S is the first available headset for on-device validation while Frame ships.

## Install and test the provided APK

The output is dist/xr/android/XRBox-0.1.10-xr.1-quest-debug.apk, with a SHA256 sidecar.
No headset has been installed/launched by the build tools. The APK is signed with
a local development key, not a production/store key.

1. Enable developer mode for your Quest account/headset through Meta's supported
   developer setup, then connect the Quest 3S by USB. Accept USB debugging inside
   the headset. No PC is needed after installation.
2. Pair your **Bluetooth Xbox controller** with the Quest. This prototype uses
   Android gamepad input; Touch motion-controller/hand UI is not implemented.
3. From the repository root, install manually with your existing adb:

   ```sh
   adb install -r dist/xr/android/XRBox-0.1.10-xr.1-quest-debug.apk
   ```

4. Open **XRBox XR Prototype** from the headset's **Unknown Sources** library.
   Alternatively launch its exported alias manually:

   ```sh
   adb shell am start -n org.xrbox.prototype/com.godot.game.GodotAppLauncher
   ```

5. Confirm the displayed build is **0.1.10-xr.1**, native target is **android /
   aarch64**, and status says **OpenXR active**. A flat window or fallback label
   does not pass XR acceptance. Confirm stereo and head tracking in the headset.
6. Use D-pad to navigate, A to select and B to hide/show the menu. Test width,
   distance, reset and recenter. Select + Start + LB + RB also opens the menu.
   Verify the screen stays still as your head moves, begins at eye height, and
   recenter works when seated and after turning. Settings are session-only.
7. Test removing/replacing the headset, opening/closing the system menu and
   disconnecting/reconnecting the Xbox controller. Quit and reopen the app.
   Report any misplaced screen, clipped menu, unresponsive control or crash.

Keep Meta's physical system buttons reserved. A keyboard can navigate the desktop
preview; this APK does not add touch/pointer picking. No passthrough, decoder,
stream performance, battery or zero-copy claim follows from this test.

If launch fails, capture only the app's process logs rather than an entire device
log (which can contain unrelated account/app data):

```sh
adb shell pidof org.xrbox.prototype
# Substitute the reported PID:
adb logcat --pid=12345 -v threadtime
```

If the process dies before you can capture it, report the crash symptom and we
will choose a targeted capture. Do not clear all device logs or change system
settings to debug this prototype. Report headset OS version, build label, whether
XR loaded, controller behavior and a screenshot/photo if practical.

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

## Known boundary

Signature/manifest/native checks and host Godot tests pass. **Quest installation,
Android native loading, XR output and physical input have not been tested yet.**
The first tracked pose triggers one initial recenter; subsequent head motion does
not move the screen. This behavior has headless regression coverage, and needs
headset confirmation. Android pause/physical input and OpenXR focus transitions
also remain manual checks before any live stream integration.

Only a debug APK is produced. Full release signing, store submission, exhaustive
license/source bundling, other-headset compatibility, MediaCodec video and secure
Android Xbox credentials remain later milestones. Primary references:

- [Godot Android export](https://docs.godotengine.org/en/stable/tutorials/export/exporting_for_android.html)
- [Godot Android OpenXR](https://docs.godotengine.org/en/stable/tutorials/xr/deploying_to_android.html)
- [Pinned Meta exporter source](https://github.com/GodotVR/godot_openxr_vendors/blob/5.1.0-stable/plugin/src/main/cpp/export/meta_export_plugin.cpp)
- [Meta developer setup](https://developers.meta.com/horizon/documentation/native/android/mobile-device-setup/)
