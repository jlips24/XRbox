//! One physical sampler shared by the app menu and streaming input.
use parking_lot::Mutex;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use xcloud_input::source::{NativeGamepadSample, Sampler};
use xcloud_proto::input::{gamepad::RawGamepadState, models::GamepadButtons as B};

#[derive(Clone, Default)]
pub struct Controller {
    sample: Arc<Mutex<NativeGamepadSample>>,
    menu: Arc<AtomicBool>,
}
impl Controller {
    pub fn sample(&self) -> NativeGamepadSample {
        self.sample.lock().clone()
    }
    pub fn menu(&self) -> bool {
        self.menu.load(Ordering::Relaxed)
    }
    pub fn set_menu(&self, open: bool) {
        self.menu.store(open, Ordering::Relaxed);
    }
    pub fn poll(&self, sampler: &mut Sampler, streaming: bool, focused: bool, ctx: &egui::Context) {
        let next = sampler();
        let mut previous = self.sample.lock();
        let chord = B::VIEW | B::MENU;
        if streaming
            && focused
            && next.state.buttons.contains(chord)
            && !previous.state.buttons.contains(chord)
        {
            self.set_menu(true);
        }
        if *previous != next {
            *previous = next;
            // Gameplay input must not drive extra GPU/UI frames on top of video presentation.
            if !streaming || self.menu() {
                ctx.request_repaint();
            }
        }
    }
    pub fn stream_sampler(&self) -> Sampler {
        let controller = self.clone();
        // Only the reserved hotkey stays suppressed after resuming; gameplay is immediate.
        let mut gate = ReleaseGate(true);
        Box::new(move || {
            let mut sample = controller.sample();
            sample.state = gate.filter(sample.state, controller.menu());
            sample
        })
    }
}

struct ReleaseGate(bool);
impl ReleaseGate {
    fn filter(&mut self, mut state: RawGamepadState, menu: bool) -> RawGamepadState {
        let hotkey = B::VIEW | B::MENU;
        if menu {
            self.0 = true;
            return RawGamepadState::default();
        }
        if self.0 {
            if (state.buttons & hotkey).is_empty() {
                self.0 = false;
            }
            state.buttons.remove(hotkey);
        }
        state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_view_and_menu_wait_for_release_after_resuming() {
        let mut gate = ReleaseGate(false);
        let active = RawGamepadState {
            buttons: B::A | B::VIEW | B::MENU,
            left_thumb: xcloud_proto::input::models::Vec2::new(0.8, 0.4),
            left_trigger: 0.7,
            ..Default::default()
        };
        assert_eq!(gate.filter(active, true), RawGamepadState::default());
        let resumed = gate.filter(active, false);
        assert_eq!(resumed.buttons, B::A);
        assert_eq!(resumed.left_thumb, active.left_thumb);
        assert_eq!(resumed.left_trigger, active.left_trigger);
        let only_view = RawGamepadState {
            buttons: B::A | B::VIEW,
            ..active
        };
        assert_eq!(gate.filter(only_view, false).buttons, B::A);
        // Both hotkey buttons must have been released before either can pass again.
        gate.filter(
            RawGamepadState {
                buttons: B::A,
                ..active
            },
            false,
        );
        assert_eq!(gate.filter(only_view, false).buttons, B::A | B::VIEW);
    }
    #[test]
    fn menu_chord_is_edge_triggered_and_requires_focus() {
        let controller = Controller::default();
        let mut sampler: Sampler = Box::new(|| NativeGamepadSample {
            connected: true,
            state: RawGamepadState {
                buttons: B::VIEW | B::MENU,
                ..Default::default()
            },
            ..Default::default()
        });
        let ctx = egui::Context::default();
        controller.poll(&mut sampler, true, true, &ctx);
        assert!(controller.menu());
        controller.set_menu(false);
        controller.poll(&mut sampler, true, true, &ctx);
        assert!(!controller.menu());
        let other = Controller::default();
        other.poll(&mut sampler, true, false, &ctx);
        assert!(!other.menu());
    }
}
