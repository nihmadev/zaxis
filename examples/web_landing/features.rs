use crate::reveal::Reveal;
use std::time::Duration;
use zaxis::{icons, vec2, Card, FontWeight, Image, Text, Ui};

struct Feature {
    icon: &'static icons::Icon,
    title: &'static str,
    text: &'static str,
}

static FEATURES: [Feature; 6] = [
    Feature {
        icon: &icons::ZAP,
        title: "Immediate mode",
        text: "Describe the interface on every frame. Retained state and cached geometry keep unchanged frames cheap.",
    },
    Feature {
        icon: &icons::CPU,
        title: "wgpu renderer",
        text: "One renderer on Vulkan, Metal, DX12, WebGPU and WebGL2, with a glyph atlas and batched meshes.",
    },
    Feature {
        icon: &icons::SPARKLES,
        title: "Animations",
        text: "Tweens, springs, keyframes and transitions on one engine that sleeps once everything settles.",
    },
    Feature {
        icon: &icons::TYPE,
        title: "Rich text",
        text: "Shaped text with weights, links, selection and copy, set in the bundled fonts.",
    },
    Feature {
        icon: &icons::LIST,
        title: "Virtualized lists",
        text: "Lists, tables and trees build only the rows in view, so ten thousand rows cost one screen.",
    },
    Feature {
        icon: &icons::GLOBE,
        title: "Native and browser",
        text: "The same code runs in a desktop window and on a canvas in the page, with no changes.",
    },
];

/// Wide pages show three columns, medium ones two, narrow ones a single column.
fn columns(width: f32) -> usize {
    if width >= 860.0 {
        3
    } else if width >= 560.0 {
        2
    } else {
        1
    }
}

/// The Features heading and its grid of cards. Cards enter in a cascade as they scroll into
/// view. Returns the screen y of the heading, which Explore scrolls to.
pub fn show(ui: &mut Ui<'_>, reveal: &mut Reveal) -> f32 {
    ui.add_space(48.0);
    let top = ui.add(
        Text::new("Features")
            .size(32.0)
            .weight(FontWeight::SEMIBOLD),
    );
    ui.muted("What the library gives an interface, on every platform it runs on.");
    ui.add_space(16.0);
    let gap = ui.style().spacing;
    let columns = columns(ui.available_width());
    let width = (ui.available_width() - gap * (columns - 1) as f32) / columns as f32;
    for (row, features) in FEATURES.chunks(columns).enumerate() {
        ui.horizontal(|ui| {
            for (column, feature) in features.iter().enumerate() {
                let index = row * columns + column;
                let delay = Duration::from_millis(column as u64 * 90);
                reveal.block(ui, index, delay, |ui| card(ui, index, width, feature));
            }
        });
    }
    ui.add_space(48.0);
    top.rect.min.y
}

fn card(ui: &mut Ui<'_>, index: usize, width: f32, feature: &Feature) -> zaxis::Rect {
    let accent = ui.style().accent;
    Card::new(("feature", index))
        .width(width)
        .show(ui, |ui| {
            ui.with_min_height(128.0, |ui| {
                ui.add(Image::new(feature.icon).size(vec2(24.0, 24.0)).tint(accent));
                ui.add(Text::new(feature.title).weight(FontWeight::SEMIBOLD));
                ui.muted(feature.text);
            });
        })
        .rect
}
