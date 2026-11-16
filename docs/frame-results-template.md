# Frame test results — November 18, 2026

Copy this file before filling it out. Use PASS / FAIL / NOT RUN, with the actual
artifact SHA256. Do not treat a missing implementation as a successful test.

| Build/environment | Observation |
| --- | --- |
| Bundle SHA256 / build.json revision | |
| App / core / Godot version | |
| Frame SteamOS build / kernel / glibc | |
| OpenXR runtime / actual renderer | |
| Physical Xbox controller model / connection | |
| Advertised codec / native-buffer format / modifier | |
| Actual selected decoder / output type | |
| Import route / resource bounds / completion synchronization | |

| ID | Test | Status | Actual result / sanitized evidence |
| --- | --- | --- | --- |
| A1 | Native ARM64 launch; all libraries resolve | NOT RUN | |
| A2 | Stereo/head tracking on headset | NOT RUN | |
| A3 | World-locked screen, level recenter, geometry | NOT RUN | |
| A4 | System overlay, removal/return, quit/relaunch | NOT RUN | |
| B1 | Named hardware H.264 decoder, native output | NOT RUN | |
| B2 | Retained native buffers → GPU conversion/import → fence completion | NOT RUN | |
| B3 | Moving local video, color/crop/720p/1080p | NOT RUN | |
| B4 | Bounded surfaces/queues, resize/reconnect/teardown | NOT RUN | |
| C1 | Every page reachable using D-pad/A/B | NOT RUN | |
| C2 | Scrolling/focus/idle stability/fixed footer | NOT RUN | |
| C3 | Meta thumbstick/A-X/B-Y/stick-click navigation on Android | NOT RUN | |
| C4 | Held controls at focus loss/disconnect do not activate on return | NOT RUN | |
| D1 | Remembered secure sign-in or visible storage error/session-only choice | NOT RUN | |
| D2 | Device code/cancel/retry/console pagination | NOT RUN | |
| D3 | Receipt / assembly / decode / GPU presentation verified separately | NOT RUN | |
| D4 | Audible audio and actual console input | NOT RUN | |
| E1 | Deferred singles / Guide / XRbox shortcut priority | NOT RUN | |
| E2 | Resume held input, gamepad disconnect/reconnect | NOT RUN | |
| E3 | Three reconnects, XR focus loss, quit while streaming | NOT RUN | |
| E4 | Secure restoration, refresh rotation, sign-out cleanup | NOT RUN | |
| F1 | 30-minute stable session; counters/resource bounds | NOT RUN | |

## First blocking failure

- Case ID and exact trigger:
- Expected behavior / actual behavior:
- Visible error and relevant sanitized app-only log excerpt:
- Remaining cases not run because of this failure:
- Decision for the next build and required retest:

Do not include device codes, tokens, raw auth replies, SDP, ICE/network addresses,
account IDs or unrelated device logs. GPU capability advertisement is not import,
presentation, efficient playback or battery evidence.
