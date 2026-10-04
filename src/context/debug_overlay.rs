//! Opt-in debug drawing: diagnostic frames and the hit regions under the cursor.

use super::{Context, HitAction, Id, Paint};
use crate::{Border, Color, CornerRadius, Rect, Shape, Vec2};

const TEXT_SIZE: f32 = 11.0;
const ISSUE: Color = Color::rgb(235, 64, 52);
const BOUNDS: Color = Color::rgb(64, 190, 255);
const CLIP: Color = Color::rgb(255, 166, 43);
const HIT: Color = Color::rgb(92, 214, 120);

fn outline(rect: Rect, color: Color, width: f32) -> Paint {
    Paint::Shape(Shape::Rect {
        rect,
        fill: Color::TRANSPARENT,
        rounding: CornerRadius::ZERO,
        border: Border::new(width, color),
    })
}

fn action_name(action: HitAction) -> &'static str {
    match action {
        HitAction::Block => "block",
        HitAction::ContextMenu => "context menu",
        HitAction::Activate => "activate",
        HitAction::Interact(_) => "interact",
        HitAction::Focus => "focus",
        HitAction::ComboBox => "combo box",
        HitAction::Tree | HitAction::TreeRow { .. } => "tree",
        HitAction::Slider => "slider",
        HitAction::DragValue => "drag value",
        HitAction::SplitResize { .. } => "split",
        HitAction::TextEdit => "text edit",
        HitAction::Move => "move",
        HitAction::Resize => "resize",
        HitAction::ScrollThumb { .. } => "scroll thumb",
        HitAction::ColumnResize { .. } => "column resize",
        HitAction::DragSource { .. } => "drag source",
        HitAction::DropTarget { .. } => "drop target",
    }
}

impl Context {
    /// A label on a dark chip, placed above `anchor` or inside it near the viewport edge.
    fn overlay_label(&mut self, paint: &mut Vec<Paint>, text: String, anchor: Vec2, color: Color) {
        let size = self.measure_text(&text, TEXT_SIZE, crate::FontWeight::REGULAR, f32::INFINITY);
        let viewport = self.viewport();
        let mut min = anchor - Vec2::new(0.0, size.y + 4.0);
        if min.y < viewport.min.y {
            min.y = anchor.y + 2.0;
        }
        min = min
            .min(viewport.max - size - Vec2::splat(4.0))
            .max(viewport.min);
        paint.push(Paint::Shape(Shape::Rect {
            rect: Rect::from_min_size(min, size + Vec2::new(4.0, 2.0)),
            fill: Color::rgba(0, 0, 0, 210),
            rounding: CornerRadius::all(2.0),
            border: Border::new(1.0, color),
        }));
        paint.push(Paint::Text {
            text,
            position: min + Vec2::new(2.0, 1.0),
            size: TEXT_SIZE,
            weight: crate::FontWeight::REGULAR,
            wrap_width: f32::INFINITY,
            color: Color::WHITE,
        });
    }

    pub(super) fn paint_debug_overlay(&mut self) {
        let overlay = self.diagnostics.overlay;
        let viewport = self.viewport();
        let mut paint = Vec::new();
        if overlay.issues {
            let issues: Vec<_> = self
                .diagnostics
                .published
                .iter()
                .map(|d| (d.rect, d.message.clone()))
                .collect();
            for (rect, message) in issues {
                // The first clause names the problem; the rest explains it in the query.
                let short = message.split(':').next().unwrap_or_default().to_owned();
                match rect.filter(|r| r.is_finite()) {
                    Some(rect) => {
                        paint.push(outline(rect, ISSUE, 2.0));
                        self.overlay_label(&mut paint, short, rect.min, ISSUE);
                    }
                    None => self.overlay_label(&mut paint, short, viewport.min, ISSUE),
                }
            }
        }
        if overlay.bounds || overlay.clip || overlay.hits {
            if let Some(pointer) = self.input.pointer {
                let window = self.top_window(pointer);
                let under: Vec<_> = self
                    .previous_hits
                    .iter()
                    .rev()
                    .filter(|hit| {
                        Some(hit.window) == window
                            && hit.rect.contains(pointer)
                            && hit.clip.contains(pointer)
                    })
                    .copied()
                    .collect();
                if let Some(top) = under.first() {
                    if overlay.clip {
                        paint.push(outline(top.clip, CLIP, 1.0));
                    }
                    if overlay.bounds {
                        paint.push(outline(top.rect, BOUNDS, 1.5));
                    }
                }
                if overlay.hits {
                    let lines: Vec<String> = under
                        .iter()
                        .map(|hit| {
                            format!("{} #{:x}", action_name(hit.action), hit.id.value() & 0xffff)
                        })
                        .collect();
                    for hit in &under {
                        paint.push(outline(hit.rect.intersect(hit.clip), HIT, 1.0));
                    }
                    if !lines.is_empty() {
                        self.overlay_label(
                            &mut paint,
                            lines.join("\n"),
                            pointer + Vec2::splat(16.0),
                            HIT,
                        );
                    }
                }
            }
        }
        if paint.is_empty() {
            return;
        }
        let id = Id::new("debug-overlay");
        self.popup_layers.push(id);
        self.paint(id, id, viewport, paint);
    }
}
