use std::path::PathBuf;
/// Non-secret app data only. Credentials use TokenStore, never this directory.
pub fn data_dir() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("XRBOX_DATA_DIR") {
        return Some(PathBuf::from(path));
    }
    directories::ProjectDirs::from("org", "XRbox", "XRbox")
        .map(|dirs| dirs.data_local_dir().to_path_buf())
}

use serde::{Deserialize, Serialize};
use xcloud_proto::{
    input::models::GamepadButtons as B,
    preferences::{StreamPreferences, StreamResolutionMode},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Resolution {
    Auto,
    P720,
    #[default]
    P1080,
}
impl Resolution {
    pub fn next(self) -> Self {
        match self {
            Self::Auto => Self::P720,
            Self::P720 => Self::P1080,
            Self::P1080 => Self::Auto,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::P720 => "720p",
            Self::P1080 => "1080p",
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoFit {
    #[default]
    Fit,
    Stretch,
    Zoom,
}
impl VideoFit {
    pub fn next(self) -> Self {
        match self {
            Self::Fit => Self::Stretch,
            Self::Stretch => Self::Zoom,
            Self::Zoom => Self::Fit,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HotkeyBindings {
    pub guide: u16,
    pub menu: u16,
}
impl Default for HotkeyBindings {
    fn default() -> Self {
        let pair = (B::VIEW | B::MENU).bits();
        Self {
            guide: pair,
            menu: pair | (B::LEFT_SHOULDER | B::RIGHT_SHOULDER).bits(),
        }
    }
}
impl HotkeyBindings {
    pub fn validate(self) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.guide != self.menu,
            "Guide and XRbox menu must use different shortcuts"
        );
        for bits in [self.guide, self.menu] {
            let buttons = B::from_bits(bits);
            let known = B::ALL.iter().fold(0, |a, b| a | b.bits()) & !B::NEXUS.bits();
            anyhow::ensure!(
                bits & !known == 0,
                "Physical Xbox/Guide and unknown buttons cannot be used"
            );
            anyhow::ensure!(
                !(buttons & (B::VIEW | B::MENU)).is_empty() && bits.count_ones() >= 2,
                "Use at least two buttons, including Select or Start"
            );
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub fullscreen_on_connect: bool,
    pub video_fit: VideoFit,
    pub resolution: Resolution,
    pub muted: bool,
    pub deadzone_percent: u8,
    pub invert_y: bool,
    pub swap_ab: bool,
    pub remember_signin: bool,
    pub hotkeys: HotkeyBindings,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            fullscreen_on_connect: false,
            video_fit: VideoFit::Fit,
            resolution: Resolution::P1080,
            muted: false,
            deadzone_percent: 15,
            invert_y: false,
            swap_ab: false,
            remember_signin: true,
            hotkeys: Default::default(),
        }
    }
}
impl Settings {
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.deadzone_percent <= 40,
            "Stick deadzone must be between 0 and 40 percent"
        );
        self.hotkeys.validate()
    }
    pub fn apply_controller(&self, input: &mut xcloud_input::source::GamepadInputSource) {
        input.set_settings(xcloud_proto::input::models::ControllerSettings {
            deadzone: f32::from(self.deadzone_percent) / 100.0,
            invert_y: self.invert_y,
            swap_ab: self.swap_ab,
            ..Default::default()
        });
    }
    pub fn stream_preferences(&self) -> StreamPreferences {
        StreamPreferences {
            resolution: match self.resolution {
                Resolution::Auto => StreamResolutionMode::Auto,
                Resolution::P720 => StreamResolutionMode::P720,
                Resolution::P1080 => StreamResolutionMode::P1080,
            },
            ..Default::default()
        }
    }
    pub fn load_at(dir: &std::path::Path) -> anyhow::Result<Self> {
        let bytes = match std::fs::read(dir.join("settings.json")) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(e.into()),
        };
        let settings: Self = serde_json::from_slice(&bytes)?;
        settings.validate()?;
        Ok(settings)
    }
    pub fn save_at(&self, dir: &std::path::Path) -> anyhow::Result<()> {
        self.validate()?;
        std::fs::create_dir_all(dir)?;
        let temp = dir.join("settings.json.tmp");
        let bytes = serde_json::to_vec_pretty(self)?;
        use std::io::Write;
        let mut file = std::fs::File::create(&temp)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temp, dir.join("settings.json"))?;
        Ok(())
    }
}
pub fn button_names(bits: u16) -> String {
    let mut buttons = B::ALL;
    buttons.sort_by_key(|b| {
        if *b == B::VIEW {
            0
        } else if *b == B::MENU {
            1
        } else {
            b.bits()
        }
    });
    buttons
        .iter()
        .filter(|b| bits & b.bits() != 0)
        .map(|b| {
            if *b == B::VIEW {
                "Select"
            } else if *b == B::MENU {
                "Start"
            } else if *b == B::LEFT_SHOULDER {
                "LB"
            } else if *b == B::RIGHT_SHOULDER {
                "RB"
            } else {
                b.name().unwrap_or("Unknown")
            }
        })
        .collect::<Vec<_>>()
        .join(" + ")
}

#[cfg(test)]
mod tests {
    use super::*;
    fn temp() -> PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let unique = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "xrbox-settings-{}-{}-{unique}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }
    #[test]
    fn settings_defaults_roundtrip_and_replacement() {
        let dir = temp();
        assert_eq!(Settings::load_at(&dir).unwrap(), Settings::default());
        let mut settings = Settings {
            muted: true,
            fullscreen_on_connect: true,
            resolution: Resolution::P720,
            ..Default::default()
        };
        settings.save_at(&dir).unwrap();
        assert_eq!(Settings::load_at(&dir).unwrap(), settings);
        settings.invert_y = true;
        settings.save_at(&dir).unwrap();
        assert_eq!(Settings::load_at(&dir).unwrap(), settings);
        assert!(!dir.join("settings.json.tmp").exists());
        std::fs::write(dir.join("settings.json"), "invalid").unwrap();
        assert!(Settings::load_at(&dir).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn invalid_bindings_and_ranges_cannot_replace_valid_settings() {
        let dir = temp();
        let mut settings = Settings::default();
        settings.save_at(&dir).unwrap();
        settings.hotkeys.guide = settings.hotkeys.menu;
        assert!(settings.save_at(&dir).is_err());
        assert_eq!(Settings::load_at(&dir).unwrap(), Settings::default());
        for bits in [B::VIEW.bits(), B::A.bits(), (B::NEXUS | B::VIEW).bits(), 1] {
            assert!(
                HotkeyBindings {
                    guide: bits,
                    ..Default::default()
                }
                .validate()
                .is_err()
            );
        }
        settings = Settings {
            deadzone_percent: 41,
            ..Default::default()
        };
        assert!(settings.validate().is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn stream_settings_are_console_preferences_and_old_files_get_defaults() {
        let dir = temp();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("settings.json"), r#"{"muted":true}"#).unwrap();
        let settings = Settings::load_at(&dir).unwrap();
        assert!(settings.muted);
        assert_eq!(settings.hotkeys, HotkeyBindings::default());
        assert_eq!(
            Settings {
                resolution: Resolution::P720,
                ..Default::default()
            }
            .stream_preferences()
            .resolution,
            StreamResolutionMode::P720
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn controller_preferences_reach_upstream_without_affecting_default_trigger_settings() {
        let settings = Settings {
            deadzone_percent: 30,
            invert_y: true,
            swap_ab: true,
            ..Default::default()
        };
        let mut input = xcloud_input::source::GamepadInputSource::new(Box::new(
            xcloud_input::native::sample_stub,
        ));
        settings.apply_controller(&mut input);
        assert!((input.settings().deadzone - 0.30).abs() < 0.001);
        assert!(input.settings().invert_y && input.settings().swap_ab);
        assert_eq!(input.settings().trigger_sensitivity, 0.5);
    }
    #[test]
    fn filesystem_failure_is_reported_without_overwriting_existing_data() {
        let path = temp();
        std::fs::write(&path, "keep this file").unwrap();
        assert!(Settings::default().save_at(&path).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "keep this file");
        std::fs::remove_file(path).unwrap();
    }
}
