use crate::config::VideoFit;

#[derive(Default)]
pub struct Display {
    pub fullscreen: bool,
    pending: bool,
}
impl Display {
    pub fn request(&mut self, fullscreen: bool, ctx: &egui::Context) {
        self.fullscreen = fullscreen;
        self.pending = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(fullscreen));
    }
    pub fn observe(&mut self, actual: Option<bool>) {
        if let Some(actual) = actual {
            if self.pending {
                if actual == self.fullscreen {
                    self.pending = false;
                }
            } else {
                self.fullscreen = actual;
            }
        }
    }
    pub fn clean(&self, active: bool, menu: bool, overlay: bool) -> bool {
        self.fullscreen && active && !menu && !overlay
    }
}

pub fn panel(clean: bool) -> egui::CentralPanel {
    if clean {
        egui::CentralPanel::default().frame(
            egui::Frame::new()
                .fill(egui::Color32::BLACK)
                .inner_margin(0.0),
        )
    } else {
        egui::CentralPanel::default()
    }
}

pub fn video_rect(available: egui::Rect, dimensions: (u32, u32), fit: VideoFit) -> egui::Rect {
    if dimensions.0 == 0 || dimensions.1 == 0 {
        return egui::Rect::from_center_size(available.center(), egui::Vec2::ZERO);
    }
    if fit == VideoFit::Stretch {
        return available;
    }
    let sx = available.width() / dimensions.0 as f32;
    let sy = available.height() / dimensions.1 as f32;
    let scale = if fit == VideoFit::Zoom {
        sx.max(sy)
    } else {
        sx.min(sy)
    };
    egui::Rect::from_center_size(
        available.center(),
        egui::vec2(dimensions.0 as f32 * scale, dimensions.1 as f32 * scale),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fullscreen_request_survives_os_transition_and_external_exit_is_observed() {
        let mut display = Display::default();
        display.request(true, &egui::Context::default());
        display.observe(Some(false));
        assert!(display.fullscreen);
        display.observe(Some(true));
        assert!(display.clean(true, false, false));
        assert!(!display.clean(true, true, false));
        assert!(!display.clean(true, false, true));
        assert!(!display.clean(false, false, false));
        display.observe(Some(false));
        assert!(!display.fullscreen);
    }
    #[test]
    fn video_layout_preserves_fits_stretches_and_crops_without_frame_copies() {
        let area = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 1000.0));
        let fit = video_rect(area, (1920, 1080), VideoFit::Fit);
        assert_eq!(fit.width(), 1000.0);
        assert_eq!(fit.height(), 562.5);
        assert_eq!(video_rect(area, (1920, 1080), VideoFit::Stretch), area);
        let zoom = video_rect(area, (1920, 1080), VideoFit::Zoom);
        assert!((zoom.height() - 1000.0).abs() < 0.01);
        assert!(zoom.width() > area.width());
        assert_eq!(
            video_rect(area, (0, 0), VideoFit::Fit).size(),
            egui::Vec2::ZERO
        );
    }
    #[test]
    fn fullscreen_command_is_emitted_and_clean_video_uses_entire_viewport() {
        let ctx = egui::Context::default();
        let mut display = Display::default();
        let area = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 720.0));
        let mut actual = egui::Rect::NOTHING;
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(area),
                ..Default::default()
            },
            |ui| {
                display.request(true, &ctx);
                panel(display.clean(true, false, false)).show(ui, |ui| {
                    actual = ui.available_rect_before_wrap();
                });
            },
        );
        assert_eq!(actual, area);
        assert!(
            output.viewport_output[&egui::ViewportId::ROOT]
                .commands
                .iter()
                .any(|cmd| matches!(cmd, egui::ViewportCommand::Fullscreen(true)))
        );
        output.textures_delta.clear();
    }
}
