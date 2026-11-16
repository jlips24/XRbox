//! Cancellable console-only authentication jobs; persistence policy stays in auth.
use super::Snapshot;
use crate::auth;
use parking_lot::Mutex;
use std::sync::Arc;
use xcloud_api::{
    auth::MicrosoftAuthService, client::XCloudApiClient, transport::ReqwestTransport,
};
use xcloud_proto::models::RemoteConsole;
pub(super) enum Outcome {
    NoSavedSession,
    Consoles(Vec<RemoteConsole>),
    Connect(String, Arc<XCloudApiClient>, String),
}
pub(super) async fn discover(auth: &MicrosoftAuthService) -> anyhow::Result<Outcome> {
    let tokens = auth::home_credentials(auth).await?;
    let api = XCloudApiClient::new(Arc::new(ReqwestTransport::new()?));
    api.update_credentials(&tokens.xhome_host, &tokens.xhome_token);
    Ok(Outcome::Consoles(api.get_consoles().await?.results))
}

pub(super) async fn restore(auth: &MicrosoftAuthService) -> anyhow::Result<Outcome> {
    if !auth.has_stored_session()? {
        return Ok(Outcome::NoSavedSession);
    }
    discover(auth).await
}

pub(super) async fn sign_in(
    auth: Arc<MicrosoftAuthService>,
    state: Arc<Mutex<Snapshot>>,
    notify: crate::Notify,
) -> anyhow::Result<Outcome> {
    let code = auth.request_device_code().await?;
    state.lock().code = Some((code.user_code.clone(), code.verification_uri.clone()));
    notify.notify();
    auth.poll_for_msa_token(
        &code.device_code,
        code.interval.max(1) as u64,
        code.expires_in.max(1) as u64,
    )
    .await?;
    discover(&auth).await
}
pub(super) async fn connect(
    auth: Arc<MicrosoftAuthService>,
    id: String,
) -> anyhow::Result<Outcome> {
    let tokens = auth::home_credentials(&auth).await?;
    let api = Arc::new(XCloudApiClient::new(Arc::new(ReqwestTransport::new()?)));
    api.update_credentials(&tokens.xhome_host, &tokens.xhome_token);
    let refresh = auth.token_store().load_refresh_token()?;
    let lpt = auth.fetch_lpt(&refresh).await?;
    Ok(Outcome::Connect(id, api, lpt))
}
