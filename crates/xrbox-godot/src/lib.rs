//! Native bring-up interface. Media/session commands are deliberately absent until
//! native decoder surfaces can be presented by Godot without CPU frame uploads.
use godot::{classes::IRefCounted, prelude::*};

struct XrboxExtension;

// SAFETY: all Godot access remains on the calling engine thread; no native worker
// calls engine APIs. Default godot-rust safeguards remain enabled.
#[gdextension]
unsafe impl ExtensionLibrary for XrboxExtension {}

#[derive(GodotClass)]
#[class(base=RefCounted)]
struct XrboxRuntime {
    base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for XrboxRuntime {
    fn init(base: Base<RefCounted>) -> Self {
        Self { base }
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
}
