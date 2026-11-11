//! Compact, fixed-size header actions. Vector glyphs do not depend on system fonts.
use crate::navigation::Navigation;
use egui::{Color32, Painter, Pos2, Stroke};

#[derive(Clone, Copy)]
pub enum Icon {
    Settings,
    Diagnostics,
    SignOut,
    Menu,
}
impl Icon {
    pub fn paint(self, p: &Painter, center: Pos2, color: Color32) {
        let stroke = Stroke::new(1.5, color);
        let point = |x, y| center + egui::vec2(x, y);
        match self {
            Self::Settings => {
                p.circle_stroke(center, 6.0, stroke);
                p.circle_stroke(center, 2.0, stroke);
                for i in 0..8 {
                    let direction = egui::Vec2::angled(i as f32 * std::f32::consts::TAU / 8.0);
                    p.line_segment([center + direction * 6.0, center + direction * 9.0], stroke);
                }
            }
            Self::Diagnostics => {
                for (x, height) in [(-6.0, 7.0), (0.0, 14.0), (6.0, 10.0)] {
                    p.rect_filled(
                        egui::Rect::from_min_max(point(x - 1.5, 7.0 - height), point(x + 1.5, 7.0)),
                        1.0,
                        color,
                    );
                }
            }
            Self::SignOut => {
                p.line_segment([point(-2.0, -7.0), point(-8.0, -7.0)], stroke);
                p.line_segment([point(-8.0, -7.0), point(-8.0, 7.0)], stroke);
                p.line_segment([point(-8.0, 7.0), point(-2.0, 7.0)], stroke);
                p.line_segment([point(-1.0, 0.0), point(8.0, 0.0)], stroke);
                p.line_segment([point(4.0, -4.0), point(8.0, 0.0)], stroke);
                p.line_segment([point(8.0, 0.0), point(4.0, 4.0)], stroke);
            }
            Self::Menu => {
                for y in [-5.0, 0.0, 5.0] {
                    p.line_segment([point(-8.0, y), point(8.0, y)], stroke);
                }
            }
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    Settings,
    Diagnostics,
    SignOut,
    Menu,
}
#[derive(Clone, Copy)]
pub enum Mode {
    Actions { diagnostics_open: bool },
    Stream,
    Hidden,
}

pub fn header(ui: &mut egui::Ui, nav: &mut Navigation, mode: Mode) -> Option<Action> {
    let mut action = None;
    let wide = ui.available_width() >= 520.0;
    let subtitle = ui.available_width() >= 760.0;
    let brand = |ui: &mut egui::Ui| {
        ui.heading(egui::RichText::new("XRBox").color(Color32::from_rgb(130, 235, 150)));
        if subtitle {
            ui.weak(" /  CONSOLE REMOTE PLAY");
        }
    };
    if wide {
        ui.horizontal(|ui| {
            brand(ui);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                action = actions(ui, nav, mode, true);
            });
        });
    } else {
        ui.horizontal(brand);
        ui.horizontal_wrapped(|ui| {
            action = actions(ui, nav, mode, false);
        });
    }
    action
}
fn actions(ui: &mut egui::Ui, nav: &mut Navigation, mode: Mode, reverse: bool) -> Option<Action> {
    match mode {
        Mode::Hidden => None,
        Mode::Stream => nav
            .toolbar_button(ui, "stream-menu", "Menu", Icon::Menu, true)
            .then_some(Action::Menu),
        Mode::Actions { diagnostics_open } => {
            let mut items = [
                (
                    "settings",
                    "Settings",
                    Icon::Settings,
                    !diagnostics_open,
                    Action::Settings,
                ),
                (
                    "diagnostics",
                    "Diagnostics",
                    Icon::Diagnostics,
                    true,
                    Action::Diagnostics,
                ),
                (
                    "signout",
                    "Sign out",
                    Icon::SignOut,
                    !diagnostics_open,
                    Action::SignOut,
                ),
            ];
            if reverse {
                items.reverse();
            }
            let mut action = None;
            for (id, label, icon, enabled, kind) in items {
                if nav.toolbar_button(ui, id, label, icon, enabled) {
                    action = Some(kind);
                }
            }
            action
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use xcloud_input::source::NativeGamepadSample;
    use xcloud_proto::input::{gamepad::RawGamepadState, models::GamepadButtons as B};
    #[test]
    fn header_actions_do_not_overlap_or_leave_viewport_at_narrow_and_desktop_widths() {
        for width in [240.0, 320.0, 480.0, 520.0, 800.0, 1280.0] {
            let ctx = egui::Context::default();
            ctx.set_visuals(egui::Visuals::dark());
            ctx.style_mut_of(egui::Theme::Dark, |style| {
                style.spacing.item_spacing = egui::vec2(12.0, 12.0);
                style
                    .text_styles
                    .insert(egui::TextStyle::Heading, egui::FontId::proportional(32.0));
            });
            let mut nav = Navigation::default();
            for _ in 0..3 {
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            Pos2::ZERO,
                            egui::vec2(width, 400.0),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        nav.begin(
                            &ctx,
                            &NativeGamepadSample::default(),
                            true,
                            "home",
                            std::time::Duration::ZERO,
                        );
                        header(
                            ui,
                            &mut nav,
                            Mode::Actions {
                                diagnostics_open: false,
                            },
                        );
                    },
                );
                output.textures_delta.clear();
                let controls = nav.test_controls();
                assert_eq!(controls.len(), 3);
                for (i, (_, rect)) in controls.iter().enumerate() {
                    assert!(
                        rect.min.x >= 0.0 && rect.max.x <= width,
                        "toolbar outside width {width}: {rect:?}"
                    );
                    assert_eq!(rect.size(), egui::vec2(76.0, 44.0));
                    for (_, other) in controls.iter().skip(i + 1) {
                        assert!(!rect.intersects(*other), "overlap at width {width}");
                    }
                }
            }
        }
    }
    #[test]
    fn controller_activates_each_toolbar_action_once_and_modal_blocks_signout() {
        for (id, expected) in [
            ("settings", Action::Settings),
            ("diagnostics", Action::Diagnostics),
            ("signout", Action::SignOut),
        ] {
            let ctx = egui::Context::default();
            let mut nav = Navigation::default();
            let mut fired = Vec::new();
            for buttons in [B::NONE, B::A, B::A] {
                let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                    let pad = NativeGamepadSample {
                        connected: true,
                        state: RawGamepadState {
                            buttons,
                            ..Default::default()
                        },
                        ..Default::default()
                    };
                    nav.begin(&ctx, &pad, true, "home", std::time::Duration::ZERO);
                    nav.focus(id);
                    if let Some(action) = header(
                        ui,
                        &mut nav,
                        Mode::Actions {
                            diagnostics_open: false,
                        },
                    ) {
                        fired.push(action);
                    }
                });
                output.textures_delta.clear();
            }
            assert_eq!(fired, vec![expected]);
        }
        let ctx = egui::Context::default();
        let mut nav = Navigation::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            nav.begin(
                &ctx,
                &NativeGamepadSample::default(),
                true,
                "diagnostics",
                std::time::Duration::ZERO,
            );
            header(
                ui,
                &mut nav,
                Mode::Actions {
                    diagnostics_open: true,
                },
            );
        });
        output.textures_delta.clear();
        assert_eq!(
            nav.test_controls()
                .iter()
                .map(|(id, _)| id.as_str())
                .collect::<Vec<_>>(),
            vec!["diagnostics"]
        );
    }
}
