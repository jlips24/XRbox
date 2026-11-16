use crate::media::Media;
use parking_lot::Mutex;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use xcloud_api::{auth::MicrosoftAuthService, client::XCloudApiClient};
use xcloud_proto::{
    preferences::StreamKind,
    streaming_types::{PeerConnectionState, StreamLifecycleState},
};
use xcloud_stream::orchestrator::{
    DisconnectIntent, OrchestratorEvent, RuntimeSignal, SignalingOrchestrator,
};
use xcloud_webrtc::{Transport, native::WebRtcTransport};

use super::{Snapshot, Video, status};
pub(super) struct Active {
    orchestrator: SignalingOrchestrator,
    transport: Arc<WebRtcTransport>,
    pub(super) media: Media,
    events: tokio::sync::broadcast::Receiver<OrchestratorEvent>,
    pipeline: crossbeam_channel::Receiver<xcloud_stream::pipeline::PipelineEvent>,
    pub(super) input: xcloud_input::source::GamepadInputSource,
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
pub(super) async fn stop(active: &mut Option<Active>, video: &Video) {
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
pub(super) fn build_active(
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
pub(super) fn streaming_ready(connected: bool, decoded: u64, baseline: u64) -> bool {
    connected && decoded > baseline
}
pub(super) async fn tick(
    a: &mut Active,
    state: &Mutex<Snapshot>,
    focused: &AtomicBool,
    ctx: &crate::Notify,
) -> bool {
    let mut terminal = poll_events(a, state, ctx);
    poll_pipeline(a, state);
    let connected = a.transport.connection_state() == PeerConnectionState::Connected
        && a.orchestrator.input_running();
    poll_route(a, connected, state, ctx, &mut terminal).await;
    poll_input_audio(a, connected, state, focused);
    poll_video_health(a, connected, state, ctx, &mut terminal);
    report_diagnostics(a, state);
    poll_readiness(a, connected, state, ctx, &mut terminal);
    terminal
}
fn poll_events(a: &mut Active, state: &Mutex<Snapshot>, ctx: &crate::Notify) -> bool {
    let mut terminal = false;
    while let Ok(event) = a.events.try_recv() {
        match event {
            OrchestratorEvent::Error(error) => {
                terminal = true;
                status(
                    state,
                    ctx,
                    format!("Session failed ({})", error.code_name()),
                );
            }
            OrchestratorEvent::Lifecycle(lifecycle) => {
                if lifecycle == StreamLifecycleState::StartingSession {
                    a.first_frame = false;
                    a.baseline = a.media.decoded.load(Ordering::Relaxed);
                    a.ready_since = None;
                    a.last_decode_at = Instant::now();
                }
                if lifecycle == StreamLifecycleState::Connected {
                    a.ready_since = Some(Instant::now());
                    status(state, ctx, "Connected; awaiting decoded video");
                } else {
                    status(state, ctx, format!("Session: {lifecycle:?}"));
                }
            }
            OrchestratorEvent::ServerInitiatedDisconnect => {
                terminal = true;
                status(state, ctx, "Console ended the session");
            }
            _ => {}
        }
    }
    terminal
}
fn poll_pipeline(a: &mut Active, state: &Mutex<Snapshot>) {
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
}
async fn poll_route(
    a: &mut Active,
    connected: bool,
    state: &Mutex<Snapshot>,
    ctx: &crate::Notify,
    terminal: &mut bool,
) {
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
                                    Ok(Ok(Some(SelectedRoute::Relay)))=> {*terminal=true; "Relay detected; stopping because XRbox requires direct console streaming"},
                                    _=>"Selected route unavailable",
                                }.into();
        if *terminal {
            status(
                state,
                ctx,
                "Relay route rejected; check that Xbox and headset/client are on the same LAN",
            );
        }
    }
}
fn poll_input_audio(
    a: &mut Active,
    connected: bool,
    state: &Mutex<Snapshot>,
    focused: &AtomicBool,
) {
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
}
fn poll_video_health(
    a: &mut Active,
    connected: bool,
    state: &Mutex<Snapshot>,
    ctx: &crate::Notify,
    terminal: &mut bool,
) {
    let decoded = a.media.decoded.load(Ordering::Relaxed);
    if decoded != a.last_decoded {
        a.last_decoded = decoded;
        a.last_decode_at = Instant::now();
        let recovering = a.first_frame && state.lock().status.starts_with("Video stalled");
        if recovering {
            status(state, ctx, "Streaming from Xbox (video recovered)");
        }
    }
    {
        let mut s = state.lock();
        s.media_health = a.media.health.lock().clone();
        s.decoded_age_seconds = a
            .first_frame
            .then(|| a.last_decode_at.elapsed().as_secs_f64());
    }
    if a.first_frame
        && connected
        && a.last_decode_at.elapsed() > Duration::from_secs(3)
        && Instant::now() >= a.next_recovery
    {
        status(state, ctx, "Video stalled; requesting a fresh keyframe");
        a.transport.request_keyframe();
        a.orchestrator.signal(RuntimeSignal::VideoRecoveryNeeded);
        a.next_recovery = Instant::now() + Duration::from_secs(3);
    }
    if a.first_frame && a.last_decode_at.elapsed() > Duration::from_secs(15) {
        status(
            state,
            ctx,
            "Video stalled for 15 seconds; session stopped. Reconnect to retry.",
        );
        *terminal = true;
    }
}
fn report_diagnostics(a: &mut Active, state: &Mutex<Snapshot>) {
    if Instant::now() >= a.next_report {
        let s = state.lock();
        tracing::info!(target:"xrbox", decoded_frames=s.media_health.decoded, assembled_frames=s.media_health.assembled, decoder_errors=s.media_health.decoder_errors, decode_queue_drops=s.media_health.queue_drops, gated_frames=s.media_health.gated, waiting_keyframe=s.media_health.waiting_keyframe, last_decoded_age_seconds=s.decoded_age_seconds, fps=s.stats.frames_per_second, bitrate_kbps=s.stats.bitrate_kbps, incomplete_frames_dropped=s.media_health.incomplete_drops, audio_device=s.audio_playing, audio_drops=s.audio_drops, controller_connected=a.input.connected(), route=%s.route, "Stream diagnostics (not end-to-end latency)");
        a.next_report = Instant::now() + Duration::from_secs(5);
    }
}
fn poll_readiness(
    a: &mut Active,
    connected: bool,
    state: &Mutex<Snapshot>,
    ctx: &crate::Notify,
    terminal: &mut bool,
) {
    if !a.first_frame
        && streaming_ready(
            connected,
            a.media.decoded.load(Ordering::Relaxed),
            a.baseline,
        )
    {
        a.first_frame = true;
        status(state, ctx, "Streaming from Xbox");
    }
    if a.ready_since
        .is_some_and(|t| t.elapsed() > Duration::from_secs(15))
        && !a.first_frame
    {
        status(
            state,
            ctx,
            "No decoded video after 15 seconds; disconnect and retry",
        );
        a.ready_since = None;
        *terminal = true;
    }
}
