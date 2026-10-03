use zaxis::{
    vec2, App, Color, Context, Frame, Image, ImageFit, ImageHandle, ImageSource, ImageState,
    PresentationMode, RunOptions, ScrollArea, Window,
};

static PNG: &[u8] = include_bytes!("../assets/images/alpha.png");
static JPEG: &[u8] = include_bytes!("../assets/images/landscape.jpg");
static WEBP: &[u8] = include_bytes!("../assets/images/landscape.webp");
static SVG: &[u8] = include_bytes!("../assets/images/icon.svg");
struct Images {
    raw: Option<ImageHandle>,
    generation: u8,
    smoke: bool,
    phase: usize,
    width: f32,
}
impl App for Images {
    fn update(&mut self, c: &mut Context, frame: &mut Frame<'_>) {
        let raw = *self.raw.get_or_insert_with(|| {
            c.load_image(ImageSource::rgba(
                [2, 2],
                vec![
                    255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 96,
                ],
            ))
            .unwrap()
        });
        Window::new("Images")
            .default_position(vec2(8.0, 8.0))
            .default_size(vec2(1020.0, 800.0))
            .show(c, |ui| {
                ui.label("Local images · built-in async decoding");
                ui.slider_labeled(&mut self.width, 80.0..=480.0, "SVG width");
                ui.horizontal(|ui| {
                    for width in [80.0, 240.0, 480.0] {
                        if ui.button(format!("SVG {width:.0} px")).clicked() {
                            self.width = width;
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.add(Image::new(SVG).size(vec2(self.width, self.width * 2.0 / 3.0)));
                    ui.add(
                        Image::new(PNG)
                            .size(vec2(130.0, 130.0))
                            .corner_radius(24.0)
                            .tint(Color::rgb(180, 230, 255))
                            .opacity(0.8),
                    );
                    ui.add(
                        Image::new(raw)
                            .size(vec2(110.0, 110.0))
                            .filter(zaxis::TextureFilter::Nearest)
                            .fit(ImageFit::Stretch)
                            .corner_radius(16.0),
                    );
                });
                if ui.button("Update RGBA").clicked() {
                    self.generation = self.generation.wrapping_add(65);
                    ui.context()
                        .update_image(
                            raw,
                            ImageSource::rgba([2, 2], [self.generation, 180, 255, 160].repeat(4)),
                        )
                        .unwrap();
                }
                ui.horizontal(|ui| {
                    for (bytes, fit) in [
                        (JPEG, ImageFit::Contain),
                        (WEBP, ImageFit::Cover),
                        (JPEG, ImageFit::Stretch),
                    ] {
                        ui.add(
                            Image::new(bytes)
                                .size(vec2(190.0, 110.0))
                                .fit(fit)
                                .corner_radius(12.0),
                        );
                    }
                    ui.add(
                        Image::new(concat!(
                            env!("CARGO_MANIFEST_DIR"),
                            "/assets/images/landscape.jpg"
                        ))
                        .size(vec2(190.0, 110.0))
                        .fit(ImageFit::Cover),
                    );
                });
                let failed = Image::new(ImageSource::bytes(b"invalid bytes"))
                    .size(vec2(90.0, 32.0))
                    .placeholder(false)
                    .show(ui);
                if let ImageState::Error(error) = failed.state {
                    ui.label(format!("Custom error: {error}"));
                }
                ScrollArea::vertical()
                    .id_source("gallery")
                    .max_height(360.0)
                    .show_rows(ui, 114.0, 400, |ui, i| {
                        ui.horizontal(|ui| {
                            ui.add(
                                Image::new(if i % 2 == 0 { JPEG } else { WEBP })
                                    .size(vec2(180.0, 104.0))
                                    .fit(ImageFit::Cover)
                                    .corner_radius(14.0),
                            );
                            ui.add(Image::new(SVG).size(vec2(180.0, 104.0)));
                            ui.label(format!("Gallery row {i}"));
                        });
                    });
            });
        if self.smoke
            && [PNG, JPEG, WEBP, SVG]
                .into_iter()
                .all(|s| c.image_state(s).is_ready())
            && c.image_state(raw).is_ready()
            && c.image_metrics().pending_jobs == 0
        {
            match self.phase {
                0 => {
                    c.update_image(
                        raw,
                        ImageSource::rgba([2, 2], vec![0, 255, 255, 128].repeat(4)),
                    )
                    .unwrap();
                    self.phase = 1;
                }
                1 => {
                    assert_eq!(c.image_metrics().pending_jobs, 0);
                    assert!(c.draw_data().textures.iter().all(|t| !t.size.contains(&0)));
                    println!("image smoke passed: {:?}", c.image_metrics());
                    frame.close();
                    self.phase = 2;
                }
                _ => {}
            }
        }
    }
}
fn main() -> Result<(), zaxis::RunError> {
    zaxis::run_with_options(
        Images {
            raw: None,
            generation: 0,
            smoke: std::env::args().any(|a| a == "--smoke-test"),
            phase: 0,
            width: 240.0,
        },
        RunOptions {
            window_attributes: zaxis::winit::window::Window::default_attributes()
                .with_title("zaxis — Images")
                .with_inner_size(zaxis::winit::dpi::LogicalSize::new(1060.0, 860.0)),
            presentation_mode: PresentationMode::Immediate,
        },
    )
}
