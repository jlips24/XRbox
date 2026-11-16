# Steam Frame acceptance — Wednesday, November 18, 2026

Target: standalone **Linux ARM64**, one Godot/OpenXR app, direct Xbox console
Remote Play, no relay PC. **GPU-native hardware video is required.** A software
decoder, CPU pixel readback/upload or Android/Lepton run cannot pass this gate.
Use an Xbox dashboard or an already installed game first; no particular game's
release or availability is a prerequisite.

## Readiness record

Record bundle SHA256, build.json revision/dirty flag, app/core/Godot versions,
Frame SteamOS build, OpenXR runtime, renderer and physical controller model.
Current source has a polished shared UI and explicit Meta menu bindings. **There
is no complete Linux playback candidate yet.** Linux live startup remains disabled;
no experimental engine patch, Vulkan probe or CPU-video fallback is included.
The source can support synthetic UI checks; a native headset candidate must be
built and identified separately before on-device cases can execute.

Before live testing, the candidate must include a linked ARM64 native extension,
all redistributable native dependencies/notices, a named hardware H.264 decoder,
a retained native-buffer path and GPU-only import/conversion into Godot. The
stock-engine investigation must establish which extensions are actually enabled
on the device, rather than merely advertised by its physical GPU. A custom
engine is not yet selected or proven necessary.

## Wednesday session order

Timebox the first session to about two hours. Start with the shipped stock engine
and observed device capabilities; do not select an engine patch from Mac/Docker
behavior. Keep [results template](frame-results-template.md) alongside
the identified candidate and mark every case PASS / FAIL / NOT RUN.

1. **0–15 minutes:** deploy, confirm native launch and actual headset stereo/XR.
2. **15–30 minutes:** run `sh frame_collect.sh > frame-capabilities.txt` on Frame.
   It queries OS/GPU/decoder advertisements only, never starts a decoder/camera,
   signs in, installs tools or changes device configuration. Missing tools are
   reported as unavailable. Review the output before sharing it.
3. **30–50 minutes:** exercise UI, controls, geometry and lifecycle. On Android,
   repeat the Meta-only navigation cases using the matching 0.0.1 APK.
4. **Hardware-video gate:** if the native candidate is unavailable or fails,
   record the exact missing capability and stop before auth/live streaming.
   Bring that evidence back for a small targeted implementation/retest. Do not
   substitute CPU video to get a pass.
5. If the native-video gate passes, continue through sign-in/gameplay/recovery
   and the endurance run below. Extend the session rather than skipping gates.

Current source cannot pass the hardware-video gate because the Linux
decoder/import adapter is not implemented. This is a known preparation
gap, not a result to discover by repeatedly attempting to connect on Wednesday.

## Preparation and manual deployment

1. Keep Xbox Remote Features enabled; use the supported console/account flow.
   Pair a Bluetooth Xbox controller with Frame. Meta/Frame XR controllers operate
   XRbox menus only; they do not emulate an Xbox gameplay controller.
2. Transfer/extract the bundle using Valve's Steam DevKit workflow. Do not install
   development packages on the headset or change its system runtime configuration.
   Keep the entire directory, including PCK, native extension, lib/ and notices.
3. Inspect `build.json` before launch. Match the supplied checksum and version.
   Confirm the candidate’s documented runtime-library requirements against SteamOS.
   System Vulkan/OpenXR loaders and GPU drivers remain provided by SteamOS.
4. Start `./XRbox`. For isolated UI testing only, run
   `./XRbox -- --ui-preview`; synthetic labels must remain visible. Do not use
   `--desktop` for the headset XR acceptance run. Use the supported DevKit launch
   environment so the headset's OpenXR runtime is available.

## Gate A — native launch, stereo and presentation placement (10 minutes)

- Native extension loads without missing-library/architecture errors. Record the
  actual renderer and OpenXR runtime; an ordinary desktop window is not a pass.
- Both eyes show stable output; head tracking works. Recenter once, then turn and
  move the head: screen/menu remain level and world-locked, not head-locked.
- Width/distance adjustments stay within bounds; reset/recenter behave predictably.
- Remove/re-wear headset and open/close the system overlay. UI recovers without
  stale button activation. Quit/relaunch twice; no hang or orphan media work.

Stop and capture the visible error if native or XR startup fails.

## Gate B — GPU-native synthetic H.264 (before account access)

This gate requires a native-video candidate after its decoder/import adapter exists.

