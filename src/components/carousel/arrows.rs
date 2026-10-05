//! Previous and next buttons over the pages.
use super::{region::with_region, style::Look};
use crate::{
    components::{Button, Ui, Widget},
    PaintMode, Rect, Shape, Vec2,
};

/// Which button was clicked: `-1` previous, `1` next.
pub(super) fn show(
    ui: &mut Ui<'_>,
    scope: crate::Id,
    stage: Rect,
    axis: usize,
    can: [bool; 2],
    enabled: bool,
    look: &Look,
) -> Option<i64> {
    let size = look.arrow_size().min(stage.size().min_element().max(0.0));
    if size < 8.0 {
        return None;
    }
    let inset = look.arrow_inset();
    let center = stage.center();
    let (first, last) = if axis == 0 {
        (
            Vec2::new(stage.min.x + inset, center.y - size * 0.5),
            Vec2::new(stage.max.x - inset - size, center.y - size * 0.5),
        )
    } else {
        (
            Vec2::new(center.x - size * 0.5, stage.min.y + inset),
            Vec2::new(center.x - size * 0.5, stage.max.y - inset - size),
        )
    };
    let mut clicked = None;
    for (step, origin) in [(-1_i64, first), (1, last)] {
        let rect = Rect::from_min_size(origin, Vec2::splat(size));
        // Unit vector from the middle of the button to the tip of its chevron.
        let tip = if axis == 0 {
            Vec2::X * step as f32
        } else {
            Vec2::Y * step as f32
        };
        let button = Button::new("")
            .id_source(("arrow", step))
            .min_size(Vec2::splat(size))
            .width(size)
            .corner_radius(size * 0.5)
            .enabled(can[usize::from(step > 0)])
            .painter(PaintMode::After, move |painter, info| {
                let color = info.style.foreground.unwrap_or(crate::Color::WHITE);
                let color = if info.state.enabled {
                    color
                } else {
                    color.with_opacity(0.4)
                };
                let reach = info.bounds.size().min_element() * 0.16;
                let center = info.bounds.center();
                let side = Vec2::new(-tip.y, tip.x);
                let apex = center + tip * reach;
                for wing in [side, -side] {
                    painter.paint(Shape::Line {
                        start: apex,
                        end: center - tip * reach + wing * reach * 1.1,
                        width: 2.0,
                        color,
                    });
                }
            });
        // The button draws a chevron and no text: name it for assistive technology, and say
        // so when the carousel it belongs to takes no input.
        let node = ui.context.a11y_len();
        let response = with_region(ui, scope.with(step), rect, stage, true, |u| {
            if u.context.a11y_on() {
                u.add(button.accessible_label(if step < 0 { "Previous" } else { "Next" }))
            } else {
                u.add(button)
            }
        });
        if !enabled {
            if let Some(node) = ui.context.a11y_node_mut(node) {
                node.disabled(true);
            }
        }
        if response.clicked() {
            clicked = Some(step);
        }
    }
    clicked
}
