//! Engine-independent Xbox console Remote Play core.
//!
//! `session` is the opt-in streaming stack. Decoder/input/credential adapters
//! are selected per platform. This crate has no Godot dependency.
use std::sync::Arc;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Thread-safe wake-up only. A frontend reads the existing latest-state mailbox
/// on its own thread; workers never receive an engine object or UI API.
#[derive(Clone)]
pub struct Notify(Arc<dyn Fn() + Send + Sync>);

impl Notify {
    pub fn new(callback: impl Fn() + Send + Sync + 'static) -> Self {
        Self(Arc::new(callback))
    }

    pub fn notify(&self) {
        (self.0)();
    }
}

impl Default for Notify {
    fn default() -> Self {
        Self::new(|| {})
    }
}

#[cfg(feature = "session")]
pub mod auth;
#[cfg(feature = "session")]
pub mod config;
#[cfg(feature = "session")]
pub mod controller;
#[cfg(feature = "session")]
pub mod diagnostics;
#[cfg(feature = "session")]
pub mod media;
#[cfg(feature = "session")]
pub mod session;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn cloned_notifier_wakes_frontend_from_worker_without_an_engine() {
        let count = Arc::new(AtomicUsize::new(0));
        let observed = count.clone();
        let notifier = Notify::new(move || {
            observed.fetch_add(1, Ordering::Relaxed);
        });
        let worker = notifier.clone();
        std::thread::spawn(move || worker.notify()).join().unwrap();
        notifier.notify();
        assert_eq!(count.load(Ordering::Relaxed), 2);
    }
}

#[cfg(all(target_os = "android", feature = "session"))]
pub mod android;
