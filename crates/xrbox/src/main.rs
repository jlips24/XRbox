mod auth;
mod config;
mod controller;
mod diagnostics;
mod media;
mod navigation;
mod session;

use session::{Command, Session};
use std::sync::atomic::Ordering;

struct App {
    session: Session,
    gpu: xcloud_gpu::VideoRenderer,
    texture: xcloud_gpu::VideoTexture,
    render: eframe::egui_wgpu::RenderState,
    persistent: bool,
    navigation: navigation::Navigation,
    confirm_signout: bool,
    launched: std::time::Instant,
    diagnostics_open: bool,
    copied_at: Option<std::time::Instant>,
    dimensions: (u32, u32),
    frame_count: u64,
    last_frame: Option<std::time::Instant>,
    decode_ms: f64,
    smoke: bool,
    smoke_frames: u32,
}
impl App {
    fn new(cc: &eframe::CreationContext<'_>, smoke: bool) -> Self {
        let render = cc.wgpu_render_state.as_ref().expect("wgpu backend").clone();
        let gpu = xcloud_gpu::VideoRenderer::with_policy(
            &render.device,
            &render.queue,
            xcloud_gpu::UpscalePolicy {
                enabled: false,
                ..Default::default()
            },
        );
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        cc.egui_ctx.style_mut_of(egui::Theme::Dark, |style| {
            style.spacing.item_spacing = egui::vec2(12.0, 12.0);
            style.spacing.button_padding = egui::vec2(22.0, 16.0);
            style
                .text_styles
                .insert(egui::TextStyle::Body, egui::FontId::proportional(18.0));
            style
                .text_styles
                .insert(egui::TextStyle::Heading, egui::FontId::proportional(32.0));
            style.visuals.selection.bg_fill = egui::Color32::from_rgb(35, 100, 62);
            style.visuals.panel_fill = egui::Color32::from_rgb(16, 20, 23);
        });
        let session = Session::new(cc.egui_ctx.clone(), !smoke);
        if smoke {
            // Synthetic only: exercise code layout without reading credentials or contacting Microsoft.
            session.state.lock().code = Some((
                "0O1I-TEST".into(),
                "https://microsoft.com/devicelogin".into(),
            ));
        } else {
            session.send(Command::Restore);
        }
        Self {
            session,
            gpu,
            texture: Default::default(),
            render,
            persistent: true,
            navigation: Default::default(),
            confirm_signout: false,
            launched: std::time::Instant::now(),
            diagnostics_open: false,
            copied_at: None,
            dimensions: (0, 0),
            frame_count: 0,
            last_frame: None,
            decode_ms: 0.0,
            smoke,
            smoke_frames: 0,
        }
    }
}
impl eframe::App for App {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let context = root.ctx().clone();
        let ctx = &context;
        self.session
            .focused
            .store(ctx.input(|i| i.focused), Ordering::Relaxed);
        let snapshot = self.session.state.lock().clone();
        let reader = self.session.video.lock().clone();
        if reader.is_none() {
            self.texture.free(&mut self.render.renderer.write());
            self.dimensions = (0, 0);
            self.frame_count = 0;
        }
        if let Some(reader) = reader {
            let (updated, next) = {
                let mut reader = reader.lock();
                (reader.has_new(), reader.take_latest())
            };
            if updated && next.is_none() {
                self.texture.free(&mut self.render.renderer.write());
                self.dimensions = (0, 0);
                self.last_frame = None;
            }
            if let Some(frame) = next {
                let mut encoder = self
                    .render
                    .device
                    .create_command_encoder(&Default::default());
                match self.gpu.prepare(&mut encoder, &frame) {
                    Ok(_) => {
                        self.render.queue.submit([encoder.finish()]);
                        self.gpu.after_submit();
                        if let Some(view) = self.gpu.display_view() {
                            self.texture.sync(
                                &mut self.render.renderer.write(),
                                &self.render.device,
                                view,
                                self.gpu.output_generation(),
                            );
                        }
                        self.dimensions = (frame.width, frame.height);
                        self.frame_count += 1;
                        self.last_frame = Some(std::time::Instant::now());
                        self.decode_ms = frame.decode_latency().as_secs_f64() * 1000.0;
                    }
                    Err(_) => {
                        self.session.state.lock().status =
                            "GPU presentation failed; disconnect and retry".into();
                    }
                }
            }
        }
        let pad = self.session.controller.sample();
        let menu_open = self.session.controller.menu();
        let menu_visible = !snapshot.active || menu_open;
        let screen = format!(
            "{}-{}-{}",
            if self.confirm_signout {
                "signout"
            } else if snapshot.active {
                if menu_open { "stream-menu" } else { "playing" }
            } else if snapshot.code.is_some() {
                "code"
            } else if snapshot.consoles.is_empty() {
                "welcome"
            } else {
                "consoles"
            },
            snapshot.busy,
            self.diagnostics_open
        );
        let back = self.navigation.begin(
            ctx,
            &pad,
            menu_visible && ctx.input(|i| i.focused),
            &screen,
            self.launched.elapsed(),
        );
        if back {
            if self.diagnostics_open {
                self.diagnostics_open = false;
            } else if self.confirm_signout {
                self.confirm_signout = false;
            } else if snapshot.active {
                self.session.controller.set_menu(false);
            } else if snapshot.busy {
                self.session.send(Command::Disconnect);
            }
        }
        if self.diagnostics_open {
            self.navigation.focus("diagnostics");
        }
        egui::Panel::top("controls").show(root, |ui| {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.heading(
                    egui::RichText::new("XRBox").color(egui::Color32::from_rgb(130, 235, 150)),
                );
                ui.weak(" /  CONSOLE REMOTE PLAY");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.set_max_width(160.0);
                    if !menu_visible {
                        if ui.button("Stream menu").clicked() {
                            self.session.controller.set_menu(true);
                        }
                    } else {
                        if self
                            .navigation
                            .button(ui, "diagnostics", "Diagnostics", true, false)
                        {
                            self.diagnostics_open = !self.diagnostics_open;
                        }
                        if self.navigation.button(
                            ui,
                            "signout",
                            "Sign out",
                            !self.confirm_signout && !self.diagnostics_open,
                            false,
                        ) {
                            self.confirm_signout = true;
                        }
                    }
                });
            });
            ui.horizontal(|ui| {
                if snapshot.busy {
                    ui.spinner();
                }
                ui.label(&snapshot.status);
            });
            ui.add_space(10.0);
        });
        egui::Panel::bottom("controller_hints").show(root, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.strong(if menu_visible {
                    "A  Select     B  Back     D-pad / Left stick  Navigate"
                } else {
                    "Select + Start  Xbox Guide    |    + LB + RB  XRBox menu"
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.weak(if pad.connected {
                        &pad.name
                    } else {
                        "Pair a controller · keyboard and mouse also supported"
                    });
                });
            });
            ui.add_space(8.0);
        });
        egui::CentralPanel::default().show(root, |ui| {
            if self.diagnostics_open { ui.disable(); }
            if snapshot.active && !menu_open {
                self.texture.draw(ui.painter(), ui.available_rect_before_wrap(), self.dimensions.0, self.dimensions.1);
                return;
            }
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(28.0);
                ui.vertical_centered(|ui| {
                    ui.set_max_width(680.0);
                    if self.confirm_signout {
                        ui.heading("Sign out of XRBox?");
                        ui.label("Your saved sign-in will be removed. An active stream will disconnect.");
                        ui.add_space(20.0);
                        if self.navigation.button(ui, "keep-account", "Keep playing", true, true) { self.confirm_signout = false; }
                        if self.navigation.button(ui, "confirm-signout", "Sign out", true, false) {
                            self.confirm_signout = false;
                            self.session.send(Command::SignOut);
                        }
                    } else if snapshot.active {
                        ui.heading("Stream menu");
                        ui.label("Gameplay input is paused while this menu is open.");
                        ui.weak("View/Select + Menu/Start sends Xbox Guide after release.");
                        ui.add_space(20.0);
                        if self.navigation.button(ui, "resume", "Resume game", true, true) { self.session.controller.set_menu(false); }
                        if self.navigation.button(ui, "disconnect", "Disconnect from Xbox", true, false) { self.session.send(Command::Disconnect); }
                    } else if let Some((code, url)) = &snapshot.code {
                        ui.heading("Let's get you signed in");
                        ui.label("Use your Microsoft account. Enter this code in your browser.");
                        ui.add_space(16.0);
                        egui::Frame::group(ui.style()).inner_margin(24.0).show(ui, |ui| {
                            ui.label(egui::RichText::new(code).font(egui::FontId::monospace(48.0)));
                        });
                        let copied = self.copied_at.is_some_and(|t| t.elapsed().as_secs() < 3);
                        if self.navigation.button(ui, "copy", if copied { "Code copied" } else { "Copy sign-in code" }, true, true) {
                            ctx.copy_text(code.clone()); self.copied_at = Some(std::time::Instant::now());
                            ctx.request_repaint_after(std::time::Duration::from_secs(3));
                        }
                        if self.navigation.button(ui, "open-signin", "Open Microsoft sign-in", true, false)
                            && matches!(url.as_str(), "https://microsoft.com/devicelogin" | "https://www.microsoft.com/link" | "https://microsoft.com/link" | "https://www.microsoft.com/devicelogin")
                        { let _ = open::that(url); }
                        ui.weak(url);
                        ui.label("Waiting for approval. We'll take you to your consoles automatically.");
                    } else if !snapshot.consoles.is_empty() {
                        ui.label(egui::RichText::new("JUMP BACK IN").color(egui::Color32::from_rgb(130, 235, 150)));
                        ui.heading("Your consoles");
                        ui.label("Choose your Xbox. Pick up where you left off.");
                        ui.add_space(20.0);
                        for console in &snapshot.consoles {
                            if self.navigation.button(ui, &format!("console-{}", console.server_id),
                                &format!("{}\n{} · {}\nPlay on this console", console.device_name, console.console_type, console.power_state), !snapshot.busy, true) {
                                self.session.send(Command::Connect(console.server_id.clone()));
                            }
                        }
                        if self.navigation.button(ui, "refresh", "Refresh consoles", !snapshot.busy, false) { self.session.send(Command::Discover(self.persistent)); }
                    } else {
                        ui.label(egui::RichText::new("WELCOME HOME").color(egui::Color32::from_rgb(130, 235, 150)));
                        ui.label(egui::RichText::new("Your Xbox.\nA bigger screen.").size(48.0).strong());
                        ui.label("Your console, your games, right here.");
                        ui.add_space(24.0);
                        if self.navigation.button(ui, "signin", "Sign in with Microsoft", !snapshot.busy, true) {
                            self.copied_at = None; self.session.send(Command::SignIn(self.persistent));
                        }
                        if self.navigation.button(ui, "remember", if self.persistent { "Remember sign-in: On" } else { "Remember sign-in: Off" }, !snapshot.busy, false) { self.persistent = !self.persistent; }
                        ui.weak("Saved securely in your OS credential store. Turn Off for session-only sign-in.");
                    }
                    if snapshot.busy && self.navigation.button(ui, "cancel", "Cancel connection", true, snapshot.code.is_none()) {
                        self.session.send(Command::Disconnect);
                    }
                    ui.add_space(24.0);
                    ui.separator();
                    ui.weak(concat!("XRBox ", env!("CARGO_PKG_VERSION"), " · Console Remote Play"));
                });
            });
        });
        egui::Window::new("Diagnostics")
            .open(&mut self.diagnostics_open)
            .show(ctx, |ui| {
                ui.label(format!(
                    "Video: {} × {} · {} presented frames",
                    self.dimensions.0, self.dimensions.1, self.frame_count
                ));
                ui.label(format!(
                    "Last decode: {:.2} ms · source {:.1} FPS",
                    self.decode_ms, snapshot.stats.frames_per_second
                ));
                ui.label(format!(
                    "Bitrate: {} kbps · incomplete frames dropped: {}",
                    snapshot.stats.bitrate_kbps, snapshot.media_health.incomplete_drops
                ));
                ui.label(format!(
                    "Frame stages: {} assembled / {} decoded / {} presented",
                    snapshot.media_health.assembled,
                    snapshot.media_health.decoded,
                    self.frame_count
                ));
                ui.label(format!(
                    "Decoder errors: {} · queue drops: {} · gated: {} · waiting keyframe: {}",
                    snapshot.media_health.decoder_errors,
                    snapshot.media_health.queue_drops,
                    snapshot.media_health.gated,
                    snapshot.media_health.waiting_keyframe
                ));
                if let Some(age) = snapshot.decoded_age_seconds {
                    ui.label(format!("Last decoded frame: {age:.1} seconds ago"));
                }
                ui.label(format!(
                    "Controller: {}",
                    if snapshot.controller.is_empty() {
                        "not detected"
                    } else {
                        &snapshot.controller
                    }
                ));
                ui.label(format!(
                    "Audio device active: {} · dropped chunks: {}",
                    snapshot.audio_playing, snapshot.audio_drops
                ));
                ui.label(format!(
                    "GPU zero-copy: {}",
                    self.gpu.last_frame_zero_copy()
                ));
                ui.label(format!(
                    "Selected route: {}",
                    if snapshot.route.is_empty() {
                        "not yet observed"
                    } else {
                        &snapshot.route
                    }
                ));
                ui.label("End-to-end latency, CPU/memory and Frame power: not measured.");
                if self.last_frame.is_some_and(|t| t.elapsed().as_secs() > 3) {
                    ui.label("Video stalled: no new presented frame for 3 seconds");
                }
            });
        if snapshot.active {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }
        if self.smoke {
            self.smoke_frames += 1;
            if self.smoke_frames == 3 {
                self.session.state.lock().code = None;
            }
            if self.smoke_frames == 5 {
                self.session.state.lock().consoles = vec![xcloud_proto::models::RemoteConsole {
                    device_name: "Synthetic Xbox".into(),
                    server_id: "smoke-only".into(),
                    console_type: "Xbox Series X".into(),
                    power_state: "On".into(),
                    ..Default::default()
                }];
            }
            if self.smoke_frames == 7 {
                self.confirm_signout = true;
            }
            if self.smoke_frames == 8 {
                self.confirm_signout = false;
                self.diagnostics_open = true;
            }
            if self.smoke_frames == 10 {
                println!("XRBox smoke: rendered 10 UI frames");
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            ctx.request_repaint();
        }
    }
    fn on_exit(&mut self) {
        self.session.shutdown();
    }
}
fn main() -> eframe::Result {
    // Upstream may log account identifiers, SDP or response bodies. Only XRBox's controlled
    // stage logger is enabled; raw upstream diagnostics are never written to disk.
    diagnostics::init_logging();
    if std::env::args().any(|s| s == "--auth-probe") {
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let result = runtime.block_on(async {
            auth::service(false)?
                .request_device_code()
                .await
                .map_err(anyhow::Error::from)
        });
        match result {
            Ok(_) => println!(
                "Microsoft device-code authentication initiated successfully (code discarded; no sign-in or persistence)."
            ),
            Err(error) => {
                eprintln!("{}", diagnostics::describe(&error));
                std::process::exit(1);
            }
        }
        return Ok(());
    }
    let smoke = std::env::args().any(|s| s == "--smoke");
    eframe::run_native(
        "XRBox",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Wgpu,
            viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 800.0]),
            ..Default::default()
        },
        Box::new(move |cc| Ok(Box::new(App::new(cc, smoke)))),
    )
}
