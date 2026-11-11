use crate::{
    config::{self, HotkeyBindings, Settings},
    navigation::Navigation,
};
use xcloud_input::source::NativeGamepadSample;
use xcloud_proto::input::models::GamepadButtons as B;

#[derive(Default, Clone, Copy, PartialEq, Debug)]
enum Page {
    #[default]
    General,
    Display,
    Streaming,
    Audio,
    Controller,
    Hotkeys,
}
#[derive(Clone, Copy)]
enum Target {
    Guide,
    Menu,
}
#[derive(Default)]
struct Recorder {
    armed: bool,
    first: Option<B>,
    best: u16,
    previous: B,
}
impl Recorder {
    fn update(&mut self, pad: &NativeGamepadSample, focused: bool) -> Option<anyhow::Result<u16>> {
        if !pad.connected || !focused {
            return Some(Err(anyhow::anyhow!(
                "Recording cancelled: controller disconnected or app lost focus"
            )));
        }
        let held = pad.state.buttons;
        if !self.armed {
            self.armed = held.is_empty();
            self.previous = held;
            return None;
        }
        if self.first.is_none() && !(held & (B::VIEW | B::MENU)).is_empty() {
            self.first = Some(if held.contains(B::VIEW) {
                B::VIEW
            } else {
                B::MENU
            });
            self.best = held.bits();
        }
        if let Some(first) = self.first {
            if !held.contains(first) {
                return Some(Ok(self.best));
            }
            if held.bits().count_ones() >= self.best.count_ones() {
                self.best = held.bits();
            }
        } else if self.previous == B::B && held.is_empty() {
            return Some(Err(anyhow::anyhow!("Recording cancelled")));
        }
        self.previous = held;
        None
    }
}

