//! Godot extension registration. Session ownership stays in the Rust core.
#[cfg(target_os = "android")]
mod android_bootstrap;
#[cfg(target_os = "android")]
mod android_video;
mod input;
#[cfg(any(target_os = "android", test))]
mod retirement;
mod runtime;
mod snapshot;
mod video;
use godot::prelude::*;
struct XrboxExtension;
#[gdextension]
unsafe impl ExtensionLibrary for XrboxExtension {}