- Decode a known local H.264 test clip with the **named hardware decoder**.
  Record capture format, dimensions/stride/crop, native handle kind and modifier.
- Establish native retention from decoder through mailbox, GPU conversion/import
  and completion fence. Count retained/retired surfaces; bounds must hold through
  resize, reconnect and teardown. Never recycle a surface at mailbox replacement.
- Confirm all colors/gray ramps, aspect ratio and motion in both eyes. Test
  720p/1080p and format/resolution changes; no corruption, green frames or tearing.
- Establish actual import and presentation independently of decode counts. Record
  CPU transfers as observed/unknown, not zero by assumption. CPU pixel transfer
  fails the chosen native-video acceptance requirement.
- Verify ten minutes of paced playback with no accumulating queue, decoder
  starvation, firmware reset or renderer hang; then close/reopen twice.

If stock Godot fails a required import/synchronization capability, retain the
minimal reproduction and enabled-extension evidence before choosing an engine
patch. GPU extension advertisement alone is insufficient. Do not swap drivers,
change the kernel, or patch the headset OS as a test workaround.

## Gate C — shared UI and controller navigation (10 minutes)

- With Bluetooth Xbox controller: D-pad/A/B reach every action, focus is always
  visible and list scrolling reveals only the newly selected action.
- Header/version/footer stay fixed while the action list scrolls. Leave an item
  focused, idle and hover elsewhere: the list must not jump or recenter.
- Test home, device-code page, long errors/diagnostics, console pagination, screen
  settings, stream menu and sign-out confirmation. Test the last action, wraparound,
  repeated up/down and returning from nested pages.
- On Quest Android, repeat using **Meta Touch controls only**: either thumbstick
  navigates; right A/left X selects; right B/left Y goes back; stick click opens
  or closes the local menu. Hold input during focus loss/controller loss and
  return: release to neutral before actions resume. Meta/system menu stays reserved.
- XR menu actions never become console gameplay input. During a stream, the
  physical gamepad's B remains gameplay input; use its deferred menu shortcut.

## Gate D — auth, discovery and console connection (10 minutes)

Only after A–C pass on the native-video candidate:

- Default remembered sign-in uses the secure credential backend. If SteamOS has
  no usable credential service, show the error and permit explicit session-only
  sign-in. Never write refresh credentials into settings or token files.
- Use the supported Microsoft device-code flow on a phone/computer. Confirm the
  full code and correct verification URL are readable. Cancel/retry once.
- Select the intended discovered console (including beyond page one). Invalid or
  removed selections cannot start media. Busy UI offers cancel and stays responsive.
- Confirm packet receipt, assembled frames, hardware-decoded frames, GPU conversion
  and actual moving video separately. Check audible audio and real game input.
- Treat route classification carefully: private addresses do not prove direct LAN.
  Relay rejection must remain visible. Do not collect raw SDP/ICE/account IDs.

## Gate E — gameplay, recovery and secure cleanup (20 minutes)

- Test sticks, triggers, D-pad, face buttons and bumpers in an already installed
  game/dashboard. Deferred Select/Start singles fire only on release; their pair
  sends Guide; pair+LB+RB opens XRbox without stray singles or Guide. Physical
  Guide stays reserved to the host.
- Resume while holding a gameplay stick/button: gameplay resumes immediately;
  only Select/Start wait for release. Open menu with Meta controls while the
  physical gamepad is held: gameplay input neutralizes during the menu.
- Unplug/re-pair controller, disconnect/reconnect three times, lose XR focus and
  return, then quit during playback. No stale controls, continuing audio, blocked
  UI thread, leaked textures or unbounded retries.
- Confirm video/audio recovery after a recoverable stall, and visible timeout for
  missing decoded video. A network/signaling success is not playback success.
- Relaunch to test secure restoration/refresh rotation; sign out and confirm the
  remembered session is removed. Test session-only mode independently.

## Endurance and evidence

Run at least 30 minutes with fixed brightness, volume and headset refresh. Record
stream resolution/FPS, assembly/decode/presentation/drop counters, queue/resource
bounds, audio drops and visible stalls. Note power/thermals as observations;
benchmark latency/battery only with repeatable matched trials. Flatpak acceptance
is separate: compare the unpackaged native app before qualifying sandbox permissions.

For each case record **PASS / FAIL / NOT RUN**, expected/actual behavior and the
build checksum. Share only app-specific sanitized logs. Never include codes,
tokens, raw auth responses, SDP, ICE addresses, account IDs or unrelated device logs.
