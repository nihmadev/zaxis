//! A card: an ordinary `Card` with a parallax material painted under its content.
//!
//! The effect is purely visual. Hover, click and keyboard focus come from one `Ui::interact` over
//! the card's rectangle, registered before the content so the buttons stay on top. Pointer and
//! hover are smoothed by springs and return to rest when the pointer leaves. Keyboard focus
//! shows the same effect with the pointer at the centre, so the layers shift symmetrically.

use super::shader::Look;
use std::collections::HashMap;
use zaxis::{
    vec2, Card, Color, ImageFit, ImageHandle, MaterialId, Padding, Rect, Sense, SpringOptions, Ui,
    Vec2,
};

const NAMES: [(&str, &str, [Color; 2]); 12] = [
    (
        "Aurora",
        "Ribbons of green light over a frozen lake.",
        [Color::rgb(18, 52, 86), Color::rgb(110, 230, 180)],
    ),
    (
        "Ember",
        "Coals that glow after the fire has gone.",
        [Color::rgb(70, 20, 24), Color::rgb(255, 150, 70)],
    ),
    (
        "Harbour",
        "Cranes and cold water at first light.",
        [Color::rgb(24, 44, 70), Color::rgb(120, 170, 210)],
    ),
    (
        "Orchid",
        "Soft petals, saturated at the edges.",
        [Color::rgb(60, 24, 80), Color::rgb(240, 130, 220)],
    ),
    (
        "Tundra",
        "Wind-cut snow and pale grass.",
        [Color::rgb(40, 52, 64), Color::rgb(210, 226, 235)],
    ),
    (
        "Dusk",
        "The last warm band before the dark.",
        [Color::rgb(34, 26, 70), Color::rgb(250, 160, 110)],
    ),
    (
        "Moss",
        "Damp stone and deep green.",
        [Color::rgb(16, 40, 30), Color::rgb(150, 210, 110)],
    ),
    (
        "Citrine",
        "Amber glass held up to the sun.",
        [Color::rgb(60, 40, 8), Color::rgb(255, 214, 90)],
    ),
    (
        "Cobalt",
        "A pigment that stays blue in any light.",
        [Color::rgb(10, 24, 80), Color::rgb(80, 140, 255)],
    ),
    (
        "Quartz",
        "Rose crystal, cloudy at the core.",
        [Color::rgb(70, 36, 50), Color::rgb(255, 190, 200)],
    ),
    (
        "Slate",
        "Layered grey roofs after rain.",
        [Color::rgb(30, 34, 40), Color::rgb(150, 164, 180)],
    ),
    (
        "Lagoon",
        "Shallow turquoise over white sand.",
        [Color::rgb(8, 54, 66), Color::rgb(90, 230, 220)],
    ),
];

/// Everything the cards of one frame share.
pub struct Deck<'a> {
    pub material: MaterialId,
    pub block: f32,
    pub depth: f32,
    pub drift: bool,
    pub textured: bool,
    pub images: &'a [ImageHandle],
}

/// State kept between frames, per card index.
#[derive(Default)]
pub struct Cards {
    /// Outer height of each card in the previous pass: the frame is painted from it, so the
    /// effect rectangle follows the content after one pass.
    heights: HashMap<usize, f32>,
    pub opened: HashMap<usize, u32>,
}

impl Cards {
    /// Cards of the grid in rows that fit `available` width at least `min_width` wide.
    pub fn grid(&mut self, ui: &mut Ui<'_>, deck: &Deck<'_>, count: usize, min_width: f32) {
        const GAP: f32 = 12.0;
        let available = ui.available_width() - 14.0;
        let columns = (((available + GAP) / (min_width + GAP)) as usize).max(1);
        let width = ((available - GAP * (columns - 1) as f32) / columns as f32).floor();
        for row in (0..count).step_by(columns) {
            ui.horizontal(|ui| {
                for index in row..(row + columns).min(count) {
                    self.card(ui, deck, index, width);
                    ui.add_space(GAP);
                }
            });
            ui.add_space(GAP);
        }
    }

