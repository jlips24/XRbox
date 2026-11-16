//! Godot dictionary contract; keep keys stable for scripts.
use godot::prelude::*;
#[cfg(target_os = "android")]
use std::sync::atomic::Ordering;
pub(super) fn dictionary(
    s: &xrbox_core::session::Session,
    _video: &crate::video::Video,
) -> VarDictionary {
    let mut out = VarDictionary::new();

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
    if let Some(p) = &_video.presenter {
        out.set("gpu_imports", p.imported.load(Ordering::Relaxed) as i64);
        out.set("gpu_error", p.failed.load(Ordering::Acquire));
    }
    out
}
