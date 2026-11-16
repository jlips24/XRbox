use super::*;
use super::{
    active::streaming_ready,
    auth_jobs::{Outcome, restore},
    worker::run_sampler,
};
use xcloud_api::auth::MicrosoftAuthService;
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
#[cfg(test)]
mod readiness_and_shutdown {
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

#[tokio::test]
async fn cancelling_an_inflight_job_waits_for_abort_and_discards_its_handle() {
    let (started, ready) = tokio::sync::oneshot::channel();
    let handle = tokio::spawn(async move {
        let _ = started.send(());
        std::future::pending::<anyhow::Result<Outcome>>().await
    });
    let aborted = handle.abort_handle();
    let mut job = Some(handle);
    ready.await.unwrap();
    super::worker::cancel(&mut job).await;
    assert!(job.is_none());
    assert!(aborted.is_finished());
    super::worker::cancel(&mut job).await;
}
