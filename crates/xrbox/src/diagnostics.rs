/// Never include an upstream response body, account identifier or token in app logs.
pub fn describe(error: &anyhow::Error) -> String {
    use xcloud_api::Error;
    match error.downcast_ref::<Error>() {
        Some(Error::Status{status,..}) => format!("Xbox service returned HTTP {status}; check account and console availability"),
        Some(Error::TokenStore(_)) => "Secure credential store unavailable. Choose session-only sign-in or unlock the OS credential store.".into(),
        Some(Error::Timeout(_)) => "Network operation timed out; check connectivity and retry".into(),
        Some(Error::Http(_)) => "Network request failed; check connectivity and retry".into(),
        Some(Error::Auth(message)) => {
            let reason = ["authorization_declined","expired_token","invalid_grant","invalid_client","device code expired"]
                .into_iter().find(|known|message.contains(known)).unwrap_or("authentication rejected");
            format!("Microsoft sign-in: {reason}; sign in again")
        }
        _ => "Operation failed. Sign in first, check network and console Remote Features, then retry.".into(),
    }
}
pub fn init_logging() {
    use std::io::Write;
    let file = crate::config::data_dir().and_then(|dir| {
        std::fs::create_dir_all(&dir).ok()?;
        let path = dir.join("diagnostics.log");
        if std::fs::metadata(&path).is_ok_and(|m| m.len() > 1024 * 1024) {
            let _ = std::fs::rename(&path, dir.join("diagnostics.previous.log"));
        }
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .ok()?;
        eprintln!("XRBox sanitized log: {}", path.display());
        Some(std::sync::Arc::new(file))
    });
    tracing_subscriber::fmt()
        .with_env_filter("off,xrbox=info")
        .with_ansi(false)
        .with_writer(move || -> Box<dyn Write + Send> {
            file.as_ref()
                .and_then(|f| f.try_clone().ok())
                .map(|f| Box::new(f) as Box<dyn Write + Send>)
                .unwrap_or_else(|| Box::new(std::io::stderr()))
        })
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn response_bodies_and_network_urls_are_not_exposed() {
        for error in [
            xcloud_api::Error::Status {
                status: 401,
                body: "secret_token".into(),
            },
            xcloud_api::Error::Http("https://private/?token=secret_token".into()),
            xcloud_api::Error::Auth("invalid_grant secret_token".into()),
        ] {
            let text = describe(&error.into());
            assert!(!text.contains("secret_token"));
            assert!(!text.contains("private"));
        }
    }
}
