use godot::{global::JoyButton, prelude::*};
use xcloud_proto::input::models::GamepadButtons as B;
pub fn button(index: i32) -> B {
    [
        (JoyButton::A, B::A),
        (JoyButton::B, B::B),
        (JoyButton::X, B::X),
        (JoyButton::Y, B::Y),
        (JoyButton::BACK, B::VIEW),
        (JoyButton::START, B::MENU),
        (JoyButton::LEFT_SHOULDER, B::LEFT_SHOULDER),
        (JoyButton::RIGHT_SHOULDER, B::RIGHT_SHOULDER),
        (JoyButton::LEFT_STICK, B::LEFT_THUMB),
        (JoyButton::RIGHT_STICK, B::RIGHT_THUMB),
        (JoyButton::DPAD_UP, B::DPAD_UP),
        (JoyButton::DPAD_DOWN, B::DPAD_DOWN),
        (JoyButton::DPAD_LEFT, B::DPAD_LEFT),
        (JoyButton::DPAD_RIGHT, B::DPAD_RIGHT),
    ]
    .into_iter()
    .find_map(|(key, value)| (key.ord() == index).then_some(value))
    .unwrap_or(B::NONE)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn godot_menu_pair_and_bumpers_match_shared_hotkeys_but_guide_is_reserved() {
        let bits = button(JoyButton::BACK.ord()) | button(JoyButton::START.ord());
        assert_eq!(bits, B::VIEW | B::MENU);
        let app =
            bits | button(JoyButton::LEFT_SHOULDER.ord()) | button(JoyButton::RIGHT_SHOULDER.ord());
        assert_eq!(
            app.bits(),
            xrbox_core::config::HotkeyBindings::default().menu
        );
        assert_eq!(button(JoyButton::GUIDE.ord()), B::NONE);
        assert_eq!(button(-1), B::NONE);
        assert_eq!(button(JoyButton::A.ord()), B::A);
        assert_eq!(button(JoyButton::DPAD_LEFT.ord()), B::DPAD_LEFT);
    }
}

/// Preserve a disconnect edge even if Godot reports a replacement pad before
/// the native 8 ms sampler runs. Otherwise an old deferred chord could complete
/// on the new controller. Ordinary held gameplay input resumes on the next tick.
#[derive(Default)]
pub struct Pad {
    latest: xcloud_input::source::NativeGamepadSample,
    lost: bool,
}
impl Pad {
    pub fn update(&mut self, sample: xcloud_input::source::NativeGamepadSample) {
        if !sample.connected {
            self.lost = true;
        }
        self.latest = sample;
    }
    pub fn sample(&mut self) -> xcloud_input::source::NativeGamepadSample {
        if self.lost {
            self.lost = false;
            Default::default()
        } else {
            self.latest.clone()
        }
    }
}
#[cfg(test)]
mod replacement_tests {
    use super::*;
    #[test]
    fn reconnect_does_not_overwrite_disconnect_and_unrelated_held_input_resumes() {
        let mut pad = Pad::default();
        pad.update(Default::default());
        let mut connected = xcloud_input::source::NativeGamepadSample {
            connected: true,
            ..Default::default()
        };
        connected.state.buttons = B::A | B::VIEW;
        pad.update(connected.clone());
        assert!(!pad.sample().connected);
        assert_eq!(pad.sample(), connected);
    }
}
