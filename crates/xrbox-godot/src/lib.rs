//! Godot control bridge; streaming/media ownership stays in the Rust session core.
mod input;
#[cfg(any(target_os = "android", test))]
mod retirement;
use godot::{classes::IRefCounted, prelude::*};
use parking_lot::Mutex;
use std::sync::{Arc, atomic::Ordering};
use xcloud_input::source::NativeGamepadSample;
use xrbox_core::session::{Command, Session};
#[cfg(target_os = "android")]
mod android_video;
struct XrboxExtension;
#[gdextension]
unsafe impl ExtensionLibrary for XrboxExtension {}
#[derive(GodotClass)]
#[class(base=RefCounted)]
struct XrboxRuntime {
    base: Base<RefCounted>,
    session: Option<Session>,
    pad: Arc<Mutex<input::Pad>>,
    #[cfg(target_os = "android")]
    presenter: Option<Arc<android_video::Presenter>>,
    #[cfg(target_os = "android")]
    texture: Option<Gd<godot::classes::ExternalTexture>>,
}
#[godot_api]
impl IRefCounted for XrboxRuntime {
    fn init(base: Base<RefCounted>) -> Self {
        Self {
            base,
            session: None,
            pad: Default::default(),
            #[cfg(target_os = "android")]
            presenter: None,
            #[cfg(target_os = "android")]
            texture: None,
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
        let mut out = VarDictionary::new();
        let Some(s) = &self.session else { return out };
        let state = s.state.lock();
        out.set("status", state.status.as_str());
        out.set("active", state.active);
        out.set("busy", state.busy);
        out.set("menu", s.controller.menu());
        let mut names = PackedStringArray::new();
        for console in &state.consoles {
            names.push(console.device_name.as_str());
        }
        out.set("consoles", &names);
        out.set(
            "code",
            state.code.as_ref().map(|c| c.0.as_str()).unwrap_or(""),
        );
        out.set(
            "uri",
            state.code.as_ref().map(|c| c.1.as_str()).unwrap_or(""),
        );
        out.set("assembled", state.media_health.assembled as i64);
        out.set("decoded", state.media_health.decoded as i64);
        out.set("decoder_errors", state.media_health.decoder_errors as i64);
        out.set("audio", state.audio_playing);
        out.set("audio_drops", state.audio_drops as i64);
        out.set("route", state.route.as_str());
        #[cfg(target_os = "android")]
        {
            out.set("codec", xcloud_media::android::selected_codec().as_str());
            out.set("codec_status", xcloud_media::android::status());
        }
        #[cfg(target_os = "android")]
        if let Some(p) = &self.presenter {
            out.set("gpu_imports", p.imported.load(Ordering::Relaxed) as i64);
            out.set("gpu_error", p.failed.load(Ordering::Acquire));
        }
        out
    }
    #[func]
    fn video_texture(&mut self) -> Option<Gd<godot::classes::Texture2D>> {
        #[cfg(target_os = "android")]
        {
            if self.texture.is_none() {
                let mut texture = godot::classes::ExternalTexture::new_gd();
                texture.set_size(Vector2::new(1920., 1080.));
                self.presenter = Some(android_video::Presenter::new());
                self.texture = Some(texture);
            }
            self.texture
                .as_ref()
                .map(|texture| texture.clone().upcast())
        }
        #[cfg(not(target_os = "android"))]
        {
            None
        }
    }
    #[func]
    fn update_video(&mut self) {
        #[cfg(target_os = "android")]
        {
            let (Some(session), Some(texture), Some(presenter)) =
                (&self.session, &mut self.texture, &self.presenter)
            else {
                return;
            };
            if presenter.failed.load(Ordering::Acquire) {
                if !presenter.closed.swap(true, Ordering::AcqRel) {
                    godot_error!("Android EGL image import failed; disconnecting stream");
                    presenter.close();
                    session.send(Command::Disconnect);
                }
                return;
            }
            let reader = session.video.lock().clone();
            if let Some(reader) = reader
                && let Some(frame) = reader.lock().take_latest()
            {
                texture.set_size(Vector2::new(frame.width as f32, frame.height as f32));
                presenter.submit(frame, texture.get_external_texture_id() as u32);
            }
        }
    }
    #[func]
    fn clear_video(&mut self) {
        #[cfg(target_os = "android")]
        if let Some(p) = self.presenter.take() {
            p.close();
        }
        #[cfg(target_os = "android")]
        {
            self.texture = None;
        }
    }
    #[func]
    fn shutdown(&mut self) {
        #[cfg(target_os = "android")]
        if let Some(p) = self.presenter.take() {
            p.close();
        }
        #[cfg(target_os = "android")]
        {
            self.texture = None;
        }
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

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_org_xrbox_AndroidServices_initializeNative(
    env: jni::JNIEnv,
    class: jni::objects::JClass,
    application: jni::objects::JObject,
    codec: jni::objects::JString,
) {
    xrbox_core::android::initialize(env, class, application, codec);
}
