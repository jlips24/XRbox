use std::sync::Arc;
use xcloud_api::{
    auth::MicrosoftAuthService,
    token_store::{KeyringTokenStore, MemoryTokenStore, TokenStore},
    transport::ReqwestTransport,
};

pub fn service(persistent: bool) -> anyhow::Result<Arc<MicrosoftAuthService>> {
    let store: Arc<dyn TokenStore> = if persistent {
        Arc::new(KeyringTokenStore::with_service("xrbox"))
    } else {
        Arc::new(MemoryTokenStore::new())
    };
    Ok(Arc::new(MicrosoftAuthService::new(
        Arc::new(ReqwestTransport::new()?),
        store,
    )))
}

pub async fn home_credentials(
    auth: &MicrosoftAuthService,
) -> anyhow::Result<xcloud_api::auth::StreamTokens> {
    let refresh = auth.token_store().load_refresh_token()?;
    anyhow::ensure!(!refresh.is_empty(), "Sign in first");
    let access = auth.refresh_msa_token(&refresh).await?;
    let tokens = auth.exchange_for_home_tokens(&access).await?;
    anyhow::ensure!(
        !tokens.xhome_host.is_empty() && !tokens.xhome_token.is_empty(),
        "No console streaming credentials returned"
    );
    Ok(tokens)
}
