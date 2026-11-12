use std::sync::Arc;
#[cfg(not(target_os = "android"))]
use xcloud_api::token_store::KeyringTokenStore;
use xcloud_api::{
    auth::MicrosoftAuthService,
    token_store::{MemoryTokenStore, TokenStore},
    transport::ReqwestTransport,
};

pub fn service(persistent: bool) -> anyhow::Result<Arc<MicrosoftAuthService>> {
    #[cfg(target_os = "android")]
    let store: Arc<dyn TokenStore> = if persistent {
        Arc::new(SessionTokenStore::new(Arc::new(
            crate::android::SecureStore,
        )))
    } else {
        Arc::new(MemoryTokenStore::new())
    };
    #[cfg(not(target_os = "android"))]
    let store: Arc<dyn TokenStore> = if persistent {
        Arc::new(SessionTokenStore::new(Arc::new(
            KeyringTokenStore::with_service("xrbox"),
        )))
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

/// Keep short-lived credentials in memory. Only the refresh token needs to survive
/// relaunch; persisting every intermediate exchange causes unnecessary Keychain access.
struct SessionTokenStore {
    durable: Arc<dyn TokenStore>,
    transient: MemoryTokenStore,
}

impl SessionTokenStore {
    fn new(durable: Arc<dyn TokenStore>) -> Self {
        Self {
            durable,
            transient: MemoryTokenStore::new(),
        }
    }
}

impl TokenStore for SessionTokenStore {
    fn save_msa_token(&self, token: &str) -> xcloud_api::Result<()> {
        self.transient.save_msa_token(token)
    }
    fn load_msa_token(&self) -> xcloud_api::Result<String> {
        self.transient.load_msa_token()
    }
    fn save_refresh_token(&self, token: &str) -> xcloud_api::Result<()> {
        // The underlying KeyringTokenStore caches reads and skips unchanged writes.
        self.durable.save_refresh_token(token)
    }
    fn load_refresh_token(&self) -> xcloud_api::Result<String> {
        self.durable.load_refresh_token()
    }
    fn save_lpt_token(
        &self,
        token: &str,
        expires: Option<chrono::DateTime<chrono::Utc>>,
    ) -> xcloud_api::Result<()> {
        self.transient.save_lpt_token(token, expires)
    }
    fn load_lpt_token(&self) -> xcloud_api::Result<String> {
        self.transient.load_lpt_token()
    }
    fn lpt_token_is_fresh(&self) -> xcloud_api::Result<bool> {
        self.transient.lpt_token_is_fresh()
    }
    fn save_stream_tokens(
        &self,
        tokens: &xcloud_api::token_store::StreamTokens,
    ) -> xcloud_api::Result<()> {
        self.transient.save_stream_tokens(tokens)
    }
    fn load_stream_tokens(&self) -> xcloud_api::Result<xcloud_api::token_store::StreamTokens> {
        self.transient.load_stream_tokens()
    }
    fn clear_all(&self) -> xcloud_api::Result<()> {
        self.transient.clear_all()?;
        self.durable.clear_all()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_refresh_credentials_survive_relaunch_and_sign_out_clears_them() {
        let durable = Arc::new(MemoryTokenStore::new());
        let session = SessionTokenStore::new(durable.clone());
        session.save_refresh_token("synthetic-refresh").unwrap();
        session.save_msa_token("synthetic-access").unwrap();
        session
            .save_lpt_token(
                "synthetic-lpt",
                Some(chrono::Utc::now() + chrono::Duration::hours(1)),
            )
            .unwrap();
        let tokens = xcloud_api::token_store::StreamTokens {
            xhome_token: "synthetic-home".into(),
            ..Default::default()
        };
        session.save_stream_tokens(&tokens).unwrap();
        assert_eq!(session.load_msa_token().unwrap(), "synthetic-access");
        assert!(session.lpt_token_is_fresh().unwrap());
        assert_eq!(
            session.load_stream_tokens().unwrap().xhome_token,
            "synthetic-home"
        );
        assert!(durable.load_msa_token().unwrap().is_empty());
        assert!(durable.load_lpt_token().unwrap().is_empty());
        assert!(durable.load_stream_tokens().unwrap().xhome_token.is_empty());

        let relaunched = SessionTokenStore::new(durable.clone());
        assert_eq!(
            relaunched.load_refresh_token().unwrap(),
            "synthetic-refresh"
        );
        assert!(relaunched.load_msa_token().unwrap().is_empty());
        assert!(relaunched.load_lpt_token().unwrap().is_empty());
        relaunched.save_refresh_token("rotated-refresh").unwrap();
        assert_eq!(durable.load_refresh_token().unwrap(), "rotated-refresh");
        relaunched.clear_all().unwrap();
        assert!(!durable.has_session().unwrap());
    }
}
