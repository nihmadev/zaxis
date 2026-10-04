//! The always-visible part of a menu: title buttons, or a single hamburger button.
use super::MenuItem;
use crate::{
    components::{Button, ButtonVariant, Ui},
    Color, Id, Padding, PaintMode, Rect, Shape, Vec2,
};

/// Draws the triggers and returns each one's rectangle (separators have none) with the
/// index of the one clicked in this pass.
pub(super) fn show(
    ui: &mut Ui<'_>,
    source: Id,
    items: &[MenuItem],
    compact: bool,
    open_index: Option<usize>,
) -> (Vec<Option<Rect>>, Option<usize>) {
    if compact {
        let side = ui.style().control_height;
        let response = ui.add(
            Button::new("##menu")
                .id_source(source)
                .variant(ButtonVariant::Ghost)
                .padding(Padding::all(0.0))
                .min_size(Vec2::splat(side))
                .selected(open_index.is_some())
                .painter(PaintMode::After, |painter, info| {
                    let color = info.style.foreground.unwrap_or(Color::WHITE);
                    let center = info.bounds.center();
                    for dy in [-4.0, 0.0, 4.0] {
                        painter.paint(Shape::Line {
                            start: center + Vec2::new(-6.0, dy),
                            end: center + Vec2::new(6.0, dy),
                            width: 1.5,
                            color,
                        });
                    }
                }),
        );
        return (vec![Some(response.rect)], response.clicked().then_some(0));
    }
    let mut rects = vec![None; items.len()];
    let mut clicked = None;
    ui.horizontal(|ui| {
        for (index, item) in items.iter().enumerate() {
            if item.is_separator() {
                continue;
            }
            let response = ui.add(
                Button::new(item.text.as_str())
                    .id_source((source, index))
                    .variant(ButtonVariant::Ghost)
                    .enabled(item.enabled)
                    .selected(open_index == Some(index)),
            );
            rects[index] = Some(response.rect);
            if response.clicked() {
                clicked = Some(index);
            }
        }
    });
    (rects, clicked)
}
