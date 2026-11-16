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
#[path = "controller_tests.rs"]
mod tests;
