//! Engine-thread texture adapter; native imports remain on the render thread.
use godot::prelude::*;
use xrbox_core::session::Session;
#[cfg(target_os = "android")]
use {
    crate::android_video,
    std::sync::{Arc, atomic::Ordering},
    xrbox_core::session::Command,
};
#[derive(Default)]
pub(super) struct Video {
    #[cfg(target_os = "android")]
    pub(super) presenter: Option<Arc<android_video::Presenter>>,
    #[cfg(target_os = "android")]
    texture: Option<Gd<godot::classes::ExternalTexture>>,
}
impl Video {
    pub(super) fn texture(&mut self) -> Option<Gd<godot::classes::Texture2D>> {
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
    pub(super) fn update(&mut self, _session: Option<&Session>) {
        #[cfg(target_os = "android")]
        {
            let (Some(session), Some(texture), Some(presenter)) =
                (_session, &mut self.texture, &self.presenter)
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
    pub(super) fn clear(&mut self) {
        #[cfg(target_os = "android")]
        if let Some(p) = self.presenter.take() {
            p.close();
        }
        #[cfg(target_os = "android")]
        {
            self.texture = None;
        }
    }
}
