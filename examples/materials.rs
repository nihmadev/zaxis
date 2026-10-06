//! Custom WGSL materials on widget shapes: a rectangle, a rounded card, an image, a backdrop
//! (frosted glass), slider-driven parameters, a ScrollArea, a scaled group, a second window and
//! a shader that does not compile. `--smoke-test` checks the lifecycle on the real runner.

use zaxis::{
    vec2, App, Color, Context, Frame, ImageFit, ImageHandle, ImageSource, MaterialId, OpenOutcome,
    Params, PresentationMode, Rect, RunOptions, ScrollArea, Sense, Shape, Transform, Ui, Window,
    WindowKey, WindowOptions,
};

#[path = "materials/shaders.rs"]
mod shaders;

static LANDSCAPE: &[u8] = include_bytes!("../assets/images/landscape.jpg");
const SECOND: &str = "second";
const PALETTES: [[Color; 2]; 4] = [
    [Color::rgb(30, 60, 130), Color::rgb(120, 230, 190)],
    [Color::rgb(90, 20, 60), Color::rgb(255, 170, 90)],
    [Color::rgb(20, 70, 50), Color::rgb(220, 240, 120)],
    [Color::rgb(60, 30, 120), Color::rgb(240, 130, 220)],
];

struct Ids {
    aurora: MaterialId,
    frost: MaterialId,
    broken: MaterialId,
}

struct Materials {
    ids: Option<Ids>,
    image: Option<ImageHandle>,
    amount: f32,
    speed: f32,
    animate: bool,
    scale: f32,
    palette: usize,
    smoke: Option<Smoke>,
}

impl Materials {
    fn aurora(
        &self,
        ui: &mut Ui<'_>,
        rect: Rect,
        id: MaterialId,
        key: &str,
        animated: bool,
    ) -> zaxis::MaterialPaint {
        let [a, b] = PALETTES[self.palette];
        ui.material(rect, id)
            .id_source(key)
            .params(
                Params::new()
                    .color("a", a)
                    .color("b", b)
                    .f32("amount", self.amount)
                    .f32("speed", self.speed),
            )
            .animated(animated)
    }

    /// A card with its own hit region: clicking it picks the next palette.
    fn card(
        &mut self,
        ui: &mut Ui<'_>,
        key: &str,
        size: zaxis::Vec2,
        build: impl FnOnce(&Self, &mut Ui<'_>, Rect),
    ) {
        let rect = ui.allocate_space(size);
        build(self, ui, rect);
        if ui.interact(rect, ("hit", key), Sense::CLICK).clicked() {
            self.palette = (self.palette + 1) % PALETTES.len();
        }
    }

    fn controls(&mut self, ui: &mut Ui<'_>, frame: &mut Frame<'_>) {
        ui.horizontal(|ui| {
            ui.slider_labeled(&mut self.amount, 0.0..=1.0, "Amount");
            ui.slider_labeled(&mut self.speed, 0.0..=2.0, "Speed");
        });
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.animate, "Animate");
            ui.slider_labeled(&mut self.scale, 0.5..=1.5, "Scale");
            if ui.button("Second window").clicked() {
                let options =
                    WindowOptions::new("Materials, second window").with_inner_size(320.0, 220.0);
                if frame.open_window(SECOND, options) == OpenOutcome::AlreadyOpen {
                    frame.windows().close(WindowKey::new(SECOND));
                }
            }
        });
    }

    fn gallery(&mut self, ui: &mut Ui<'_>, ids: &Ids) {
        let size = vec2(150.0, 96.0);
        ui.horizontal(|ui| {
            self.card(ui, "still", size, |s, ui, rect| {
                s.aurora(ui, rect, ids.aurora, "still", false).show(ui);
            });
            self.card(ui, "moving", size, |s, ui, rect| {
                s.aurora(ui, rect, ids.aurora, "moving", s.animate).show(ui);
            });
            self.card(ui, "rounded", size, |s, ui, rect| {
                s.aurora(ui, rect, ids.aurora, "rounded", s.animate)
                    .corner_radius(28.0)
                    .show(ui);
            });
            self.card(ui, "image", size, |s, ui, rect| {
                let mut paint = s
                    .aurora(ui, rect, ids.aurora, "image", false)
                    .corner_radius(12.0);
                if let Some(image) = s.image {
                    paint = paint.image(image).fit(ImageFit::Cover).params(
                        Params::new()
                            .color("a", PALETTES[s.palette][0])
                            .color("b", PALETTES[s.palette][1])
                            .f32("amount", s.amount)
                            .f32("use_image", 1.0),
                    );
                }
                paint.show(ui);
            });
        });
    }

    /// Stripes drawn first, then glass over their middle: the shader reads the blurred stripes.
    fn glass(&mut self, ui: &mut Ui<'_>, ids: &Ids) {
        let area = ui.allocate_space(vec2(620.0, 110.0));
        for n in 0..12 {
            let stripe = Rect::from_min_size(
                area.min + vec2(52.0 * n as f32, 0.0),
                vec2(26.0, area.size().y),
            );
            let hue = PALETTES[n % PALETTES.len()][n % 2];
            ui.paint(Shape::rect(stripe, hue));
        }
        let pane = Rect::from_min_size(area.min + vec2(150.0, 14.0), vec2(320.0, 82.0));
        ui.material(pane, ids.frost)
            .corner_radius(18.0)
            .backdrop_blur(10.0)
            .params(
                Params::new()
                    .color("tint", Color::rgb(220, 235, 255))
                    .f32("strength", self.amount),
            )
            .show(ui);
    }

    fn lists(&mut self, ui: &mut Ui<'_>, ids: &Ids) {
        ui.horizontal(|ui| {
            ui.with_width(300.0, |ui| {
                ScrollArea::vertical()
                    .id_source("rows")
                    .max_height(110.0)
                    .show(ui, |ui| {
                        for row in 0..10 {
                            let rect = ui.allocate_space(vec2(280.0, 28.0));
                            self.aurora(ui, rect, ids.aurora, &format!("row{row}"), false)
                                .corner_radius(8.0)
                                .show(ui);
                            ui.add_space(4.0);
                        }
                    });
            });
            // Layout is unchanged; the group is drawn scaled, with its material.
            let corner = ui.allocate_space(vec2(0.0, 0.0)).min;
            let scale = Transform::around(corner + vec2(60.0, 40.0), self.scale, zaxis::Vec2::ZERO);
            ui.visual("scaled", scale, 1.0, |ui| {
                let rect = ui.allocate_space(vec2(120.0, 80.0));
                self.aurora(ui, rect, ids.aurora, "scaled", self.animate)
                    .corner_radius(16.0)
                    .show(ui);
            });
            ui.vertical(|ui| {
                match ui.context().material_error(ids.broken) {
                    Some(error) => ui.muted(format!("{error}")),
                    None => ui.muted("compiled"),
                };
                let rect = ui.allocate_space(vec2(120.0, 40.0));
                // A rejected material draws its fallback; the app keeps running.
                ui.material(rect, ids.broken)
                    .fallback(Color::rgb(150, 40, 40))
                    .corner_radius(8.0)
                    .show(ui);
            });
        });
    }
}

