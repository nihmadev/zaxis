//! The overflow menu: a button at the end of a strip that scrolls, listing every tab in a
//! popup so that one far out of sight is a pick away.

use super::style::Look;
use crate::{
    components::{theme::ButtonStyle, ButtonVariant, Popup, Sense, Ui},
    context::Paint,
    AccessRole, Button, Rect, Shape, Vec2,
};

pub(super) struct Choice {
    pub label: String,
    pub selected: bool,
    pub enabled: bool,
}

/// The button and, while `open`, its popup. Returns the tab that was picked.
pub(super) fn show(
    ui: &mut Ui<'_>,
    anchor: Rect,
    open: &mut bool,
    choices: &[Choice],
    look: &Look,
) -> Option<usize> {
    let response = ui.interact(anchor, "overflow-menu", Sense::CLICK);
    if response.clicked() {
        *open = !*open;
    }
    let mut paints = Vec::new();
    if response.hovered || *open {
        paints.push(Paint::Shape(
            Shape::rect(
                anchor.shrink(look.plate_inset.y),
                look.plate
                    .with_opacity(if response.pressed { 1.0 } else { 0.7 }),
            )
            .corner_radius(look.plate_radius)
            .into(),
        ));
    }
    let c = anchor.center();
    let ink = if response.hovered || *open {
        look.text_hover
    } else {
        look.text_idle
    };
    for (a, b) in [
        (Vec2::new(-4.0, -2.0), Vec2::new(0.0, 2.0)),
        (Vec2::new(0.0, 2.0), Vec2::new(4.0, -2.0)),
    ] {
        paints.push(Paint::Shape(Shape::Line {
            start: c + a,
            end: c + b,
            width: 1.25,
            color: ink,
        }));
    }
    ui.context
        .paint(response.id.with("paint"), ui.window, ui.clip, paints);
    ui.a11y(response.id, anchor, AccessRole::Button, |node| {
        node.label("All tabs").clicks(response.id).has_popup();
    });
    let mut picked = None;
    let height = (choices.len() as f32 * (ui.style().control_height + 2.0)).min(320.0) + 8.0;
    let popup = Popup::new(response.id, anchor).size(Vec2::new(240.0, height));
    popup.show(ui, open, |ui| {
        crate::ScrollArea::vertical()
            .id_source("overflow-list")
            .max_height(height - 8.0)
            .show(ui, |ui| {
                for (i, choice) in choices.iter().enumerate() {
                    let row = Button::new(choice.label.as_str())
                        .id_source(i)
                        .variant(ButtonVariant::Ghost)
                        .selected(choice.selected)
                        .style(ButtonStyle::default());
                    let row = ui.add_enabled_ui(choice.enabled, |ui| ui.add(row));
                    if row.clicked() {
                        picked = Some(i);
                    }
                }
            });
    });
    if picked.is_some() {
        *open = false;
    }
    picked
}
