# Roadmap and acceptance gates

1. **Desktop console proof of concept (current):** native Rust sign-in, console
   discovery, home negotiation, native video/audio, controller input, redacted
   stages and clean disconnect. Validate against a real Xbox; synthetic tests
   alone do not satisfy this gate. Improve selected-route and performance
   instrumentation using console-test results.
2. **Console reliability:** address observed auth, provisioning, input, audio,
   pacing and reconnect failures. Confirm the selected LAN route. Expand
   diagnostics and packaging tests. Consider PSBox protocol reuse only if
   OpenXCloud integration proves materially unsuitable.
3. **Frame feasibility:** native Linux ARM64 package in Valve's runtime; inspect
   hardware decoder access, formats/modifiers, GPU synchronization/import,
   credential facilities, audio/controller access and system virtual screen.
4. **Efficient Frame playback:** extend native frame and decoder integration to
   keep surfaces on GPU where possible. Measure matched unplugged baseline,
   local video and Xbox streaming trials, including temperature and power.
   Decide settings from measurements rather than desktop results.
5. **Custom XR only if needed:** minimal OpenXR quad, configurable position/size,
   then optional passthrough if runtime extensions permit. Compare power and
   latency with the system screen before selecting the permanent presenter.

No cloud gaming, relay PC, social features, catalogs, motion-controller gameplay,
complex environments, frame generation or upscaling in the first milestone.
