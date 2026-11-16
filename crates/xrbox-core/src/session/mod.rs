//! Public session handle. Worker and active-stream ownership remain engine independent.
mod active;
mod auth_jobs;
#[cfg(test)]
mod tests;
mod worker;
use parking_lot::Mutex;
use std::{
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};
use tokio::sync::mpsc;
use xcloud_proto::{models::RemoteConsole, streaming_types::StreamingStatsSnapshot};
pub type Video = Arc<Mutex<Option<Arc<Mutex<xcloud_media::MailboxReader>>>>>;
#[derive(Clone, Default)]
pub struct Snapshot {
    pub status: String,
    pub busy: bool,
    pub active: bool,
    pub consoles: Vec<RemoteConsole>,
    pub code: Option<(String, String)>,
    pub stats: StreamingStatsSnapshot,
    pub controller: String,
    pub audio_playing: bool,
    pub audio_drops: u64,
    pub route: String,
    pub media_health: crate::media::MediaHealth,
    pub decoded_age_seconds: Option<f64>,
}
pub enum Command {
    Configure(crate::config::Settings),
    Restore,
    SignIn(bool),
    Discover(bool),
    Connect(String),
    Disconnect,
    SignOut,
    Shutdown,
}
pub struct Session {
    pub tx: mpsc::Sender<Command>,
    pub state: Arc<Mutex<Snapshot>>,
    pub video: Video,
    pub focused: Arc<AtomicBool>,
    pub controller: crate::controller::Controller,
    join: Option<std::thread::JoinHandle<()>>,
}
impl Session {
    pub fn new(ctx: crate::Notify, native_controller: bool) -> Self {
        Self::with_sampler(
            ctx,
            if native_controller {
                xcloud_input::native::native_sampler()
            } else {
                Box::new(xcloud_input::native::sample_stub)
            },
        )
    }
    pub fn with_sampler(ctx: crate::Notify, sampler: xcloud_input::source::Sampler) -> Self {
        let (tx, rx) = mpsc::channel(16);
        let state = Arc::new(Mutex::new(Snapshot {
            status: "Sign in to discover your Xbox".into(),
            ..Default::default()
        }));
        let video: Video = Default::default();
        let focused = Arc::new(AtomicBool::new(true));
        let controller = crate::controller::Controller::default();
        let shared_controller = controller.clone();
        let (s, v, f) = (state.clone(), video.clone(), focused.clone());
        let join = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .expect("Tokio runtime");
            runtime.block_on(worker::run_sampler(
                rx,
                s,
                v,
                f,
                ctx,
                shared_controller,
                sampler,
            ));
            runtime.shutdown_timeout(Duration::from_secs(2));
        });
        Self {
            tx,
            state,
            video,
            focused,
            controller,
            join: Some(join),
        }
    }
    pub fn send(&self, command: Command) {
        if self.tx.try_send(command).is_err() {
            self.state.lock().status = "Command queue unavailable; try again".into();
        }
    }
    pub fn shutdown(&mut self) {
        let _ = self.tx.blocking_send(Command::Shutdown);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}
fn status(state: &Mutex<Snapshot>, ctx: &crate::Notify, message: impl Into<String>) {
    let message = message.into();
    tracing::info!(target: "xrbox", stage = %message);
    state.lock().status = message;
    ctx.notify();
}
