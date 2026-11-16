use super::{Command, Snapshot, Video, status};
use super::{
    active::{Active, build_active, stop},
    auth_jobs::{Outcome, connect, discover, restore, sign_in},
};
use crate::auth;
use parking_lot::Mutex;
use std::{sync::atomic::Ordering, time::Instant};
use std::{
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};
use tokio::sync::mpsc;
use xcloud_api::auth::MicrosoftAuthService;
type Job = tokio::task::JoinHandle<anyhow::Result<Outcome>>;
struct Worker {
    state: Arc<Mutex<Snapshot>>,
    video: Video,
    focused: Arc<AtomicBool>,
    ctx: crate::Notify,
    controller: crate::controller::Controller,
    sampler: xcloud_input::source::Sampler,
    preferences: crate::config::Settings,
    auth: Arc<MicrosoftAuthService>,
    auth_persistent: bool,
    active: Option<Active>,
    job: Option<Job>,
    input_clock: Instant,
}
pub(super) async fn run_sampler(
    mut rx: mpsc::Receiver<Command>,
    state: Arc<Mutex<Snapshot>>,
    video: Video,
    focused: Arc<AtomicBool>,
    ctx: crate::Notify,
    controller: crate::controller::Controller,
    sampler: xcloud_input::source::Sampler,
) {
    let mut worker = Worker {
        state,
        video,
        focused,
        ctx,
        controller,
        sampler,
        preferences: Default::default(),
        auth: auth::service(false).expect("HTTP client"),
        auth_persistent: false,
        active: None,
        job: None,
        input_clock: Instant::now(),
    };
    let mut menu_ticker = tokio::time::interval(Duration::from_millis(33));
    menu_ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut ticker = tokio::time::interval(Duration::from_millis(8));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            command = rx.recv() => {
                let Some(command) = command else { break };
                if worker.command(command).await { break; }
            }
            result = async { worker.job.as_mut().expect("guarded job").await }, if worker.job.is_some() => {
                worker.completed(result);
            }
            _ = menu_ticker.tick(), if worker.active.is_none() => {
                worker.controller.set_menu(false);
                worker.poll_input(false);
            }
            _ = ticker.tick(), if worker.active.is_some() => {
                worker.poll_input(true);
                worker.tick().await;
            }
        }
    }
    cancel(&mut worker.job).await;
    stop(&mut worker.active, &worker.video).await;
}
impl Worker {
    async fn command(&mut self, command: Command) -> bool {
        if let Command::Configure(settings) = command {
            if settings.validate().is_ok() {
                let _ = self.controller.set_bindings(settings.hotkeys);
                if let Some(a) = self.active.as_mut() {
                    settings.apply_controller(&mut a.input);
                    a.media.muted.store(settings.muted, Ordering::Relaxed);
                    if settings.muted && !self.preferences.muted {
                        a.media.audio.lock().stop();
                    }
                }
                self.preferences = settings;
            }
            return false;
        }
        cancel(&mut self.job).await;
        self.state.lock().busy = false;
        self.state.lock().code = None;
        match command {
            Command::Configure(_) => unreachable!("handled without cancelling session jobs"),
            Command::Shutdown => return true,
            Command::Disconnect => {
                stop(&mut self.active, &self.video).await;
                self.state.lock().active = false;
                status(&self.state, &self.ctx, "Disconnected");
            }
            Command::SignOut => {
                stop(&mut self.active, &self.video).await;
                let result = self.auth.clear_session();
                // The current persistent service already clears the same Keychain item.
                let persistent_clear = if self.auth_persistent {
                    Ok(())
                } else {
                    auth::service(true).and_then(|a| a.clear_session().map_err(anyhow::Error::from))
                };
                *self.state.lock() = Snapshot::default();
                status(
                    &self.state,
                    &self.ctx,
                    if result.is_ok() && persistent_clear.is_ok() {
                        "Signed out"
                    } else {
                        "Credential store could not be cleared; try sign out again"
                    },
                );
            }
            Command::Restore => {
                self.auth = match auth::service(true) {
                    Ok(service) => service,
                    Err(_) => {
                        status(
                            &self.state,
                            &self.ctx,
                            "Secure storage unavailable. Uncheck Remember sign-in to use this session only.",
                        );
                        return false;
                    }
                };
                self.auth_persistent = true;
                let auth = self.auth.clone();
                self.state.lock().busy = true;
                status(&self.state, &self.ctx, "Checking saved sign-in");
                self.job = Some(tokio::spawn(async move { restore(&auth).await }));
            }
            Command::SignIn(persistent) => {
                stop(&mut self.active, &self.video).await;
                *self.state.lock() = Snapshot::default();
                match auth::service(persistent) {
                    Ok(service) => {
                        self.auth = service;
                        self.auth_persistent = persistent;
                    }
                    Err(_) => {
                        status(&self.state, &self.ctx, "Cannot initialize authentication");
                        return false;
                    }
                }
                let (auth, s, c) = (self.auth.clone(), self.state.clone(), self.ctx.clone());
                status(&self.state, &self.ctx, "Starting Microsoft sign-in");
                self.state.lock().busy = true;
                self.job = Some(tokio::spawn(async move { sign_in(auth, s, c).await }));
            }
            Command::Discover(persistent) => {
                if !self.auth.has_stored_session().unwrap_or(false)
                    && let Ok(restored) = auth::service(persistent)
                {
                    self.auth = restored;
                    self.auth_persistent = persistent;
                }
                if self.active.is_some() {
                    return false;
                }
                let auth = self.auth.clone();
                status(
                    &self.state,
                    &self.ctx,
                    "Refreshing credentials and discovering consoles",
                );
                self.state.lock().busy = true;
                self.job = Some(tokio::spawn(async move { discover(&auth).await }));
            }
            Command::Connect(id) => {
                if self.active.is_some()
                    || !self
                        .state
                        .lock()
                        .consoles
                        .iter()
                        .any(|c| c.server_id == id && !id.is_empty())
                {
                    return false;
                }
                let auth = self.auth.clone();
                status(&self.state, &self.ctx, "Preparing console credentials");
                self.state.lock().busy = true;
                self.job = Some(tokio::spawn(async move { connect(auth, id).await }));
            }
        }
        false
    }
    fn completed(&mut self, result: Result<anyhow::Result<Outcome>, tokio::task::JoinError>) {
        self.job = None;
        self.state.lock().busy = false;
        self.state.lock().code = None;
        match result {
            Ok(Ok(Outcome::NoSavedSession)) => {
                status(&self.state, &self.ctx, "Sign in to connect to your Xbox");
            }
            Ok(Ok(Outcome::Consoles(consoles))) => {
                status(
                    &self.state,
                    &self.ctx,
                    if consoles.is_empty() {
                        "No consoles found. Check Remote Features and account."
                    } else {
                        "Select your console"
                    },
                );
                self.state.lock().consoles = consoles;
            }
            Ok(Ok(Outcome::Connect(id, api, lpt))) => {
                match build_active(
                    api,
                    self.auth.clone(),
                    self.ctx.clone(),
                    &id,
                    &lpt,
                    &self.controller,
                    &self.preferences,
                ) {
                    Ok(a) => {
                        *self.video.lock() = Some(a.media.reader.clone());
                        self.active = Some(a);
                        self.state.lock().active = true;
                        status(&self.state, &self.ctx, "Provisioning console session");
                    }
                    Err(_) => status(&self.state, &self.ctx, "Could not initialize media session"),
                }
            }
            // Do not forward upstream errors containing response bodies or account identifiers.
            Ok(Err(error)) => status(&self.state, &self.ctx, crate::diagnostics::describe(&error)),
            _ => status(
                &self.state,
                &self.ctx,
                "Background operation stopped unexpectedly; retry",
            ),
        }
        self.ctx.notify();
    }
    fn poll_input(&mut self, streaming: bool) {
        self.controller.poll(
            &mut self.sampler,
            streaming,
            self.focused.load(Ordering::Relaxed),
            self.input_clock.elapsed(),
            &self.ctx,
        );
    }
    async fn tick(&mut self) {
        let terminal = super::active::tick(
            self.active.as_mut().expect("guarded active"),
            &self.state,
            &self.focused,
            &self.ctx,
        )
        .await;
        if terminal {
            stop(&mut self.active, &self.video).await;
            let mut s = self.state.lock();
            s.active = false;
            s.audio_playing = false;
            s.controller.clear();
            self.ctx.notify();
        }
    }
}
pub(super) async fn cancel(job: &mut Option<tokio::task::JoinHandle<anyhow::Result<Outcome>>>) {
    if let Some(job) = job.take() {
        job.abort();
        let _ = job.await;
    }
}
