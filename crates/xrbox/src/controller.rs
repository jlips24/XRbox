//! One physical sampler; app shortcuts produce protocol input, never OS key events.
use parking_lot::Mutex;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use xcloud_input::source::{NativeGamepadSample, Sampler};
use xcloud_proto::input::{gamepad::RawGamepadState, models::GamepadButtons as B};

const GUIDE_CHORD: B = B::from_bits(B::VIEW.bits() | B::MENU.bits());
const APP_CHORD: B =
    B::from_bits(GUIDE_CHORD.bits() | B::LEFT_SHOULDER.bits() | B::RIGHT_SHOULDER.bits());
const GUIDE_PULSE: Duration = Duration::from_millis(120);

#[derive(Default)]
struct Shared {
    sample: NativeGamepadSample,
    routed: RawGamepadState,
    hotkeys: Hotkeys,
}
#[derive(Clone, Default)]
pub struct Controller {
    state: Arc<Mutex<Shared>>,
    menu: Arc<AtomicBool>,
}
impl Controller {
    pub fn sample(&self) -> NativeGamepadSample {
        self.state.lock().sample.clone()
    }
    pub fn menu(&self) -> bool {
        self.menu.load(Ordering::Relaxed)
    }
    pub fn set_menu(&self, open: bool) {
        self.menu.store(open, Ordering::Relaxed);
    }
    pub fn poll(
        &self,
        sampler: &mut Sampler,
        streaming: bool,
        focused: bool,
        now: Duration,
        ctx: &egui::Context,
    ) {
        let next = sampler();
        let mut shared = self.state.lock();
        let (routed, open) = shared.hotkeys.route(
            next.state,
            next.connected && streaming && focused,
            self.menu(),
            now,
        );
        if open {
            self.set_menu(true);
        }
        shared.routed = routed;
        let changed = shared.sample != next;
        shared.sample = next;
        // Gameplay input must not drive extra GPU/UI frames on top of video presentation.
        if (changed && (!streaming || self.menu())) || open {
            ctx.request_repaint();
        }
    }
    pub fn stream_sampler(&self) -> Sampler {
        let controller = self.clone();
        Box::new(move || {
            let shared = controller.state.lock();
            NativeGamepadSample {
                state: if controller.menu() {
                    RawGamepadState::default()
                } else {
                    shared.routed
                },
                ..shared.sample.clone()
            }
        })
    }
}

