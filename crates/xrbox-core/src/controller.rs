//! One physical sampler; app shortcuts produce protocol input, never OS key events.
use parking_lot::Mutex;
use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use xcloud_input::source::{NativeGamepadSample, Sampler};
use xcloud_proto::input::{gamepad::RawGamepadState, models::GamepadButtons as B};

const GUIDE_CHORD: B = B::from_bits(B::VIEW.bits() | B::MENU.bits());
#[cfg(test)]
const APP_CHORD: B =
    B::from_bits(GUIDE_CHORD.bits() | B::LEFT_SHOULDER.bits() | B::RIGHT_SHOULDER.bits());
const SHORTCUT_PULSE: Duration = Duration::from_millis(120);

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
    pub fn set_bindings(&self, bindings: crate::config::HotkeyBindings) -> anyhow::Result<()> {
        bindings.validate()?;
        let mut shared = self.state.lock();
        shared.hotkeys = Hotkeys {
            bindings,
            ..Default::default()
        };
        shared.routed = RawGamepadState::default();
        Ok(())
    }
    pub fn poll(
        &self,
        sampler: &mut Sampler,
        streaming: bool,
        focused: bool,
        now: Duration,
        ctx: &crate::Notify,
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
            ctx.notify();
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

// More specific bindings come last. Singles and combinations use the same recognizer.
#[derive(Clone, Copy)]
enum Action {
    Press(B),
    Menu,
}
fn bindings(settings: crate::config::HotkeyBindings) -> [(B, Action); 4] {
    [
        (B::VIEW, Action::Press(B::VIEW)),
        (B::MENU, Action::Press(B::MENU)),
        (B::from_bits(settings.guide), Action::Press(B::NEXUS)),
        (B::from_bits(settings.menu), Action::Menu),
    ]
}
const MAX_PENDING_PRESSES: usize = 16;
const RELEASE_GAP: Duration = Duration::from_millis(8);
struct Pending {
    first: B,
    binding: usize,
}

/// Buffer Select/Start until the initiating button is released. Remember the most
/// specific complete binding seen while it was held, even if other buttons release
/// first. All other gameplay inputs pass through without buffering.
struct Hotkeys {
    bindings: crate::config::HotkeyBindings,
    pending: Option<Pending>,
    wait_release: bool,
    presses: VecDeque<B>,
    pulse: Option<(B, Duration)>,
    next_press_at: Duration,
}
impl Default for Hotkeys {
    fn default() -> Self {
        Self {
            bindings: Default::default(),
            pending: None,
            wait_release: true,
            presses: VecDeque::new(),
            pulse: None,
            next_press_at: Duration::ZERO,
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
            *self = Self {
                bindings: self.bindings,
                ..Default::default()
            };
            return (RawGamepadState::default(), false);
        }
        let held = state.buttons;
        // Never forward the physical Guide or raw Select/Start presses.
        state.buttons.remove(B::NEXUS | GUIDE_CHORD);
        if self.wait_release {
            if (held & GUIDE_CHORD).is_empty() {
                self.wait_release = false;
            }
        } else {
            let bindings = bindings(self.bindings);
            let matching = bindings
                .iter()
                .enumerate()
                .filter(|(_, (buttons, _))| held.contains(*buttons))
                .max_by_key(|(index, (buttons, _))| (buttons.bits().count_ones(), *index))
                .map(|(index, _)| index);
            if self.pending.is_none() && !(held & GUIDE_CHORD).is_empty() {
                // If both arrive in one sample, Select is the deterministic first button.
                self.pending = Some(Pending {
                    first: if held.contains(B::VIEW) {
                        B::VIEW
                    } else {
                        B::MENU
                    },
                    binding: matching.expect("single-button binding"),
                });
            }
            if let Some(pending) = &mut self.pending {
                if held.contains(pending.first) {
                    if let Some(binding) = matching {
                        let score = |i: usize| (bindings[i].0.bits().count_ones(), i);
                        if score(binding) > score(pending.binding) {
                            pending.binding = binding;
                        }
                    }
                } else {
                    let action = bindings[pending.binding].1;
                    self.pending = None;
                    self.wait_release = !(held & GUIDE_CHORD).is_empty();
                    match action {
                        Action::Menu => {
                            *self = Self {
                                bindings: self.bindings,
                                ..Default::default()
                            };
                            return (RawGamepadState::default(), true);
                        }
                        Action::Press(button) => {
                            if self.presses.len() < MAX_PENDING_PRESSES {
                                self.presses.push_back(button);
                            } else {
                                tracing::warn!(target: "xrbox", "Controller shortcut queue full; action ignored");
                            }
                        }
                    }
                }
            }
        }
        if self.pulse.is_some_and(|(_, until)| now >= until) {
            self.pulse = None;
            // A real release sample separates repeated presses of the same button.
            self.next_press_at = now + RELEASE_GAP;
        }
        if self.pulse.is_none()
            && now >= self.next_press_at
            && let Some(button) = self.presses.pop_front()
        {
            self.pulse = Some((button, now + SHORTCUT_PULSE));
        }
        if let Some((button, _)) = self.pulse {
            state.buttons.insert(button);
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
    fn step(keys: &mut Hotkeys, buttons: B, ms: u64) -> (RawGamepadState, bool) {
        keys.route(raw(buttons), true, false, Duration::from_millis(ms))
    }
    #[test]
    fn individual_select_and_start_are_deferred_and_press_once_on_release() {
        for button in [B::VIEW, B::MENU] {
            let mut keys = armed();
            assert_eq!(step(&mut keys, button, 0), (raw(B::NONE), false));
            assert_eq!(step(&mut keys, button, 5000), (raw(B::NONE), false));
            assert_eq!(step(&mut keys, B::NONE, 5008), (raw(button), false));
            assert_eq!(step(&mut keys, B::NONE, 5127).0.buttons, button);
            assert_eq!(step(&mut keys, B::NONE, 5128).0.buttons, B::NONE);
            assert_eq!(step(&mut keys, B::NONE, 6000).0.buttons, B::NONE);
        }
    }
    #[test]
    fn guide_uses_first_button_release_for_both_press_and_release_orders() {
        for first in [B::VIEW, B::MENU] {
            let other = if first == B::VIEW { B::MENU } else { B::VIEW };
            for release_first in [true, false] {
                let mut keys = armed();
                assert!(step(&mut keys, first, 0).0.buttons.is_empty());
                assert!(step(&mut keys, GUIDE_CHORD, 8).0.buttons.is_empty());
                if !release_first {
                    assert!(step(&mut keys, first, 16).0.buttons.is_empty());
                }
                let held = if release_first { other } else { B::NONE };
                assert_eq!(step(&mut keys, held, 24), (raw(B::NEXUS), false));
                assert_eq!(step(&mut keys, held, 144), (raw(B::NONE), false));
                assert_eq!(step(&mut keys, B::NONE, 152), (raw(B::NONE), false));
            }
        }
    }
    fn orders() -> Vec<[usize; 4]> {
        let mut result = Vec::new();
        for a in 0..4 {
            for b in 0..4 {
                for c in 0..4 {
                    for d in 0..4 {
                        let order = [a, b, c, d];
                        if !(0..4).any(|i| (i + 1..4).any(|j| order[i] == order[j])) {
                            result.push(order);
                        }
                    }
                }
            }
        }
        result
    }
    #[test]
    fn app_menu_wins_all_24_press_by_24_release_orders_without_individual_or_guide_presses() {
        let buttons = [B::VIEW, B::MENU, B::LEFT_SHOULDER, B::RIGHT_SHOULDER];
        for press in orders() {
            for release in orders() {
                let mut keys = armed();
                let mut held = B::NONE;
                let first = buttons[*press.iter().find(|&&i| i < 2).unwrap()];
                for (i, index) in press.into_iter().enumerate() {
                    held.insert(buttons[index]);
                    let (output, open) = step(&mut keys, held, i as u64 * 1000);
                    assert!((output.buttons & (GUIDE_CHORD | B::NEXUS)).is_empty());
                    assert!(!open);
                }
                let mut menu = false;
                let mut opens = 0;
                for (i, index) in release.into_iter().enumerate() {
                    held.remove(buttons[index]);
                    let (output, open) = keys.route(
                        raw(held),
                        true,
                        menu,
                        Duration::from_millis(4000 + i as u64 * 1000),
                    );
                    assert_eq!(open, !menu && buttons[index] == first);
                    assert!((output.buttons & (GUIDE_CHORD | B::NEXUS)).is_empty());
                    if open {
                        menu = true;
                        opens += 1;
                    }
                }
                assert_eq!(opens, 1);
                assert_eq!(step(&mut keys, B::NONE, 8000), (raw(B::NONE), false));
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
        assert_eq!(step(&mut keys, B::A | B::VIEW, 0).0.buttons, B::A);
        step(&mut keys, B::A, 8);
        assert_eq!(step(&mut keys, B::A | B::VIEW, 16).0.buttons, B::A);
        assert_eq!(step(&mut keys, B::A, 24).0.buttons, B::A | B::VIEW);
    }
    #[test]
    fn focus_loss_disconnect_idle_and_open_menu_cancel_pending_and_queued_presses() {
        for buttons in [B::VIEW, B::MENU, GUIDE_CHORD, APP_CHORD] {
            for menu in [false, true] {
                for release in [false, true] {
                    let mut keys = armed();
                    step(&mut keys, buttons, 0);
                    if release {
                        step(&mut keys, B::NONE, 8);
                    }
                    assert_eq!(
                        keys.route(raw(buttons), menu, menu, Duration::from_millis(16)),
                        (raw(B::NONE), false)
                    );
                    // Focus return suppresses only Select/Start, not held bumpers.
                    let passthrough = buttons & (B::LEFT_SHOULDER | B::RIGHT_SHOULDER);
                    assert_eq!(step(&mut keys, buttons, 24), (raw(passthrough), false));
                    assert_eq!(step(&mut keys, B::NONE, 32), (raw(B::NONE), false));
                }
            }
        }
    }
    #[test]
    fn unrelated_inputs_are_immediate_and_physical_guide_is_not_forwarded() {
        let mut keys = armed();
        for button in [B::A, B::B, B::LEFT_SHOULDER, B::RIGHT_SHOULDER] {
            assert_eq!(step(&mut keys, button, 0), (raw(button), false));
            let mut state = raw(button | B::VIEW);
            state.left_trigger = 0.8;
            let output = keys.route(state, true, false, Duration::ZERO).0;
            assert_eq!(output.buttons, button);
            assert_eq!(output.left_trigger, 0.8);
            keys = armed();
        }
        assert_eq!(step(&mut keys, B::NEXUS | B::A, 0), (raw(B::A), false));
    }
    #[test]
    fn rapid_taps_are_separated_by_release_and_different_single_does_not_upgrade_pending() {
        let mut keys = armed();
        step(&mut keys, B::VIEW, 0);
        assert_eq!(step(&mut keys, B::MENU, 8).0.buttons, B::VIEW);
        // Start arriving after Select releases was never part of the gesture.
        step(&mut keys, B::NONE, 16);
        step(&mut keys, B::VIEW, 24);
        step(&mut keys, B::NONE, 32);
        assert_eq!(step(&mut keys, B::NONE, 128).0.buttons, B::NONE);
        assert_eq!(step(&mut keys, B::NONE, 136).0.buttons, B::VIEW);
        assert_eq!(step(&mut keys, B::NONE, 256).0.buttons, B::NONE);
        assert_eq!(step(&mut keys, B::NONE, 264).0.buttons, B::NONE);
    }
    #[test]
    fn pending_press_queue_is_bounded() {
        let mut keys = armed();
        for _ in 0..100 {
            step(&mut keys, B::VIEW, 0);
            step(&mut keys, B::NONE, 0);
        }
        assert_eq!(keys.presses.len(), MAX_PENDING_PRESSES);
    }
    #[test]
    fn shared_router_encodes_deferred_singles_guide_and_app_menu_in_real_protocol_packets() {
        for first in [B::VIEW, B::MENU] {
            for chord in [first, GUIDE_CHORD, APP_CHORD] {
                let controller = Controller::default();
                let queue = Arc::new(Mutex::new(xcloud_proto::input::queue::InputQueue::default()));
                let mut source =
                    xcloud_input::source::GamepadInputSource::new(controller.stream_sampler());
                source.set_input_queue(Some(queue.clone()));
                source.set_active(true);
                let ctx = crate::Notify::default();
                let mut packets = Vec::new();
                for (buttons, time) in [
                    (B::NONE, 0),
                    (first, 8),
                    (chord, 16),
                    (chord, 24),
                    (B::NONE, 32),
                    (B::NONE, 160),
                ] {
                    let mut sampler: Sampler = Box::new(move || NativeGamepadSample {
                        connected: true,
                        state: raw(buttons),
                        ..Default::default()
                    });
                    controller.poll(&mut sampler, true, true, Duration::from_millis(time), &ctx);
                    source.tick(time);
                    let packet = queue.lock().flush(time).expect("input packet");
                    assert_eq!(packet[14], 1);
                    packets.push(B::from_bits(u16::from_le_bytes([packet[16], packet[17]])));
                }
                let expected = if chord == APP_CHORD {
                    B::NONE
                } else if chord == GUIDE_CHORD {
                    B::NEXUS
                } else {
                    first
                };
                // Bumpers remain ordinary live gameplay input while constructing the gesture.
                let passthrough = chord & (B::LEFT_SHOULDER | B::RIGHT_SHOULDER);
                assert_eq!(
                    packets,
                    vec![
                        B::NONE,
                        B::NONE,
                        passthrough,
                        passthrough,
                        expected,
                        B::NONE
                    ]
                );
                assert_eq!(controller.menu(), chord == APP_CHORD);
            }
        }
    }
    #[test]
    fn shared_router_ignores_idle_unfocused_or_disconnected_shortcuts() {
        for (streaming, focused, connected) in [
            (false, true, true),
            (true, false, true),
            (true, true, false),
        ] {
            let controller = Controller::default();
            let ctx = crate::Notify::default();
            for buttons in [B::NONE, APP_CHORD, B::NONE] {
                let mut sampler: Sampler = Box::new(move || NativeGamepadSample {
                    connected,
                    state: raw(buttons),
                    ..Default::default()
                });
                controller.poll(&mut sampler, streaming, focused, Duration::ZERO, &ctx);
            }
            assert!(!controller.menu());
            assert_eq!(
                controller.stream_sampler()().state,
                RawGamepadState::default()
            );
        }
    }
    #[test]
    fn focus_loss_drops_queued_taps_without_replaying_on_return() {
        let mut keys = armed();
        step(&mut keys, B::VIEW, 0);
        step(&mut keys, B::NONE, 8);
        step(&mut keys, B::MENU, 16);
        step(&mut keys, B::NONE, 24);
        assert_eq!(keys.presses.len(), 1);
        assert_eq!(
            keys.route(raw(B::NONE), false, false, Duration::from_millis(32)),
            (raw(B::NONE), false)
        );
        assert!(keys.presses.is_empty());
        assert_eq!(step(&mut keys, B::NONE, 1000), (raw(B::NONE), false));
    }
    #[test]
    fn custom_bindings_use_specificity_then_menu_priority_and_survive_cancellation() {
        let bindings = crate::config::HotkeyBindings {
            guide: (B::VIEW | B::X | B::LEFT_SHOULDER).bits(),
            menu: (B::MENU | B::Y).bits(),
        };
        let mut keys = Hotkeys {
            bindings,
            ..Default::default()
        };
        step(&mut keys, B::NONE, 0);
        let chord = B::VIEW | B::X | B::LEFT_SHOULDER | B::MENU | B::Y;
        step(&mut keys, chord, 8);
        assert_eq!(step(&mut keys, B::NONE, 16), (raw(B::NEXUS), false));
        keys.route(raw(B::NONE), false, false, Duration::from_millis(24));
        assert_eq!(keys.bindings, bindings);
        step(&mut keys, B::NONE, 32);
        step(&mut keys, B::MENU | B::Y, 40);
        assert!(step(&mut keys, B::NONE, 48).1);
        assert_eq!(keys.bindings, bindings);
        keys = Hotkeys {
            bindings: crate::config::HotkeyBindings {
                guide: (B::VIEW | B::X).bits(),
                menu: (B::MENU | B::Y).bits(),
            },
            ..Default::default()
        };
        step(&mut keys, B::NONE, 0);
        step(&mut keys, B::VIEW | B::X | B::MENU | B::Y, 8);
        assert!(step(&mut keys, B::NONE, 16).1);
    }
    #[test]
    fn changing_bindings_cancels_pending_input_and_invalid_bindings_leave_current_mapping() {
        let controller = Controller::default();
        let ctx = crate::Notify::default();
        let poll = |buttons: B, time| {
            let mut sampler: Sampler = Box::new(move || NativeGamepadSample {
                connected: true,
                state: raw(buttons),
                ..Default::default()
            });
            controller.poll(&mut sampler, true, true, Duration::from_millis(time), &ctx);
        };
        poll(B::NONE, 0);
        poll(GUIDE_CHORD, 8);
        let bindings = crate::config::HotkeyBindings {
            guide: (B::VIEW | B::X).bits(),
            menu: (B::MENU | B::Y).bits(),
        };
        controller.set_bindings(bindings).unwrap();
        poll(B::NONE, 16);
        assert_eq!(controller.stream_sampler()().state, raw(B::NONE));
        assert!(
            controller
                .set_bindings(crate::config::HotkeyBindings {
                    guide: 0,
                    ..bindings
                })
                .is_err()
        );
        poll(B::MENU | B::Y, 24);
        poll(B::NONE, 32);
        assert!(controller.menu());
    }
}
