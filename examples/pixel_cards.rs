//! Parallax pixel cards: a grid of `Card`s whose background is one custom material (WGSL).
//! Hover a card and its image breaks into blocks that slide at three depths against the
//! pointer; keyboard focus shows the same effect centred. One `MaterialId`, one pipeline.
//!
//! `--smoke-test` drives hover, scrolling, resizing and the idle check on the real runner and exits.

use zaxis::{
    vec2, App, ComboBox, ComboBoxOption, Context, Frame, ImageHandle, ImageSource, MaterialId,
    PresentationMode, RunOptions, ScrollArea, Window,
};

#[path = "pixel_cards/cards.rs"]
mod cards;
#[path = "pixel_cards/shader.rs"]
mod shader;
#[path = "pixel_cards/smoke.rs"]
mod smoke;

use cards::{Cards, Deck};

static LANDSCAPE: &[u8] = include_bytes!("../assets/images/landscape.jpg");
static LANDSCAPE_WEBP: &[u8] = include_bytes!("../assets/images/landscape.webp");

struct PixelCards {
    material: Option<MaterialId>,
    images: Vec<ImageHandle>,
    cards: Cards,
    block: f32,
    depth: f32,
    drift: bool,
    textured: bool,
    count: Option<usize>,
    counts: Vec<ComboBoxOption<usize>>,
    smoke: Option<smoke::Smoke>,
}

impl PixelCards {
    fn new(smoke: bool) -> Self {
        Self {
            material: None,
            images: Vec::new(),
            cards: Cards::default(),
            block: 8.0,
            depth: 14.0,
            drift: false,
            textured: true,
            count: Some(12),
            counts: [6, 9, 12]
                .into_iter()
                .map(|n| ComboBoxOption::new(n, n, format!("{n} cards")))
                .collect(),
            smoke: smoke.then(smoke::Smoke::default),
        }
    }
}

/// A 64×64 plasma of whole pixels, so the nearest-neighbour look of the blocks has detail to show.
fn plasma() -> ImageSource {
    let mut pixels = Vec::with_capacity(64 * 64 * 4);
    for y in 0..64u32 {
        for x in 0..64u32 {
            let v = ((x * 7 + y * 3) % 64) as f32 / 64.0;
            let w = (((x * x + y * y) / 24) % 64) as f32 / 64.0;
            pixels.extend([
                (v * 255.0) as u8,
                (w * 200.0) as u8,
                (255.0 - v * 160.0) as u8,
                255,
            ]);
        }
    }
    ImageSource::rgba([64, 64], pixels)
}

impl App for PixelCards {
    fn update(&mut self, c: &mut Context, frame: &mut Frame<'_>) {
        let material = *self
            .material
            .get_or_insert_with(|| c.register_material(&shader::material()));
        if self.images.is_empty() {
            for source in [
                ImageSource::from(LANDSCAPE),
                ImageSource::from(LANDSCAPE_WEBP),
                plasma(),
            ] {
                self.images.extend(c.load_image(source));
            }
        }
        Window::new("Pixel cards")
            .default_position(vec2(8.0, 8.0))
            .default_size(vec2(980.0, 760.0))
            .show(c, |ui| {
                ui.horizontal(|ui| {
                    ui.slider_labeled(&mut self.block, 2.0..=24.0, "Block, px");
                    ui.slider_labeled(&mut self.depth, 0.0..=40.0, "Parallax, px");
                });
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.textured, "Textured");
                    ui.checkbox(&mut self.drift, "Drift");
                    ui.add(
                        ComboBox::new(&mut self.count, &self.counts)
                            .id_source("count")
                            .width(140.0),
                    );
                });
                let deck = Deck {
                    material,
                    block: self.block.round(),
                    depth: self.depth,
                    drift: self.drift,
                    textured: self.textured,
                    images: &self.images,
                };
                ScrollArea::vertical()
                    .id_source("deck")
                    .max_height(ui.available_height())
                    .show(ui, |ui| {
                        self.cards.grid(ui, &deck, self.count.unwrap_or(12), 230.0);
                    });
            });
        if let Some(smoke) = &mut self.smoke {
            smoke.step(c, frame);
        }
    }
}

fn main() -> Result<(), zaxis::RunError> {
    let smoke = std::env::args().any(|a| a == "--smoke-test");
    zaxis::run_with_options(
        PixelCards::new(smoke),
        RunOptions {
            window_attributes: zaxis::winit::window::Window::default_attributes()
                .with_title("zaxis — Pixel cards")
                .with_inner_size(zaxis::winit::dpi::LogicalSize::new(1000.0, 780.0)),
            presentation_mode: PresentationMode::Vsync,
            ..Default::default()
        },
    )
}
