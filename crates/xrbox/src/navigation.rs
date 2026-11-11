//! Spatial focus and controller repeat, independent of the streaming protocol.
use std::time::Duration;
use xcloud_input::source::NativeGamepadSample;
use xcloud_proto::input::models::GamepadButtons as B;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}
#[derive(Default)]
struct Input {
    previous: B,
    direction: Option<Direction>,
    repeat_at: Duration,
    armed: bool,
}
impl Input {
    fn update(
        &mut self,
        sample: &NativeGamepadSample,
        enabled: bool,
        now: Duration,
    ) -> (Option<Direction>, bool, bool) {
        let buttons = sample.state.buttons;
        let x = sample.state.left_thumb.x;
        let y = sample.state.left_thumb.y;
        if !enabled || !sample.connected {
            self.armed = false;
            self.direction = None;
            self.previous = buttons;
            return (None, false, false);
        }
        // Reconnect, focus return and screen transitions never act on held buttons.
        if !self.armed {
            self.armed = buttons.is_empty() && x.abs() < 0.35 && y.abs() < 0.35;
            self.previous = buttons;
            return (None, false, false);
        }
        let direction = if buttons.contains(B::DPAD_UP) {
            Some(Direction::Up)
        } else if buttons.contains(B::DPAD_DOWN) {
            Some(Direction::Down)
        } else if buttons.contains(B::DPAD_LEFT) {
            Some(Direction::Left)
        } else if buttons.contains(B::DPAD_RIGHT) {
            Some(Direction::Right)
        } else if x.abs().max(y.abs()) < 0.6 {
            None
        } else if x.abs() > y.abs() {
            Some(if x > 0.0 {
                Direction::Right
            } else {
                Direction::Left
            })
        } else {
            Some(if y > 0.0 {
                Direction::Up
            } else {
                Direction::Down
            })
        };
        let movement = if direction != self.direction {
            self.repeat_at = now + Duration::from_millis(400);
            direction
        } else if direction.is_some() && now >= self.repeat_at {
            self.repeat_at = now + Duration::from_millis(150);
            direction
        } else {
            None
        };
        self.direction = direction;
        let select = buttons.contains(B::A) && !self.previous.contains(B::A);
        let back = buttons.contains(B::B) && !self.previous.contains(B::B);
        self.previous = buttons;
        (movement, select, back)
    }
}

