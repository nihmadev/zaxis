//! The drag preview: a snapshot of what the source painted, replayed in an
//! overlay layer that owns no hits, so it never shadows a target and is never
//! clipped by the source's window or scroll area.
use super::super::{Context, HitAction, Id, Paint};
use super::state::{Session, Snapshot};
use crate::components::drag_drop::PreviewKind;
use crate::{Rect, Shape, Transform, Vec2};

/// Layer shared by snapshot and custom previews. It is ranked with the popup
/// layers at the end of the pass, so it is above every window.
pub fn layer_id() -> Id {
    Id::new("zaxis-drag-preview")
}
pub(super) fn return_id() -> Id {
    layer_id().with("return")
}

/// Uniform scale that fits `size` into `max`; previews never grow.
pub(crate) fn fit_scale(size: Vec2, max: Vec2) -> f32 {
    if size.x <= 0.0 || size.y <= 0.0 {
        return 1.0;
    }
    (max.x / size.x).min(max.y / size.y).clamp(0.05, 1.0)
}

/// Top-left of the preview: the grab point stays under the pointer, and a
/// keyboard drag floats just below its virtual pointer.
pub(crate) fn origin(session: &Session, scale: f32) -> Vec2 {
    if session.keyboard {
        session.pointer + Vec2::splat(12.0)
    } else {
        session.pointer - session.grab * scale
    }
}

impl Context {
    /// Remember the source's painted elements. They are final here: placement,
    /// scrolling and clipping have been applied and ordering is not yet sorted.
    fn drag_capture_snapshot(&mut self) {
        let Some(session) = &self.drag.session else {
            return;
        };
        if !session.started
            || session.snapshot.is_some()
            || session.info.preview == PreviewKind::None
        {
            return;
        }
        let (id, window) = (session.info.id, session.window);
        let Some(rect) = self
            .interaction
            .hits
            .iter()
            .find(|hit| hit.id == id && matches!(hit.action, HitAction::DragSource { .. }))
            .map(|hit| hit.rect)
        else {
            return;
        };
        let limit = Rect::from_min_max(rect.min - Vec2::ONE, rect.max + Vec2::ONE);
        let mut items = Vec::new();
        for element in &self.paint_state.elements {
            if element.layer != window || element.blur.is_some() {
                continue;
            }
            let Some(cached) = self.paint_state.cache.get(&element.id) else {
                continue;
            };
            let inside = cached.bounds.is_some_and(|b| {
                b.min.x >= limit.min.x
                    && b.min.y >= limit.min.y
                    && b.max.x <= limit.max.x
                    && b.max.y <= limit.max.y
            });
            if inside {
                let order = self
                    .paint_state
                    .order
                    .get(&element.id)
                    .copied()
                    .unwrap_or(0);
                items.push((order, element.id, cached.paint.clone()));
            }
        }
        items.sort_by_key(|item| item.0);
        let items = items
            .into_iter()
            .map(|(_, id, paint)| (id, paint))
            .collect();
        if let Some(session) = self.drag.session.as_mut() {
            session.snapshot = Some(Snapshot { rect, items });
        }
    }

    /// Emit the preview for this pass: the live one under the pointer, or the
    /// one returning to its source after a cancel.
    pub(crate) fn drag_emit_preview(&mut self) {
        self.drag_capture_snapshot();
        let layer = layer_id();
        let custom = self.drag.custom_preview == self.frame;
        let mut painted = custom;
        if let Some(session) = self.drag.session.as_ref().filter(|s| s.started) {
            let resolved = session.info.resolved;
            if session.info.preview != PreviewKind::None && !custom {
                if let Some(snapshot) = session.snapshot.clone() {
                    let scale = fit_scale(snapshot.rect.size(), resolved.preview_max_size);
                    let at = origin(session, scale);
                    self.paint_snapshot(&snapshot, at, scale, resolved.preview_opacity);
                    painted = true;
                }
            }
        } else if let Some(returning) = self.drag.returning.take() {
            let to = self
                .interaction
                .previous_hits
                .iter()
                .find(|hit| hit.id == returning.source)
                .map_or(returning.to, |hit| hit.rect.min);
            let progress = self.transition_visible(
                return_id(),
                Some(0.0_f32),
                1.0_f32,
                returning.tween.clone(),
                true,
            );
            if progress.completed() || returning.snapshot.items.is_empty() {
                self.remove_animation(return_id());
            } else {
                let p = progress.value.clamp(0.0, 1.0);
                let at = returning.from.lerp(to, p);
                let scale = returning.scale + (1.0 - returning.scale) * p;
                self.paint_snapshot(
                    &returning.snapshot,
                    at,
                    scale,
                    returning.opacity * (1.0 - p),
                );
                painted = true;
                self.drag.returning = Some(returning);
            }
        }
        if painted {
            self.popups.layers.push(layer);
        }
    }

    fn paint_snapshot(&mut self, snapshot: &Snapshot, at: Vec2, scale: f32, opacity: f32) {
        let layer = layer_id();
        let viewport = self.viewport();
        // Whole physical pixels keep replayed glyphs crisp at any DPI.
        let at = (at * self.scale).round() / self.scale;
        let transform = Transform::around(snapshot.rect.min, scale, at - snapshot.rect.min);
        // A small card keeps a bare label readable over whatever it travels across.
        let card = Rect::from_min_max(
            snapshot.rect.min - Vec2::splat(4.0),
            snapshot.rect.max + Vec2::splat(4.0),
        );
        let rounding = crate::CornerRadius::all(6.0);
        self.paint(
            layer.with("card"),
            layer,
            viewport,
            vec![Paint::Visual {
                paint: vec![
                    Paint::Shape(Shape::Shadow {
                        rect: card,
                        rounding,
                        shadow: crate::Shadow {
                            color: crate::Color::rgba(0, 0, 0, 110),
                            offset: Vec2::new(0.0, 4.0),
                            blur_radius: 12.0,
                            spread: 0.0,
                        },
                    }),
                    Paint::Shape(Shape::Rect {
                        rect: card,
                        fill: self.style.window_fill,
                        rounding,
                        border: self.style.border,
                    }),
                ],
                transform,
                opacity,
            }],
        );
        for (n, (_, paint)) in snapshot.items.iter().enumerate() {
            self.paint(
                layer.with(n),
                layer,
                viewport,
                vec![Paint::Visual {
                    paint: paint.clone(),
                    transform,
                    opacity,
                }],
            );
        }
    }
}