/// Resolve overlapping shortcuts before passing state to the protocol encoder.
/// The short chord commits on release, so adding LB/RB can take priority at any
/// point while Select+Start remain held. No delay is added to ordinary gameplay.
struct Hotkeys {
    candidate: bool,
    wait_release: bool,
    guide_until: Option<Duration>,
}
impl Default for Hotkeys {
    fn default() -> Self {
        Self {
            candidate: false,
            wait_release: true,
            guide_until: None,
        }
    }
}
impl Hotkeys {
    fn route(
        &mut self,
        mut state: RawGamepadState,
        eligible: bool,
        menu: bool,
        now: Duration,
    ) -> (RawGamepadState, bool) {
        if !eligible || menu {
            *self = Self::default();
            return (RawGamepadState::default(), false);
        }
        // The physical Guide belongs to the host OS. Only the virtual shortcut
        // below may insert NEXUS into the console's input stream.
        state.buttons.remove(B::NEXUS);
        if self.wait_release {
            if (state.buttons & GUIDE_CHORD).is_empty() {
                self.wait_release = false;
            } else {
                state.buttons.remove(GUIDE_CHORD);
            }
        } else if state.buttons.contains(APP_CHORD) {
            *self = Self::default();
            return (RawGamepadState::default(), true);
        } else if state.buttons.contains(GUIDE_CHORD) {
            self.candidate = true;
            state.buttons.remove(GUIDE_CHORD);
        } else if self.candidate {
            self.candidate = false;
            self.wait_release = !(state.buttons & GUIDE_CHORD).is_empty();
            self.guide_until = Some(now + GUIDE_PULSE);
            state.buttons.remove(GUIDE_CHORD);
        }
        if self.guide_until.is_some_and(|until| now < until) {
            state.buttons.insert(B::NEXUS);
        } else {
            self.guide_until = None;
        }
        (state, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn raw(buttons: B) -> RawGamepadState {
        RawGamepadState {
            buttons,
            ..Default::default()
        }
    }
    fn armed() -> Hotkeys {
        let mut keys = Hotkeys::default();
        keys.route(Default::default(), true, false, Duration::ZERO);
        keys
    }
    #[test]
    fn guide_chord_commits_once_on_release_with_bounded_press_and_release() {
        let mut keys = armed();
        assert_eq!(
            keys.route(raw(GUIDE_CHORD), true, false, Duration::ZERO),
            (raw(B::NONE), false)
        );
        assert_eq!(
            keys.route(raw(GUIDE_CHORD), true, false, Duration::from_secs(5)),
            (raw(B::NONE), false)
        );
        assert_eq!(
            keys.route(raw(B::VIEW), true, false, Duration::from_secs(5)),
            (raw(B::NEXUS), false)
        );
        assert_eq!(
            keys.route(raw(B::VIEW), true, false, Duration::from_millis(5119))
                .0
                .buttons,
            B::NEXUS
        );
        assert!(
            keys.route(raw(B::VIEW), true, false, Duration::from_millis(5120))
                .0
                .buttons
                .is_empty()
        );
        assert!(
            keys.route(raw(B::NONE), true, false, Duration::from_secs(6))
                .0
                .buttons
                .is_empty()
        );
        keys.route(raw(GUIDE_CHORD), true, false, Duration::from_secs(7));
        assert_eq!(
            keys.route(raw(B::NONE), true, false, Duration::from_secs(7))
                .0
                .buttons,
            B::NEXUS
        );
    }
    #[test]
    fn larger_chord_wins_in_every_button_press_order_and_never_emits_guide() {
        let buttons = [B::VIEW, B::MENU, B::LEFT_SHOULDER, B::RIGHT_SHOULDER];
        for a in 0..4 {
            for b in 0..4 {
                for c in 0..4 {
                    for d in 0..4 {
                        let order = [a, b, c, d];
                        if (0..4).any(|i| (i + 1..4).any(|j| order[i] == order[j])) {
                            continue;
                        }
                        let mut keys = armed();
                        let mut held = B::NONE;
                        for (i, index) in order.into_iter().enumerate() {
                            held.insert(buttons[index]);
                            // Even a slow addition of bumpers must not fire the smaller chord.
                            let (output, open) =
                                keys.route(raw(held), true, false, Duration::from_secs(i as u64));
                            assert!(!output.buttons.contains(B::NEXUS));
                            assert_eq!(open, i == 3);
                        }
                        // Closing while held must neither reopen nor emit Guide.
                        for _ in 0..3 {
                            assert!(
                                !keys
                                    .route(raw(APP_CHORD), true, false, Duration::from_secs(5))
                                    .1
                            );
                        }
                        assert!(
                            !keys
                                .route(raw(B::NONE), true, false, Duration::from_secs(6))
                                .0
                                .buttons
                                .contains(B::NEXUS)
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn only_select_and_start_wait_for_release_after_resuming() {
        let mut keys = armed();
        let active = RawGamepadState {
            buttons: B::A | APP_CHORD,
            left_thumb: xcloud_proto::input::models::Vec2::new(0.8, 0.4),
            left_trigger: 0.7,
            ..Default::default()
        };
        assert_eq!(
            keys.route(active, true, true, Duration::ZERO).0,
            RawGamepadState::default()
        );
        let resumed = keys.route(active, true, false, Duration::ZERO).0;
        assert_eq!(resumed.buttons, B::A | B::LEFT_SHOULDER | B::RIGHT_SHOULDER);
        assert_eq!(resumed.left_thumb, active.left_thumb);
        assert_eq!(resumed.left_trigger, active.left_trigger);
        assert_eq!(
            keys.route(raw(B::A | B::VIEW), true, false, Duration::ZERO)
                .0
                .buttons,
            B::A
        );
        keys.route(raw(B::A), true, false, Duration::ZERO);
        assert_eq!(
            keys.route(raw(B::A | B::VIEW), true, false, Duration::ZERO)
                .0
                .buttons,
            B::A | B::VIEW
        );
    }
    #[test]
    fn focus_loss_disconnect_and_open_menu_cancel_pending_and_active_guide() {
        for menu in [false, true] {
            for release_first in [false, true] {
                let mut keys = armed();
                keys.route(raw(GUIDE_CHORD), true, false, Duration::ZERO);
                if release_first {
                    keys.route(raw(B::NONE), true, false, Duration::ZERO);
                }
                assert_eq!(
                    keys.route(raw(GUIDE_CHORD), menu, menu, Duration::ZERO),
                    (raw(B::NONE), false)
                );
                // Regaining focus with held hotkeys cannot replay the interrupted action.
                assert_eq!(
                    keys.route(raw(GUIDE_CHORD), true, false, Duration::from_secs(1)),
                    (raw(B::NONE), false)
                );
                assert_eq!(
                    keys.route(raw(B::NONE), true, false, Duration::from_secs(1)),
                    (raw(B::NONE), false)
                );
            }
        }
    }
    #[test]
    fn normal_inputs_are_immediate_and_physical_guide_is_not_forwarded() {
        let mut keys = armed();
        for button in [
            B::VIEW,
            B::MENU,
            B::A,
            B::B,
            B::LEFT_SHOULDER,
            B::RIGHT_SHOULDER,
        ] {
            assert_eq!(
                keys.route(raw(button), true, false, Duration::ZERO),
                (raw(button), false)
            );
        }
        assert_eq!(
            keys.route(raw(B::NEXUS | B::A), true, false, Duration::ZERO),
            (raw(B::A), false)
        );
    }
    #[test]
    fn shared_controller_routes_virtual_guide_through_real_protocol_input_source() {
        let controller = Controller::default();
        let queue = Arc::new(Mutex::new(xcloud_proto::input::queue::InputQueue::default()));
        let mut source = xcloud_input::source::GamepadInputSource::new(controller.stream_sampler());
        source.set_input_queue(Some(queue.clone()));
        source.set_active(true);
        let ctx = egui::Context::default();
        let mut packets = Vec::new();
        for (buttons, time) in [
            (B::NONE, 0),
            (GUIDE_CHORD, 8),
            (B::NONE, 16),
            (B::NONE, 144),
        ] {
            let mut sampler: Sampler = Box::new(move || NativeGamepadSample {
                connected: true,
                state: raw(buttons),
                ..Default::default()
            });
            controller.poll(&mut sampler, true, true, Duration::from_millis(time), &ctx);
            source.tick(time);
            let packet = queue.lock().flush(time).expect("input packet");
            assert_eq!(packet[14], 1); // one encoded controller
            packets.push(B::from_bits(u16::from_le_bytes([packet[16], packet[17]])));
        }
        // Verify the wire representation, including the subsequent release.
        assert_eq!(packets, vec![B::NONE, B::NONE, B::NEXUS, B::NONE]);
    }
    #[test]
    fn shared_router_opens_only_app_menu_and_ignores_idle_unfocused_or_disconnected_chords() {
        for (streaming, focused, connected) in [
            (true, true, true),
            (false, true, true),
            (true, false, true),
            (true, true, false),
        ] {
            let controller = Controller::default();
            let ctx = egui::Context::default();
            for buttons in [B::NONE, APP_CHORD] {
                let mut sampler: Sampler = Box::new(move || NativeGamepadSample {
                    connected,
                    state: raw(buttons),
                    ..Default::default()
                });
                controller.poll(&mut sampler, streaming, focused, Duration::ZERO, &ctx);
            }
            assert_eq!(controller.menu(), streaming && focused && connected);
            let mut output = controller.stream_sampler();
            assert_eq!(output().state, RawGamepadState::default());
        }
    }
}
