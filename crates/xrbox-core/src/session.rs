use crate::{auth, media::Media};
use parking_lot::Mutex;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::mpsc;
use xcloud_api::{
    auth::MicrosoftAuthService, client::XCloudApiClient, transport::ReqwestTransport,
};
use xcloud_proto::{
    models::RemoteConsole,
    preferences::StreamKind,
    streaming_types::{PeerConnectionState, StreamLifecycleState, StreamingStatsSnapshot},
};
use xcloud_stream::orchestrator::{
    DisconnectIntent, OrchestratorEvent, RuntimeSignal, SignalingOrchestrator,
};
use xcloud_webrtc::{Transport, native::WebRtcTransport};

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
enum Outcome {
    NoSavedSession,
    Consoles(Vec<RemoteConsole>),
    Connect(String, Arc<XCloudApiClient>, String),
}
struct Active {
    orchestrator: SignalingOrchestrator,
    transport: Arc<WebRtcTransport>,
    media: Media,
    events: tokio::sync::broadcast::Receiver<OrchestratorEvent>,
    pipeline: crossbeam_channel::Receiver<xcloud_stream::pipeline::PipelineEvent>,
    input: xcloud_input::source::GamepadInputSource,
    started: Instant,
    first_frame: bool,
    baseline: u64,
    ready_since: Option<Instant>,
    route_job: Option<
        tokio::task::JoinHandle<
            xcloud_webrtc::Result<Option<xcloud_webrtc::native::SelectedRoute>>,
        >,
    >,
    next_route: Instant,
    next_report: Instant,
    last_decoded: u64,
    last_decode_at: Instant,
    next_recovery: Instant,
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
            runtime.block_on(run_sampler(rx, s, v, f, ctx, shared_controller, sampler));
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
async fn cancel(job: &mut Option<tokio::task::JoinHandle<anyhow::Result<Outcome>>>) {
    if let Some(job) = job.take() {
        job.abort();
        let _ = job.await;
    }
}
async fn stop(active: &mut Option<Active>, video: &Video) {
    if let Some(mut a) = active.take() {
        a.input.set_active(false);
        // Allow the upstream input flush timer to send the neutral state before closing.
        tokio::time::sleep(Duration::from_millis(20)).await;
        a.orchestrator.disconnect(DisconnectIntent::UserInitiated);
        a.media.audio.lock().stop();
        // The orchestrator's DELETE is asynchronous; allow a bounded completion window.
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    *video.lock() = None;
}
#[cfg(test)]
async fn run(
    rx: mpsc::Receiver<Command>,
    state: Arc<Mutex<Snapshot>>,
    video: Video,
    focused: Arc<AtomicBool>,
    ctx: crate::Notify,
    controller: crate::controller::Controller,
    native_controller: bool,
) {
    let sampler = if native_controller {
        xcloud_input::native::native_sampler()
    } else {
        Box::new(xcloud_input::native::sample_stub)
    };
    run_sampler(rx, state, video, focused, ctx, controller, sampler).await;
}
async fn run_sampler(
    mut rx: mpsc::Receiver<Command>,
    state: Arc<Mutex<Snapshot>>,
    video: Video,
    focused: Arc<AtomicBool>,
    ctx: crate::Notify,
    controller: crate::controller::Controller,
    mut sampler: xcloud_input::source::Sampler,
) {
    let mut menu_ticker = tokio::time::interval(Duration::from_millis(33));
    menu_ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut preferences = crate::config::Settings::default();
    let mut auth = auth::service(false).expect("HTTP client");
    let mut auth_persistent = false;
    let mut active: Option<Active> = None;
    let mut job: Option<tokio::task::JoinHandle<anyhow::Result<Outcome>>> = None;
    let input_clock = Instant::now();
    let mut ticker = tokio::time::interval(Duration::from_millis(8));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            command = rx.recv() => {
                let Some(command) = command else { break };
                if let Command::Configure(settings) = command {
                    if settings.validate().is_ok() {
                        let _ = controller.set_bindings(settings.hotkeys);
                        if let Some(a) = active.as_mut() {
                            settings.apply_controller(&mut a.input);
                            a.media.muted.store(settings.muted, Ordering::Relaxed);
                            if settings.muted && !preferences.muted { a.media.audio.lock().stop(); }
                        }
                        preferences = settings;
                    }
                    continue;
                }
                cancel(&mut job).await;
                state.lock().busy = false;
                state.lock().code = None;
                match command {
                    Command::Configure(_) => unreachable!("handled without cancelling session jobs"),
                    Command::Shutdown => break,
                    Command::Disconnect => {
                        stop(&mut active, &video).await;
                        state.lock().active = false;
                        status(&state, &ctx, "Disconnected");
                    }
                    Command::SignOut => {
                        stop(&mut active, &video).await;
                        let result = auth.clear_session();
                        // The current persistent service already clears the same Keychain item.
                        let persistent_clear = if auth_persistent {
                            Ok(())
                        } else {
                            auth::service(true).and_then(|a| a.clear_session().map_err(anyhow::Error::from))
                        };
                        *state.lock() = Snapshot::default();
                        status(
                            &state,
                            &ctx,
                            if result.is_ok() && persistent_clear.is_ok() {
                                "Signed out"
                            } else {
                                "Credential store could not be cleared; try sign out again"
                            },
                        );
                    }
                    Command::Restore => {
                        auth = match auth::service(true) {
                            Ok(service) => service,
                            Err(_) => {
                                status(&state, &ctx, "Secure storage unavailable. Uncheck Remember sign-in to use this session only.");
                                continue;
                            }
                        };
                        auth_persistent = true;
                        let auth = auth.clone();
                        state.lock().busy = true;
                        status(&state, &ctx, "Checking saved sign-in");
                        job = Some(tokio::spawn(async move {
                            restore(&auth).await
                        }));
                    }
                    Command::SignIn(persistent) => {
                        stop(&mut active, &video).await;
                        *state.lock() = Snapshot::default();
                        match auth::service(persistent) {
                            Ok(service) => {
                                auth = service;
                                auth_persistent = persistent;
                            },
                            Err(_) => {
                                status(&state, &ctx, "Cannot initialize authentication");
                                continue;
                            }
                        }
                        let (auth, s, c) = (auth.clone(), state.clone(), ctx.clone());
                        status(&state, &ctx, "Starting Microsoft sign-in");
                        state.lock().busy = true;
                        job = Some(tokio::spawn(async move {
                            let code = auth.request_device_code().await?;
                            s.lock().code = Some((code.user_code.clone(), code.verification_uri.clone()));
                            c.notify();
                            auth.poll_for_msa_token(
                                &code.device_code,
                                code.interval.max(1) as u64,
                                code.expires_in.max(1) as u64,
                            )
                            .await?;
                            discover(&auth).await
                        }));
                    }
                    Command::Discover(persistent) => {
                        if !auth.has_stored_session().unwrap_or(false)
                            && let Ok(restored) = auth::service(persistent)
                        {
                            auth = restored;
                            auth_persistent = persistent;
                        }
                        if active.is_some() {
                            continue;
                        }
                        let auth = auth.clone();
                        status(
                            &state,
                            &ctx,
                            "Refreshing credentials and discovering consoles",
                        );
                        state.lock().busy = true;
                        job = Some(tokio::spawn(async move {
                            discover(&auth).await
                        }));
                    }
                    Command::Connect(id) => {
                        if active.is_some()
                            || !state
                                .lock()
                                .consoles
                                .iter()
                                .any(|c| c.server_id == id && !id.is_empty())
                        {
                            continue;
                        }
                        let auth = auth.clone();
                        status(&state, &ctx, "Preparing console credentials");
                        state.lock().busy = true;
                        job = Some(tokio::spawn(async move {
                            let tokens = auth::home_credentials(&auth).await?;
                            let api = Arc::new(XCloudApiClient::new(Arc::new(ReqwestTransport::new()?)));
                            api.update_credentials(&tokens.xhome_host, &tokens.xhome_token);
                            let refresh = auth.token_store().load_refresh_token()?;
                            let lpt = auth.fetch_lpt(&refresh).await?;
                            Ok(Outcome::Connect(id, api, lpt))
                        }));
                    }
                }
            }
            result = async { job.as_mut().expect("guarded job").await }, if job.is_some() => {
                job = None;
                state.lock().busy = false;
                state.lock().code = None;
                match result {
                    Ok(Ok(Outcome::NoSavedSession)) => {
                        status(&state, &ctx, "Sign in to connect to your Xbox");
                    }
                    Ok(Ok(Outcome::Consoles(consoles))) => {
                        status(
                            &state,
                            &ctx,
                            if consoles.is_empty() {
                                "No consoles found. Check Remote Features and account."
                            } else {
                                "Select your console"
                            },
                        );
                        state.lock().consoles = consoles;
                    }
                    Ok(Ok(Outcome::Connect(id, api, lpt))) => {
                        match build_active(api, auth.clone(), ctx.clone(), &id, &lpt, &controller, &preferences) {
                            Ok(a) => {
                                *video.lock() = Some(a.media.reader.clone());
                                active = Some(a);
                                state.lock().active = true;
                                status(&state, &ctx, "Provisioning console session");
                            }
                            Err(_) => status(&state, &ctx, "Could not initialize media session"),
                        }
                    }
                    // Do not forward upstream errors containing response bodies or account identifiers.
                    Ok(Err(error)) => status(&state, &ctx, crate::diagnostics::describe(&error)),
                    _ => status(
                        &state,
                        &ctx,
                        "Background operation stopped unexpectedly; retry",
                    ),
                }
                ctx.notify();
            }
            _ = menu_ticker.tick(), if active.is_none() => {
                controller.set_menu(false);
                controller.poll(&mut sampler, false, focused.load(Ordering::Relaxed), input_clock.elapsed(), &ctx);
            }
            _ = ticker.tick(), if active.is_some() => {
                controller.poll(&mut sampler, true, focused.load(Ordering::Relaxed), input_clock.elapsed(), &ctx);
                let a = active.as_mut().expect("guarded active");
                let mut terminal = false;
                while let Ok(event) = a.events.try_recv() {
                    match event {
                        OrchestratorEvent::Error(error) => {
                            terminal = true;
                            status(
                                &state,
                                &ctx,
                                format!("Session failed ({})", error.code_name()),
                            );
                        }
                        OrchestratorEvent::Lifecycle(lifecycle) => {
                            if lifecycle == StreamLifecycleState::StartingSession {
                                a.first_frame = false;
                                a.baseline = a.media.decoded.load(Ordering::Relaxed);
                                a.ready_since = None;
                                a.last_decode_at=Instant::now();
                            }
                            if lifecycle == StreamLifecycleState::Connected {
                                a.ready_since = Some(Instant::now());
                                status(&state, &ctx, "Connected; awaiting decoded video");
                            } else {
                                status(&state, &ctx, format!("Session: {lifecycle:?}"));
                            }
                        }
                        OrchestratorEvent::ServerInitiatedDisconnect => {
                            terminal = true;
                            status(&state, &ctx, "Console ended the session");
                        }
                        _ => {}
                    }
                }
                while let Ok(event) = a.pipeline.try_recv() {
                    use xcloud_stream::pipeline::PipelineEvent;
                    match event {
                        PipelineEvent::KeyframeRecoveryNeeded => {
                            a.orchestrator.signal(RuntimeSignal::VideoRecoveryNeeded)
                        }
                        PipelineEvent::Resolution { width, height } => a
                            .orchestrator
                            .signal(RuntimeSignal::DecodedResolution { width, height }),
                        PipelineEvent::StatsChanged(stats) => state.lock().stats = stats,
                    }
                }
                let connected = a.transport.connection_state() == PeerConnectionState::Connected
                    && a.orchestrator.input_running();
                if connected && a.route_job.is_none() && Instant::now() >= a.next_route {
                    let transport = a.transport.clone();
                    a.route_job = Some(tokio::task::spawn_blocking(move || {
                        transport.selected_route()
                    }));
                    a.next_route = Instant::now() + Duration::from_secs(5);
                }
                if a.route_job.as_ref().is_some_and(|j| j.is_finished()) {
                    let route = a.route_job.take().expect("finished job").await;
                    use xcloud_webrtc::native::SelectedRoute;
                    state.lock().route=match route {
                                    Ok(Ok(Some(SelectedRoute::PrivateAddressPeer)))=>"Direct peer at private/link-local address (LAN candidate; VPNs may also use private addresses)",
                                    Ok(Ok(Some(SelectedRoute::DirectNonPrivatePeer)))=>"Direct peer at non-private address (not verified as LAN)",
                                    Ok(Ok(Some(SelectedRoute::Relay)))=> {terminal=true; "Relay detected; stopping because XRBox requires direct console streaming"},
                                    _=>"Selected route unavailable",
                                }.into();
                    if terminal {
                        status(
                            &state,
                            &ctx,
                            "Relay route rejected; check that Xbox and headset/client are on the same LAN",
                        );
                    }
                }

                a.input.set_active(connected);
                a.input.set_input_focused(focused.load(Ordering::Relaxed));
                a.input.tick(a.started.elapsed().as_millis() as u64);
                {
                    let mut s = state.lock();
                    s.controller = a.input.controller_name().to_owned();
                    let mut audio = a.media.audio.lock();
                    audio.poll_replies();
                    s.audio_playing = audio.is_playing();
                    s.audio_drops = audio.counters().dropped_chunks.load(Ordering::Relaxed);
                }
                let decoded=a.media.decoded.load(Ordering::Relaxed);
                if decoded!=a.last_decoded {
                    a.last_decoded=decoded;
                    a.last_decode_at=Instant::now();
                    let recovering=a.first_frame && state.lock().status.starts_with("Video stalled");
                    if recovering {status(&state,&ctx,"Streaming from Xbox (video recovered)");}
                }
                {
                    let mut s=state.lock();
                    s.media_health=a.media.health.lock().clone();
                    s.decoded_age_seconds=a.first_frame.then(||a.last_decode_at.elapsed().as_secs_f64());
                }
                if a.first_frame && connected && a.last_decode_at.elapsed()>Duration::from_secs(3) && Instant::now()>=a.next_recovery {
                    status(&state,&ctx,"Video stalled; requesting a fresh keyframe");
                    a.transport.request_keyframe();
                    a.orchestrator.signal(RuntimeSignal::VideoRecoveryNeeded);
                    a.next_recovery=Instant::now()+Duration::from_secs(3);
                }
                if a.first_frame && a.last_decode_at.elapsed()>Duration::from_secs(15) {
                    status(&state,&ctx,"Video stalled for 15 seconds; session stopped. Reconnect to retry.");
                    terminal=true;
                }
                if Instant::now() >= a.next_report {
                    let s = state.lock();
                    tracing::info!(target:"xrbox", decoded_frames=s.media_health.decoded, assembled_frames=s.media_health.assembled, decoder_errors=s.media_health.decoder_errors, decode_queue_drops=s.media_health.queue_drops, gated_frames=s.media_health.gated, waiting_keyframe=s.media_health.waiting_keyframe, last_decoded_age_seconds=s.decoded_age_seconds, fps=s.stats.frames_per_second, bitrate_kbps=s.stats.bitrate_kbps, incomplete_frames_dropped=s.media_health.incomplete_drops, audio_device=s.audio_playing, audio_drops=s.audio_drops, controller_connected=a.input.connected(), route=%s.route, "Stream diagnostics (not end-to-end latency)");
                    a.next_report = Instant::now() + Duration::from_secs(5);
                }
                if !a.first_frame
                    && streaming_ready(
                        connected,
                        a.media.decoded.load(Ordering::Relaxed),
                        a.baseline,
                    )
                {
                    a.first_frame = true;
                    status(&state, &ctx, "Streaming from Xbox");
                }
                if a.ready_since
                    .is_some_and(|t| t.elapsed() > Duration::from_secs(15))
                    && !a.first_frame
                {
                    status(
                        &state,
                        &ctx,
                        "No decoded video after 15 seconds; disconnect and retry",
                    );
                    a.ready_since = None;
                    terminal = true;
                }
                if terminal {
                    stop(&mut active, &video).await;
                    let mut s = state.lock();
                    s.active = false;
                    s.audio_playing = false;
                    s.controller.clear();
                    ctx.notify();
                }
            }
        }
    }
    cancel(&mut job).await;
    stop(&mut active, &video).await;
}
fn build_active(
    api: Arc<XCloudApiClient>,
    auth: Arc<MicrosoftAuthService>,
    ctx: crate::Notify,
    id: &str,
    lpt: &str,
    controller: &crate::controller::Controller,
    preferences: &crate::config::Settings,
) -> anyhow::Result<Active> {
    let transport = Arc::new(WebRtcTransport::try_new()?);
    let (media, pipeline) = Media::attach(&transport, ctx, preferences.muted);
    let orchestrator = SignalingOrchestrator::new(Some(api), Some(auth), transport.clone())?;
    orchestrator.set_preferences(preferences.stream_preferences());
    orchestrator.set_diagnostics_polling_enabled(true);
    let events = orchestrator.subscribe();
    let mut input = xcloud_input::source::GamepadInputSource::new(controller.stream_sampler());
    preferences.apply_controller(&mut input);
    input.set_input_queue(Some(orchestrator.input_queue()));
    orchestrator.set_gamepad_connection_state(0, true);
    orchestrator.connect_stream(StreamKind::Home, id, lpt);
    Ok(Active {
        orchestrator,
        transport,
        media,
        events,
        pipeline,
        input,
        started: Instant::now(),
        first_frame: false,
        baseline: 0,
        ready_since: None,
        route_job: None,
        next_route: Instant::now(),
        next_report: Instant::now() + Duration::from_secs(5),
        last_decoded: 0,
        last_decode_at: Instant::now(),
        next_recovery: Instant::now(),
    })
}