#[derive(Default)]
pub struct Editor {
    pub open: bool,
    page: Page,
    pub draft: Settings,
    pub message: String,
    capture: Option<(Target, Recorder)>,
    saving: Option<std::thread::JoinHandle<anyhow::Result<Settings>>>,
}
impl Editor {
    pub fn show(&mut self, settings: &Settings) {
        self.draft = settings.clone();
        self.open = true;
        self.page = Page::General;
        self.capture = None;
    }
    pub fn screen(&self) -> String {
        format!("settings-{:?}-{}", self.page, self.capture.is_some())
    }
    pub fn capturing(&self) -> bool {
        self.capture.is_some()
    }
    pub fn back(&mut self) {
        if self.saving.is_some() {
            return;
        }
        if self.capture.take().is_some() {
            self.message = "Recording cancelled".into();
        } else if self.page != Page::General {
            self.page = Page::General;
        } else {
            self.open = false;
        }
    }
    pub fn finish(&mut self) {
        if let Some(job) = self.saving.take() {
            let _ = job.join();
        }
    }
    pub fn poll(&mut self) -> Option<Settings> {
        if !self.saving.as_ref().is_some_and(|job| job.is_finished()) {
            return None;
        }
        let result = self
            .saving
            .take()
            .unwrap()
            .join()
            .unwrap_or_else(|_| Err(anyhow::anyhow!("Settings writer stopped")));
        match result {
            Ok(settings) => {
                self.message =
                    "Settings saved. Resolution changes apply on the next connection.".into();
                Some(settings)
            }
            Err(_) => {
                self.message = "Cannot save settings. Check the app data directory and retry; current settings are unchanged.".into();
                None
            }
        }
    }
    fn save(&mut self, ctx: &egui::Context) {
        if let Err(error) = self.draft.validate() {
            self.message = error.to_string();
            return;
        }
        let settings = self.draft.clone();
        let ctx = ctx.clone();
        self.saving = Some(std::thread::spawn(move || {
            let result = (|| {
                let dir = config::data_dir()
                    .ok_or_else(|| anyhow::anyhow!("App data directory unavailable"))?;
                settings.save_at(&dir)?;
                Ok(settings)
            })();
            ctx.request_repaint();
            result
        }));
    }
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        nav: &mut Navigation,
        pad: &NativeGamepadSample,
        fullscreen: bool,
    ) -> bool {
        let mut toggle_fullscreen = false;
        ui.heading("Settings");
        if let Some((target, recorder)) = &mut self.capture {
            ui.label("Release the selection button, then hold your new combination.");
            ui.label("Include Select or Start and at least one other button. Release the initiating Select/Start to finish.");
            ui.label("Physical Xbox/Guide is reserved for the system. B alone cancels; Escape also cancels.");
            ui.strong(config::button_names(recorder.best));
            if let Some(result) = recorder.update(pad, ui.ctx().input(|i| i.focused)) {
                match result {
                    Ok(bits) => {
                        let mut candidate = self.draft.hotkeys;
                        match target {
                            Target::Guide => candidate.guide = bits,
                            Target::Menu => candidate.menu = bits,
                        }
                        match candidate.validate() {
                            Ok(()) => {
                                self.draft.hotkeys = candidate;
                                self.message = "Shortcut recorded. Apply to save.".into();
                            }
                            Err(error) => self.message = error.to_string(),
                        }
                    }
                    Err(error) => self.message = error.to_string(),
                }
                self.capture = None;
            }
            if ui.button("Cancel recording").clicked() {
                self.capture = None;
            }
            return false;
        }
        let enabled = self.saving.is_none();
        match self.page {
            Page::General => {
                ui.label("Choose a category. A selects or cycles a value; B goes back.");
                for (id, label, page) in [
                    ("display", "Display", Page::Display),
                    ("streaming", "Streaming", Page::Streaming),
                    ("audio", "Audio", Page::Audio),
                    ("controller", "Controller", Page::Controller),
                    ("hotkeys", "Hotkeys", Page::Hotkeys),
                ] {
                    if nav.button(ui, id, label, enabled, true) {
                        self.page = page;
                    }
                }
                ui.weak("Remember sign-in affects future sign-ins and startup restoration. Sign out removes saved credentials.");
                if nav.button(
                    ui,
                    "remember-settings",
                    &format!("Remember sign-in: {}", on(self.draft.remember_signin)),
                    enabled,
                    false,
                ) {
                    self.draft.remember_signin = !self.draft.remember_signin;
                }
            }
            Page::Display => {
                ui.label("Display");
                if nav.button(
                    ui,
                    "fullscreen-now",
                    if fullscreen {
                        "Leave fullscreen"
                    } else {
                        "Enter fullscreen"
                    },
                    enabled,
                    true,
                ) {
                    toggle_fullscreen = true;
                }
                if nav.button(
                    ui,
                    "fullscreen-auto",
                    &format!(
                        "Fullscreen on connection: {}",
                        on(self.draft.fullscreen_on_connect)
                    ),
                    enabled,
                    false,
                ) {
                    self.draft.fullscreen_on_connect = !self.draft.fullscreen_on_connect;
                }
                if nav.button(
                    ui,
                    "video-fit",
                    &format!("Picture size: {:?}", self.draft.video_fit),
                    enabled,
                    false,
                ) {
                    self.draft.video_fit = self.draft.video_fit.next();
                }
                ui.weak("Fit keeps the whole picture. Stretch fills the screen. Zoom crops edges. F11 toggles fullscreen; Escape leaves it during gameplay.");
            }
            Page::Streaming => {
                ui.label("Streaming");
                if nav.button(
                    ui,
                    "resolution",
                    &format!("Requested resolution: {}", self.draft.resolution.label()),
                    enabled,
                    true,
                ) {
                    self.draft.resolution = self.draft.resolution.next();
                }
                ui.weak("Applies to the next connection. The console decides the actual resolution. Hardware decoding remains required on macOS.");
            }
            Page::Audio => {
                if nav.button(
                    ui,
                    "mute",
                    &format!("Mute game audio: {}", on(self.draft.muted)),
                    enabled,
                    true,
                ) {
                    self.draft.muted = !self.draft.muted;
                }
                ui.weak("Applies after saving, including during an active stream. Uses the system's current audio output.");
            }
            Page::Controller => {
                if nav.button(
                    ui,
                    "deadzone",
                    &format!("Stick deadzone: {}%", self.draft.deadzone_percent),
                    enabled,
                    true,
                ) {
                    self.draft.deadzone_percent = (self.draft.deadzone_percent / 5 * 5 + 5) % 45;
                }
                if nav.button(
                    ui,
                    "invert-y",
                    &format!("Invert stick Y: {}", on(self.draft.invert_y)),
                    enabled,
                    false,
                ) {
                    self.draft.invert_y = !self.draft.invert_y;
                }
                if nav.button(
                    ui,
                    "swap-ab",
                    &format!("Swap gameplay A/B: {}", on(self.draft.swap_ab)),
                    enabled,
                    false,
                ) {
                    self.draft.swap_ab = !self.draft.swap_ab;
                }
                ui.weak("Changes gameplay input only. Menu A selects and B goes back.");
            }
            Page::Hotkeys => {
                if nav.button(
                    ui,
                    "record-guide",
                    &format!(
                        "Xbox Guide: {}",
                        config::button_names(self.draft.hotkeys.guide)
                    ),
                    enabled,
                    true,
                ) {
                    self.capture = Some((Target::Guide, Recorder::default()));
                }
                if nav.button(
                    ui,
                    "record-menu",
                    &format!(
                        "XRBox menu: {}",
                        config::button_names(self.draft.hotkeys.menu)
                    ),
                    enabled,
                    false,
                ) {
                    self.capture = Some((Target::Menu, Recorder::default()));
                }
                if nav.button(
                    ui,
                    "hotkeys-reset",
                    "Reset hotkeys to defaults",
                    enabled,
                    false,
                ) {
                    self.draft.hotkeys = HotkeyBindings::default();
                }
                ui.weak("Select/Start are deferred until the first pressed is released. Other controls remain live; include Select or Start in each combination. If both match, the most specific wins; ties choose XRBox menu.");
            }
        }
        ui.add_space(12.0);
        if !self.message.is_empty() {
            ui.label(&self.message);
        }
        if nav.button(
            ui,
            "settings-apply",
            if enabled {
                "Apply and save settings"
            } else {
                "Saving settings…"
            },
            enabled,
            false,
        ) {
            self.save(ui.ctx());
        }
        if nav.button(
            ui,
            "settings-back",
            if self.page == Page::General {
                "Close settings (discard unapplied changes)"
            } else {
                "Back to categories"
            },
            enabled,
            false,
        ) {
            self.back();
        }
        toggle_fullscreen
    }
}
fn on(value: bool) -> &'static str {
    if value { "On" } else { "Off" }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pad(buttons: B) -> NativeGamepadSample {
        NativeGamepadSample {
            connected: true,
            state: xcloud_proto::input::gamepad::RawGamepadState {
                buttons,
                ..Default::default()
            },
            ..Default::default()
        }
    }
    #[test]
    fn recording_ignores_activation_then_captures_chord_on_first_release() {
        let mut r = Recorder::default();
        assert!(r.update(&pad(B::A), true).is_none());
        r.update(&pad(B::NONE), true);
        r.update(&pad(B::VIEW), true);
        r.update(&pad(B::VIEW | B::X), true);
        assert!(r.update(&pad(B::VIEW), true).is_none());
        assert_eq!(
            r.update(&pad(B::NONE), true).unwrap().unwrap(),
            (B::VIEW | B::X).bits()
        );
    }
    #[test]
    fn recording_cancels_on_focus_disconnect_or_b_release() {
        let mut r = Recorder::default();
        assert!(r.update(&pad(B::NONE), false).unwrap().is_err());
        assert!(
            r.update(&NativeGamepadSample::default(), true)
                .unwrap()
                .is_err()
        );
        r.update(&pad(B::NONE), true);
        r.update(&pad(B::B), true);
        assert!(r.update(&pad(B::NONE), true).unwrap().is_err());
    }
    #[test]
    fn back_closes_categories_without_applying_draft_and_defaults_remain_usable() {
        let settings = Settings::default();
        let mut editor = Editor::default();
        editor.show(&settings);
        editor.draft.muted = true;
        editor.page = Page::Hotkeys;
        editor.back();
        assert!(editor.open);
        editor.back();
        assert!(!editor.open);
        assert!(!settings.muted);
        editor.show(&settings);
        assert!(!editor.draft.muted);
    }
    #[test]
    fn every_settings_category_is_controller_activatable_in_headless_layout() {
        for page in [
            Page::General,
            Page::Display,
            Page::Streaming,
            Page::Audio,
            Page::Controller,
            Page::Hotkeys,
        ] {
            let ctx = egui::Context::default();
            let mut nav = Navigation::default();
            let mut editor = Editor::default();
            editor.show(&Settings::default());
            editor.page = page;
            let mut toggled = false;
            for buttons in [B::NONE, B::A] {
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        focused: true,
                        ..Default::default()
                    },
                    |ui| {
                        nav.begin(
                            &ctx,
                            &pad(buttons),
                            true,
                            &editor.screen(),
                            std::time::Duration::ZERO,
                        );
                        toggled |= editor.ui(ui, &mut nav, &pad(buttons), false);
                    },
                );
                output.textures_delta.clear();
            }
            match page {
                Page::General => assert_eq!(editor.page, Page::Display),
                Page::Display => assert!(toggled),
                Page::Streaming => assert_eq!(editor.draft.resolution, config::Resolution::Auto),
                Page::Audio => assert!(editor.draft.muted),
                Page::Controller => assert_eq!(editor.draft.deadzone_percent, 20),
                Page::Hotkeys => assert!(editor.capturing()),
            }
        }
    }
    #[test]
    fn completed_save_returns_settings_once_and_errors_do_not_apply_or_expose_raw_paths() {
        for success in [true, false] {
            let mut editor = Editor::default();
            editor.show(&Settings::default());
            editor.draft.muted = true;
            let (release, wait) = std::sync::mpsc::channel();
            editor.saving = Some(std::thread::spawn(move || {
                wait.recv().unwrap();
                if success {
                    Ok(Settings {
                        muted: true,
                        ..Default::default()
                    })
                } else {
                    Err(anyhow::anyhow!("private/raw/path"))
                }
            }));
            editor.back();
            assert!(
                editor.open,
                "save in progress must finish before closing settings"
            );
            release.send(()).unwrap();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
            while !editor.saving.as_ref().unwrap().is_finished() {
                assert!(std::time::Instant::now() < deadline);
                std::thread::yield_now();
            }
            let applied = editor.poll();
            assert_eq!(applied.is_some(), success);
            if let Some(settings) = applied {
                assert!(settings.muted);
            }
            assert!(!editor.message.contains("private/raw/path"));
            assert!(editor.poll().is_none());
        }
    }
}
