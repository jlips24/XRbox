use crate::input;
use godot::{classes::IRefCounted, prelude::*};
use parking_lot::Mutex;
use std::sync::{Arc, atomic::Ordering};
use xcloud_input::source::NativeGamepadSample;
use xrbox_core::session::{Command, Session};
#[derive(GodotClass)]
#[class(base=RefCounted)]
struct XrboxRuntime {
    base: Base<RefCounted>,
    session: Option<Session>,
    pad: Arc<Mutex<input::Pad>>,
    video: crate::video::Video,
}
#[godot_api]
impl IRefCounted for XrboxRuntime {
    fn init(base: Base<RefCounted>) -> Self {
        Self {
            base,
            session: None,
            pad: Default::default(),
            video: Default::default(),
        }
    }
}
#[godot_api]
impl XrboxRuntime {
    #[func]
    fn core_version(&self) -> GString {
        xrbox_core::VERSION.into()
    }
    #[func]
    fn native_target(&self) -> GString {
        GString::from(&format!(
            "{} / {}",
            std::env::consts::OS,
            std::env::consts::ARCH
        ))
    }
    #[func]
    fn streaming_supported(&self) -> bool {
        cfg!(target_os = "android")
    }
    #[func]
    fn start_client(&mut self, restore: bool) -> bool {
        #[cfg(target_os = "android")]
        if !xrbox_core::android::ready() {
            return false;
        }
        if !cfg!(target_os = "android") {
            return false;
        }
        if self.session.is_none() {
            let pad = self.pad.clone();
            let s = Session::with_sampler(
                xrbox_core::Notify::default(),
                Box::new(move || pad.lock().sample()),
            );
            s.controller.set_menu(true);
            if restore {
                s.send(Command::Restore);
            }
            self.session = Some(s);
        }
        true
    }
    #[func]
    fn sign_in(&self, persistent: bool) {
        if let Some(s) = &self.session {
            s.send(Command::SignIn(persistent));
        }
    }
    #[func]
    fn discover(&self, persistent: bool) {
        if let Some(s) = &self.session {
            s.send(Command::Discover(persistent));
        }
    }
    #[func]
    fn sign_out(&self) {
        if let Some(s) = &self.session {
            s.send(Command::SignOut);
        }
    }
    #[func]
    fn disconnect_stream(&self) {
        if let Some(s) = &self.session {
            s.send(Command::Disconnect);
        }
    }
    #[func]
    fn connect_console(&self, index: i64) -> bool {
        let Some(s) = &self.session else { return false };
        let state = s.state.lock();
        if state.busy || state.active {
            return false;
        }
        let Some(console) = usize::try_from(index)
            .ok()
            .and_then(|i| state.consoles.get(i))
        else {
            return false;
        };
        let id = console.server_id.clone();
        drop(state);
        s.send(Command::Connect(id));
        true
    }
    #[func]
    fn set_menu(&self, open: bool) {
        if let Some(s) = &self.session {
            s.controller.set_menu(open);
        }
    }
    #[func]
    fn set_focused(&self, focused: bool) {
        if let Some(s) = &self.session {
            s.focused.store(focused, Ordering::Release);
        }
    }
    #[func]
    fn pad_sample(
        &self,
        connected: bool,
        buttons: PackedInt32Array,
        left: Vector2,
        right: Vector2,
        triggers: Vector2,
    ) {
        use xcloud_proto::input::{
            gamepad::RawGamepadState,
            models::{GamepadButtons, Vec2},
        };
        self.pad.lock().update(NativeGamepadSample {
            connected,
            name: if connected {
                "Android gamepad".into()
            } else {
                String::new()
            },
            state: RawGamepadState {
                buttons: buttons
                    .as_slice()
                    .iter()
                    .fold(GamepadButtons::NONE, |bits, index| {
                        bits | input::button(*index)
                    }),
                left_thumb: Vec2 {
                    x: left.x.clamp(-1., 1.),
                    y: left.y.clamp(-1., 1.),
                },
                right_thumb: Vec2 {
                    x: right.x.clamp(-1., 1.),
                    y: right.y.clamp(-1., 1.),
                },
                left_trigger: triggers.x.clamp(0., 1.),
                right_trigger: triggers.y.clamp(0., 1.),
                left_trigger_pressed: triggers.x > 0.5,
                right_trigger_pressed: triggers.y > 0.5,
            },
        });
    }
    #[func]
    fn snapshot(&self) -> VarDictionary {
        self.session
            .as_ref()
            .map(|s| crate::snapshot::dictionary(s, &self.video))
            .unwrap_or_default()
    }
    #[func]
    fn video_texture(&mut self) -> Option<Gd<godot::classes::Texture2D>> {
        self.video.texture()
    }
    #[func]
    fn update_video(&mut self) {
        self.video.update(self.session.as_ref());
    }
    #[func]
    fn clear_video(&mut self) {
        self.video.clear();
    }
    #[func]
    fn shutdown(&mut self) {
        self.video.clear();
        if let Some(mut s) = self.session.take() {
            s.focused.store(false, Ordering::Release);
            s.controller.set_menu(true);
            std::thread::spawn(move || s.shutdown());
        }
    }
}
impl Drop for XrboxRuntime {
    fn drop(&mut self) {
        self.shutdown();
    }
}