/// A signaling state or a frame from a previous attempt cannot establish playback.
fn streaming_ready(connected: bool, decoded: u64, baseline: u64) -> bool {
    connected && decoded > baseline
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn streaming_requires_connection_and_new_decoded_media() {
        assert!(!streaming_ready(false, 1, 0));
        assert!(!streaming_ready(true, 0, 0));
        assert!(!streaming_ready(true, 10, 10));
        assert!(streaming_ready(true, 11, 10));
    }
    #[test]
    fn invalid_console_disconnect_and_shutdown_do_not_start_media() {
        let mut session = Session::new(crate::Notify::default(), false);
        session.send(Command::Connect("not-discovered".into()));
        session.send(Command::Disconnect);
        session.shutdown();
        assert!(!session.state.lock().active);
        assert!(session.video.lock().is_none());
        assert!(!session.state.lock().busy);
    }
}

async fn discover(auth: &MicrosoftAuthService) -> anyhow::Result<Outcome> {
    let tokens = auth::home_credentials(auth).await?;
    let api = XCloudApiClient::new(Arc::new(ReqwestTransport::new()?));
    api.update_credentials(&tokens.xhome_host, &tokens.xhome_token);
    Ok(Outcome::Consoles(api.get_consoles().await?.results))
}

async fn restore(auth: &MicrosoftAuthService) -> anyhow::Result<Outcome> {
    if !auth.has_stored_session()? {
        return Ok(Outcome::NoSavedSession);
    }
    discover(auth).await
}

