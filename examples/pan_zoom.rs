//! A scene built entirely with public local placement, paint and controls.
#![forbid(unsafe_code)]
#[path = "pan_zoom/scene.rs"]
mod scene;
#[path = "pan_zoom/smoke.rs"]
mod smoke;

use zaxis::{vec2, App, Context, Frame, Padding, PanZoom, PanZoomState, Rect, Root, Theme};

struct Demo {
    camera: PanZoomState,
    scene: scene::Scene,
    enabled: bool,
    image: bool,
    first: bool,
    smoke: bool,
    smoke_stage: usize,
    viewport: Rect,
}
impl Default for Demo {
    fn default() -> Self {
        Self {
            camera: PanZoomState::default(),
            scene: scene::Scene::default(),
            enabled: true,
            image: false,
            first: true,
            smoke: false,
            smoke_stage: 0,
            viewport: Rect::default(),
        }
    }
}
impl Demo {
    fn build(&mut self, c: &mut Context) {
        Root::new().padding(Padding::all(12.0)).show(c, |ui| {
            // Leave camera updates before the scene so they are observed on this pass.
            let size = vec2(
                ui.available_width(),
                (ui.available_height() - 50.0).max(0.0),
            );
            if self.first {
                self.camera.fit(scene::bounds(), size, 28.0, 0.1..=8.0);
                self.first = false;
            }
            ui.horizontal(|ui| {
                if ui.button("−").clicked() {
                    self.camera
                        .zoom_at(self.camera.scale / 1.25, size * 0.5, 0.1..=8.0);
                }
                if ui.button("+").clicked() {
                    self.camera
                        .zoom_at(self.camera.scale * 1.25, size * 0.5, 0.1..=8.0);
                }
                if ui.button("Reset").clicked() {
                    self.camera.reset();
                }
                if ui.button("Fit").clicked() {
                    self.camera.fit(scene::bounds(), size, 28.0, 0.1..=8.0);
                }
                ui.checkbox(&mut self.image, "Image");
                ui.checkbox(&mut self.enabled, "Enabled");
            });
            let out = PanZoom::new("canvas", size)
                .enabled(self.enabled)
                .accessible_label("Scene")
                .show(ui, &mut self.camera, |ui, visible| {
                    self.scene.show(ui, visible, self.image)
                });
            self.viewport = out.viewport;
        });
    }
}
impl App for Demo {
    fn update(&mut self, c: &mut Context, frame: &mut Frame<'_>) {
        c.set_theme(Theme::dark());
        let before = self.camera;
        if self.smoke && self.smoke_stage > 0 {
            smoke::input(c, self.viewport, self.smoke_stage);
        }
        self.build(c);
        if self.smoke {
            smoke::verify(&self.camera, before, self.smoke_stage);
            self.smoke_stage += 1;
            if self.smoke_stage >= 4 {
                frame.close();
            } else {
                c.request_repaint();
            }
        }
    }
}
fn main() -> Result<(), zaxis::RunError> {
    zaxis::run_with_options(
        Demo {
            smoke: std::env::args().any(|a| a == "--smoke-test"),
            ..Default::default()
        },
        zaxis::RunOptions {
            window_attributes: zaxis::winit::window::Window::default_attributes()
                .with_title("zaxis — PanZoom")
                .with_inner_size(zaxis::winit::dpi::LogicalSize::new(1100.0, 760.0)),
            ..Default::default()
        },
    )
}