impl App for Materials {
    fn update(&mut self, c: &mut Context, frame: &mut Frame<'_>) {
        if self.ids.is_none() {
            self.ids = Some(Ids {
                aurora: c.register_material(&shaders::aurora()),
                frost: c.register_material(&shaders::frost()),
                broken: c.register_material(&shaders::broken()),
            });
            self.image = c.load_image(ImageSource::from(LANDSCAPE)).ok();
        }
        let ids = self
            .ids
            .as_ref()
            .map(|i| Ids {
                aurora: i.aurora,
                frost: i.frost,
                broken: i.broken,
            })
            .unwrap();
        if frame.window_key().as_str() == SECOND {
            Window::new("Second")
                .default_size(vec2(300.0, 190.0))
                .show(c, |ui| {
                    let rect = ui.allocate_space(vec2(260.0, 140.0));
                    self.aurora(ui, rect, ids.aurora, "second", self.animate)
                        .corner_radius(20.0)
                        .show(ui);
                });
            return;
        }
        Window::new("Materials")
            .default_position(vec2(8.0, 8.0))
            .default_size(vec2(700.0, 560.0))
            .show(c, |ui| {
                self.controls(ui, frame);
                self.gallery(ui, &ids);
                self.glass(ui, &ids);
                self.lists(ui, &ids);
            });
        if let Some(mut smoke) = self.smoke.take() {
            smoke.step(c, frame, self);
            self.smoke = Some(smoke);
        }
    }
}

/// Opens the second window, checks both windows share one material id, toggles animation.
#[derive(Default)]
struct Smoke {
    phase: usize,
    waited: usize,
}

impl Smoke {
    fn step(&mut self, c: &mut Context, frame: &mut Frame<'_>, app: &mut Materials) {
        c.request_repaint();
        self.waited += 1;
        assert!(
            self.waited < 900,
            "smoke test stalled in phase {}",
            self.phase
        );
        let ids = app.ids.as_ref().unwrap();
        match self.phase {
            0 if c.draw_data().materials.len() >= 2 => {
                assert!(
                    c.material_error(ids.aurora).is_none() && c.material_error(ids.frost).is_none()
                );
                assert!(
                    c.material_error(ids.broken).is_some(),
                    "the broken shader is reported"
                );
                app.animate = true;
                self.next();
            }
            1 if c.wants_animation_frame() => {
                app.animate = false;
                app.amount = 0.5;
                self.next();
            }
            2 if !c.wants_animation_frame() => {
                frame.open_window(
                    SECOND,
                    WindowOptions::new("second").with_inner_size(320.0, 220.0),
                );
                self.next();
            }
            3 if frame.stats().windows_open == 2 => {
                println!(
                    "materials smoke passed: {} materials",
                    c.shared_resources().material_count()
                );
                frame.close();
            }
            _ => {}
        }
    }

    fn next(&mut self) {
        self.phase += 1;
        self.waited = 0;
    }
}

fn main() -> Result<(), zaxis::RunError> {
    zaxis::run_with_options(
        Materials {
            ids: None,
            image: None,
            amount: 0.7,
            speed: 0.5,
            animate: false,
            scale: 1.0,
            palette: 0,
            smoke: std::env::args()
                .any(|a| a == "--smoke-test")
                .then(Smoke::default),
        },
        RunOptions {
            window_attributes: zaxis::winit::window::Window::default_attributes()
                .with_title("zaxis — Materials")
                .with_inner_size(zaxis::winit::dpi::LogicalSize::new(740.0, 640.0)),
            presentation_mode: PresentationMode::Vsync,
            ..Default::default()
        },
    )
}
