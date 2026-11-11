use std::path::PathBuf;
/// Non-secret app data only. Credentials use TokenStore, never this directory.
pub fn data_dir() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("XRBOX_DATA_DIR") {
        return Some(PathBuf::from(path));
    }
    directories::ProjectDirs::from("org", "XRBox", "XRBox")
        .map(|dirs| dirs.data_local_dir().to_path_buf())
}