    fn card(&mut self, ui: &mut Ui<'_>, deck: &Deck<'_>, index: usize, width: f32) {
        let (name, text, tints) = NAMES[index % NAMES.len()];
        const INSET: f32 = 14.0;
        let previous = self.heights.get(&index).copied().unwrap_or(0.0);
        let mut clicks = 0;
        let out = Card::new(("card", index))
            .width(width)
            // No padding of the Card itself: its content area is the whole card, so the effect
            // is not clipped to a padded column. The inset is applied inside.
            .padding(Padding::all(0.0))
            .show(ui, |content| {
                let corner = content.allocate_space(Vec2::ZERO).min;
                let rect = Rect::from_min_size(corner, vec2(width, previous));
                if rect.size().min_element() > 1.0 {
                    clicks += effect(content, deck, index, rect, tints);
                }
                content.add_space(INSET);
                content.horizontal(|row| {
                    row.add_space(INSET);
                    row.with_width(width - 2.0 * INSET, |column| {
                        column.vertical(|body| {
                            body.heading(name);
                            body.muted(text);
                            body.add_space(4.0);
                            body.horizontal(|buttons| {
                                if buttons.button("Open").clicked() {
                                    clicks += 1;
                                }
                                if buttons.button("Reset").clicked() {
                                    self.opened.insert(index, 0);
                                    clicks = 0;
                                }
                            });
                            body.muted(format!(
                                "opened {}×",
                                self.opened.get(&index).copied().unwrap_or(0)
                            ));
                        });
                    });
                });
                content.add_space(INSET);
            });
        *self.opened.entry(index).or_default() += clicks;
        self.heights.insert(index, out.rect.size().y);
    }
}

/// Interaction and paint of one card's rectangle. Returns the number of activations.
fn effect(ui: &mut Ui<'_>, deck: &Deck<'_>, index: usize, rect: Rect, tints: [Color; 2]) -> u32 {
    let response = ui.interact(rect, ("card-hit", index), Sense::CLICK | Sense::FOCUS);
    let visible = rect.intersect(ui.clip_rect());
    let pointer = ui
        .context()
        .input()
        .pointer
        .filter(|p| visible.contains(*p));
    let rest = vec2(0.5, 0.5);
    let (hover, target) = match pointer {
        Some(p) => (
            1.0,
            ((p - rect.min) / rect.size()).clamp(Vec2::ZERO, Vec2::ONE),
        ),
        None if response.focus_visible => (1.0, rest),
        None => (0.0, rest),
    };
    let hover = ui
        .spring_transition(
            ("fx-hover", index),
            hover,
            SpringOptions::frequency(3.5, 1.0),
        )
        .value
        .value;
    let pointer = ui
        .spring_transition(
            ("fx-pointer", index),
            target,
            SpringOptions::frequency(6.0, 0.85),
        )
        .value
        .value;
    let textured = deck.textured && index % 4 != 3 && !deck.images.is_empty();
    let look = Look {
        pointer,
        hover,
        block: deck.block,
        depth: deck.depth,
        seed: index as f32,
        textured,
        drift: deck.drift,
        tints,
    };
    // Inside the card's hairline border, with the radius reduced by its width.
    let radius = ui.style().rounding;
    let inner = rect.shrink(1.0);
    let mut paint = ui
        .material(inner, deck.material)
        .params(look.params())
        .corner_radius(zaxis::CornerRadius {
            top_left: (radius.top_left - 1.0).max(0.0),
            top_right: (radius.top_right - 1.0).max(0.0),
            bottom_right: (radius.bottom_right - 1.0).max(0.0),
            bottom_left: (radius.bottom_left - 1.0).max(0.0),
        })
        .id_source(("fx", index))
        // Only a hovered card with drift enabled reads the clock; at rest nothing asks for frames.
        .animated(deck.drift && hover > 0.001);
    if textured {
        paint = paint
            .image(deck.images[index % deck.images.len()])
            .fit(ImageFit::Cover);
    }
    paint.show(ui);
    u32::from(response.clicked())
}
