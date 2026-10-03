use std::hash::Hash;

use super::Ui;
use crate::{
    context::{popup::PopupState, HitAction, HitRegion, Paint},
    layout::LayoutCursor,
    Border, Color, CornerRadius, Id, Layout, Padding, Rect, Shape, Vec2,
};

/// Anchored overlay above every window, independent of the parent's clip.
/// One popup owns input at a time. Outside presses and Escape restore focus;
/// the outside press is consumed before reaching lower content.
pub struct Popup {
    source: Id,
    anchor: Rect,
    size: Vec2,
    gap: f32,
    padding: Padding,
    rounding: CornerRadius,
    fill: Option<Color>,
    border: Option<Border>,
    return_focus: Option<Id>,
    pub(super) key_target: Option<Id>,
    pub(super) progress: f32,
    pub(super) animated: bool,
}

pub struct PopupOutput<R> {
    pub inner: R,
    pub rect: Rect,
    pub opens_upward: bool,
}

impl Popup {
    pub fn new(source: impl Hash, anchor: Rect) -> Self {
        Self {
            source: Id::new(source),
            anchor,
            size: Vec2::new(anchor.size().x, 120.0),
            gap: 4.0,
            padding: Padding::all(2.0),
            rounding: CornerRadius::all(4.0),
            fill: None,
            border: None,
            return_focus: None,
            key_target: None,
            progress: 1.0,
            animated: false,
        }
    }
    pub fn size(mut self, size: Vec2) -> Self {
        assert!(size.is_finite() && size.min_element() >= 0.0);
        self.size = size;
        self
    }
    pub fn gap(mut self, gap: f32) -> Self {
        assert!(gap.is_finite() && gap >= 0.0);
        self.gap = gap;
        self
    }
    pub fn padding(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self
    }
    pub fn rounding(mut self, rounding: CornerRadius) -> Self {
        self.rounding = rounding;
        self
    }
    pub fn fill(mut self, fill: Color) -> Self {
        self.fill = Some(fill);
        self
    }
    pub fn border(mut self, border: Border) -> Self {
        self.border = Some(border);
        self
    }
    pub fn return_focus(mut self, id: Id) -> Self {
        self.return_focus = Some(id);
        self
    }

    pub(super) fn id(ui: &Ui<'_>, source: Id) -> Id {
        ui.scope.with(("popup", source))
    }

    pub fn show<R>(
        self,
        ui: &mut Ui<'_>,
        open: &mut bool,
        build: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> Option<PopupOutput<R>> {
        let id = Self::id(ui, self.source);
        if ui.context.dismissed_popups.remove(&id) {
            *open = false;
        }
        if !ui.enabled {
            *open = false;
        }
        if !*open && ui.context.popup.as_ref().is_some_and(|p| p.id == id) {
            ui.context.dismiss_popup(true);
            ui.context.dismissed_popups.remove(&id);
        }
        if !*open && (!self.animated || self.progress <= 0.0) {
            return None;
        }
        let viewport = ui.context.viewport();
        let anchor_clip = ui.clip_rect().intersect(viewport);
        let (full, upward) = place(self.anchor, self.size, viewport, self.gap);
        if full.is_empty() {
            *open = false;
            if ui.context.popup.as_ref().is_some_and(|p| p.id == id) {
                ui.context.dismiss_popup(true);
                ui.context.dismissed_popups.remove(&id);
            }
            return None;
        }
        let height = full.size().y * self.progress.clamp(0.0, 1.0);
        let rect = if upward {
            Rect::from_min_max(Vec2::new(full.min.x, full.max.y - height), full.max)
        } else {
            Rect::from_min_size(full.min, Vec2::new(full.size().x, height))
        };
        if *open {
            let existing = ui.context.popup.as_ref().filter(|p| p.id == id);
            let return_focus = self
                .return_focus
                .or_else(|| existing.and_then(|p| p.return_focus))
                .or(ui.context.focused_widget);
            let opening = existing.is_none();
            if ui.context.popup.as_ref().is_some_and(|p| p.id != id) {
                ui.context.dismiss_popup(true);
            }
            if opening {
                ui.context.set_focus(self.key_target);
            }
            ui.context.popup = Some(PopupState {
                id,
                owner: ui.window,
                anchor: self.anchor.intersect(anchor_clip),
                rect,
                return_focus,
                key_target: self.key_target,
                last_frame: ui.context.frame,
            });
        }
        ui.context.popup_layers.push(id);
        if *open {
            ui.context.register_hit(HitRegion {
                id: id.with("block"),
                window: id,
                rect: viewport,
                clip: viewport,
                action: HitAction::Block,
            });
            if let Some(target) = self.key_target {
                ui.context.register_hit(HitRegion {
                    id: target,
                    window: id,
                    rect: self.anchor,
                    clip: anchor_clip,
                    action: HitAction::ComboBox,
                });
            }
        }
        let style = ui.style().clone();
        ui.context.paint(
            id.with("body"),
            id,
            rect.intersect(viewport),
            vec![Paint::Shape(Shape::Rect {
                rect,
                fill: self.fill.unwrap_or(style.window_fill),
                rounding: self.rounding,
                border: self.border.unwrap_or(style.border),
            })],
        );
        let mut child = Ui {
            flow: None,
            context: ui.context,
            window: id,
            scope: id,
            sequence: 0,
            clip: self.padding.inset(rect).intersect(viewport),
            layout: LayoutCursor::new(self.padding.inset(full), Layout::Vertical, 2.0),
            enabled: *open && ui.enabled,
            backdrop_blur: 0.0,
            hover_style: ui.hover_style,
        };
        child.begin_layout(crate::Align::Start);
        let inner = build(&mut child);
        child.finish_layout();
        if *open && !child.context.popup.as_ref().is_some_and(|p| p.id == id) {
            *open = false;
            child.context.dismissed_popups.remove(&id);
        }
        if !*open {
            child.context.remove_popup_hits(id);
        }
        Some(PopupOutput {
            inner,
            rect,
            opens_upward: upward,
        })
    }
}

pub(crate) fn place(anchor: Rect, size: Vec2, viewport: Rect, gap: f32) -> (Rect, bool) {
    let below = (viewport.max.y - anchor.max.y - gap).max(0.0);
    let above = (anchor.min.y - viewport.min.y - gap).max(0.0);
    let upward = size.y > below && above > below;
    let height = size.y.min(if upward { above } else { below });
    let width = size.x.min(viewport.size().x).max(0.0);
    let x = anchor
        .min
        .x
        .clamp(viewport.min.x, (viewport.max.x - width).max(viewport.min.x));
    let y = if upward {
        anchor.min.y - gap - height
    } else {
        anchor.max.y + gap
    };
    (
        Rect::from_min_size(Vec2::new(x, y), Vec2::new(width, height)).intersect(viewport),
        upward,
    )
}