#[derive(Default)]
pub struct Navigation {
    input: Input,
    controls: Vec<(String, egui::Rect)>,
    focus: Option<String>,
    scroll_focus: Option<String>,
    screen: String,
    select: bool,
}
impl Navigation {
    pub fn begin(
        &mut self,
        ctx: &egui::Context,
        sample: &NativeGamepadSample,
        enabled: bool,
        screen: &str,
        now: Duration,
    ) -> bool {
        if self.screen != screen {
            self.screen = screen.into();
            self.focus = None;
            self.scroll_focus = None;
            self.input.armed = false;
            self.controls.clear();
        }
        let (direction, select, back) = self.input.update(sample, enabled, now);
        self.select = select
            || (enabled
                && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)));
        let keyboard = if enabled {
            ctx.input(|i| {
                if i.key_pressed(egui::Key::ArrowUp) {
                    Some(Direction::Up)
                } else if i.key_pressed(egui::Key::ArrowDown) {
                    Some(Direction::Down)
                } else if i.key_pressed(egui::Key::ArrowLeft) {
                    Some(Direction::Left)
                } else if i.key_pressed(egui::Key::ArrowRight) {
                    Some(Direction::Right)
                } else {
                    None
                }
            })
        } else {
            None
        };
        if let Some(direction) = direction.or(keyboard)
            && let Some((_, origin)) = self
                .controls
                .iter()
                .find(|(id, _)| Some(id) == self.focus.as_ref())
        {
            let origin = origin.center();
            let target = self
                .controls
                .iter()
                .filter_map(|(id, rect)| {
                    let delta = rect.center() - origin;
                    let (along, across) = match direction {
                        Direction::Up => (-delta.y, delta.x.abs()),
                        Direction::Down => (delta.y, delta.x.abs()),
                        Direction::Left => (-delta.x, delta.y.abs()),
                        Direction::Right => (delta.x, delta.y.abs()),
                    };
                    (along > 1.0).then_some((id, along + across * 3.0))
                })
                .min_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((id, _)) = target {
                let id = id.clone();
                self.focus(&id);
            }
        }
        self.controls.clear();
        // Idle menus are event-driven; only held navigation needs repeat repaints.
        if enabled && self.input.direction.is_some() {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
        back || (enabled && ctx.input(|i| i.key_pressed(egui::Key::Escape)))
    }
    pub fn focus(&mut self, id: &str) {
        if self.focus.as_deref() != Some(id) {
            self.focus = Some(id.into());
            self.scroll_focus = Some(id.into());
        }
    }
    pub fn button(
        &mut self,
        ui: &mut egui::Ui,
        id: &str,
        text: &str,
        enabled: bool,
        primary: bool,
    ) -> bool {
        let enabled = enabled && ui.is_enabled();
        if enabled && primary && self.focus.is_none() {
            self.focus(id);
        }
        let focused = self.focus.as_deref() == Some(id);
        let button = egui::Button::new(egui::RichText::new(text).size(19.0))
            .min_size(egui::vec2(
                ui.available_width().min(520.0),
                if primary { 112.0 } else { 52.0 },
            ))
            .fill(if primary {
                egui::Color32::from_rgb(25, 91, 46)
            } else {
                egui::Color32::from_rgb(35, 40, 45)
            })
            .stroke(egui::Stroke::new(
                if focused { 3.0 } else { 1.0 },
                if focused {
                    egui::Color32::from_rgb(155, 255, 175)
                } else {
                    egui::Color32::from_rgb(65, 75, 80)
                },
            ));
        let response = ui.push_id(id, |ui| ui.add_enabled(enabled, button)).inner;
        if enabled {
            self.controls.push((id.into(), response.rect));
        }
        if enabled && self.scroll_focus.as_deref() == Some(id) {
            // Reveal newly selected controls once; idle hover must not fight scrolling.
            response.scroll_to_me(None);
            self.scroll_focus = None;
        }
        let clicked = response.clicked() || (enabled && focused && self.select);
        if clicked {
            self.focus(id);
            self.select = false;
            ui.ctx().request_repaint();
        }
        clicked
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample(buttons: B) -> NativeGamepadSample {
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
    fn repeat_edges_and_focus_gating() {
        let mut input = Input::default();
        let t = Duration::ZERO;
        assert!(!input.update(&sample(B::A), true, t).1);
        input.update(&sample(B::NONE), true, t);
        assert!(input.update(&sample(B::A), true, t).1);
        assert!(!input.update(&sample(B::A), true, t).1);
        assert_eq!(
            input.update(&sample(B::DPAD_DOWN), true, t).0,
            Some(Direction::Down)
        );
        assert_eq!(
            input
                .update(&sample(B::DPAD_DOWN), true, Duration::from_millis(399))
                .0,
            None
        );
        assert_eq!(
            input
                .update(&sample(B::DPAD_DOWN), true, Duration::from_millis(400))
                .0,
            Some(Direction::Down)
        );
        assert_eq!(
            input
                .update(&sample(B::DPAD_DOWN), true, Duration::from_millis(450))
                .0,
            None
        );
        assert!(!input.update(&sample(B::B), false, t).2);
        assert!(!input.update(&sample(B::B), true, t).2);
        input.update(&sample(B::NONE), true, t);
        assert!(input.update(&sample(B::B), true, t).2);
        assert_eq!(
            input.update(&NativeGamepadSample::default(), true, t).0,
            None
        );
    }
    #[test]
    fn stick_deadzone_and_y_up_mapping() {
        let mut input = Input::default();
        let mut s = sample(B::NONE);
        input.update(&s, true, Duration::ZERO);
        s.state.left_thumb.y = 0.4;
        assert_eq!(input.update(&s, true, Duration::ZERO).0, None);
        s.state.left_thumb.y = 0.8;
        assert_eq!(
            input.update(&s, true, Duration::ZERO).0,
            Some(Direction::Up)
        );
    }
    #[test]
    fn spatial_navigation_uses_visible_enabled_tiles() {
        let mut nav = Navigation {
            screen: "home".into(),
            focus: Some("first".into()),
            ..Default::default()
        };
        nav.input.armed = true;
        nav.controls = vec![
            (
                "first".into(),
                egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(100.0, 50.0)),
            ),
            (
                "below".into(),
                egui::Rect::from_min_size(egui::pos2(0.0, 80.0), egui::vec2(100.0, 50.0)),
            ),
            (
                "right".into(),
                egui::Rect::from_min_size(egui::pos2(150.0, 0.0), egui::vec2(100.0, 50.0)),
            ),
        ];
        nav.begin(
            &egui::Context::default(),
            &sample(B::DPAD_RIGHT),
            true,
            "home",
            Duration::ZERO,
        );
        assert_eq!(nav.focus.as_deref(), Some("right"));
        nav.begin(
            &egui::Context::default(),
            &sample(B::A),
            true,
            "new-screen",
            Duration::ZERO,
        );
        assert!(nav.focus.is_none());
        assert!(!nav.select); // held A cannot activate a different screen
    }

    #[test]
    fn controller_a_activates_focused_tile_once_in_real_egui_layout() {
        let ctx = egui::Context::default();
        let mut nav = Navigation::default();
        let mut clicks = 0;
        for buttons in [B::NONE, B::A, B::A] {
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                nav.begin(&ctx, &sample(buttons), true, "home", Duration::ZERO);
                if nav.button(ui, "play", "Play", true, true) {
                    clicks += 1;
                }
                assert!(!nav.button(ui, "disabled", "Disabled", false, false));
            });
            // Headless layout test deliberately has no texture renderer.
            output.textures_delta.clear();
        }
        assert_eq!(clicks, 1);
        assert_eq!(nav.controls.len(), 1);
    }
    #[test]
    fn disabled_modal_background_cannot_be_selected() {
        let ctx = egui::Context::default();
        let mut nav = Navigation::default();
        for buttons in [B::NONE, B::A] {
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                nav.begin(&ctx, &sample(buttons), true, "modal", Duration::ZERO);
                ui.disable();
                assert!(!nav.button(ui, "background-play", "Play", true, true));
            });
            output.textures_delta.clear();
            assert!(nav.controls.is_empty());
        }
    }
    #[test]
    fn keyboard_enter_selects_one_tile_and_escape_goes_back() {
        let ctx = egui::Context::default();
        let mut nav = Navigation::default();
        let mut clicks = 0;
        for key in [None, Some(egui::Key::Enter), Some(egui::Key::Escape)] {
            let mut input = egui::RawInput::default();
            if let Some(key) = key {
                input.events.push(egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                });
            }
            let mut output = ctx.run_ui(input, |ui| {
                let back = nav.begin(&ctx, &sample(B::NONE), true, "home", Duration::ZERO);
                assert_eq!(back, key == Some(egui::Key::Escape));
                if nav.button(ui, "play", "Play", true, true) {
                    clicks += 1;
                }
                if nav.button(ui, "other", "Other", true, false) {
                    clicks += 1;
                }
            });
            output.textures_delta.clear();
        }
        assert_eq!(clicks, 1);
    }
    #[test]
    fn stationary_hover_preserves_user_scroll_without_recentering_focus_every_frame() {
        let ctx = egui::Context::default();
        ctx.style_mut_of(egui::Theme::Dark, |s| {
            s.scroll_animation = egui::style::ScrollAnimation::none()
        });
        ctx.set_visuals(egui::Visuals::dark());
        let mut nav = Navigation::default();
        for frame in 0..8 {
            let mut offset = 0.0;
            let mut input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 400.0),
                )),
                ..Default::default()
            };
            input
                .events
                .push(egui::Event::PointerMoved(egui::pos2(40.0, 200.0)));
            let mut output = ctx.run_ui(input, |ui| {
                nav.begin(&ctx, &sample(B::NONE), true, "home", Duration::ZERO);
                let area = egui::ScrollArea::vertical()
                    .max_height(260.0)
                    .vertical_scroll_offset(200.0)
                    .show(ui, |ui| {
                        nav.button(ui, "signin", "Sign in with Microsoft", true, true);
                        ui.add_space(250.0);
                        nav.button(ui, "other", "Another action", true, false);
                        ui.add_space(500.0);
                        ui.weak("XRBox · Console Remote Play");
                    });
                offset = area.state.clamped_offset().y;
            });
            output.textures_delta.clear();
            // Initial focus reveal may be applied on the following frame.
            if frame >= 2 {
                assert!(
                    (offset - 200.0).abs() < 0.1,
                    "stationary hover moved scroll to {offset}"
                );
            }
        }
    }
    #[test]
    fn changed_focus_reveals_offscreen_tile_but_repeated_focus_does_not_lock_scroll() {
        let ctx = egui::Context::default();
        ctx.set_visuals(egui::Visuals::dark());
        ctx.style_mut_of(egui::Theme::Dark, |s| {
            s.scroll_animation = egui::style::ScrollAnimation::none()
        });
        let mut nav = Navigation::default();
        let mut revealed = false;
        for frame in 0..10 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 400.0),
                )),
                time: Some(frame as f64 / 30.0),
                ..Default::default()
            };
            let mut offset = 0.0;
            let mut output = ctx.run_ui(input, |ui| {
                nav.begin(&ctx, &sample(B::NONE), true, "consoles", Duration::ZERO);
                if frame >= 2 {
                    nav.focus("second");
                }
                let mut area = egui::ScrollArea::vertical().max_height(260.0);
                if frame >= 6 {
                    area = area.vertical_scroll_offset(200.0);
                }
                offset = area
                    .show(ui, |ui| {
                        nav.button(ui, "first", "First console", true, true);
                        ui.add_space(600.0);
                        nav.button(ui, "second", "Second console", true, true);
                        ui.add_space(300.0);
                        ui.label("End");
                    })
                    .state
                    .clamped_offset()
                    .y;
            });
            output.textures_delta.clear();
            if (2..6).contains(&frame) {
                revealed |= offset > 400.0;
            }
            if frame >= 8 {
                assert!(
                    (offset - 200.0).abs() < 0.1,
                    "repeated focus recentered to {offset}"
                );
            }
        }
        assert!(
            revealed,
            "new focus must still reveal the offscreen console tile"
        );
    }
}