#[cfg(test)]
mod restore_tests {
    use super::*;
    use xcloud_api::{
        token_store::{MemoryTokenStore, TokenStore},
        transport::FakeTransport,
    };

    #[tokio::test]
    async fn empty_store_needs_sign_in_without_network() {
        let transport = Arc::new(FakeTransport::new());
        let auth = MicrosoftAuthService::new(transport.clone(), Arc::new(MemoryTokenStore::new()));
        assert!(matches!(
            restore(&auth).await.unwrap(),
            Outcome::NoSavedSession
        ));
        assert_eq!(transport.request_count(), 0);
    }

    #[tokio::test]
    async fn saved_login_attempts_refresh_and_reports_revocation_safely() {
        let transport = Arc::new(FakeTransport::new());
        transport.enqueue_json(
            "token",
            r#"{"error":"invalid_grant","error_description":"private detail"}"#,
            400,
        );
        let store = Arc::new(MemoryTokenStore::new());
        store.save_refresh_token("synthetic-refresh").unwrap();
        let auth = MicrosoftAuthService::new(transport.clone(), store);
        let error = restore(&auth).await.err().expect("revoked token must fail");
        assert_eq!(transport.request_count(), 1);
        let message = crate::diagnostics::describe(&error);
        assert!(!message.contains("private detail"));
        assert!(!message.contains("synthetic-refresh"));
        assert!(
            message.contains("sign in")
                || message.contains("Sign in")
                || message.contains("HTTP 400")
        );
    }
}

#[cfg(test)]
mod configure_tests {
    use super::*;
    #[tokio::test]
    async fn configuration_preserves_auth_progress_and_applies_custom_controller_bindings() {
        use xcloud_proto::input::{gamepad::RawGamepadState, models::GamepadButtons as B};
        let (tx, rx) = mpsc::channel(2);
        let bindings = crate::config::HotkeyBindings {
            guide: (B::VIEW | B::X).bits(),
            menu: (B::MENU | B::Y).bits(),
        };
        tx.send(Command::Configure(crate::config::Settings {
            hotkeys: bindings,
            ..Default::default()
        }))
        .await
        .unwrap();
        drop(tx);
        let state = Arc::new(Mutex::new(Snapshot {
            busy: true,
            code: Some(("synthetic".into(), "synthetic".into())),
            ..Default::default()
        }));
        let controller = crate::controller::Controller::default();
        let ctx = crate::Notify::default();
        run(
            rx,
            state.clone(),
            Default::default(),
            Arc::new(AtomicBool::new(true)),
            ctx.clone(),
            controller.clone(),
            false,
        )
        .await;
        assert!(state.lock().busy);
        assert!(state.lock().code.is_some());
        for buttons in [B::NONE, B::MENU | B::Y, B::NONE] {
            let mut sampler: xcloud_input::source::Sampler =
                Box::new(move || xcloud_input::source::NativeGamepadSample {
                    connected: true,
                    state: RawGamepadState {
                        buttons,
                        ..Default::default()
                    },
                    ..Default::default()
                });
            controller.poll(&mut sampler, true, true, Duration::ZERO, &ctx);
        }
        assert!(controller.menu());
    }
}
