use std::hash::Hash;

use super::Ui;
use crate::{
    context::{popup::PopupState, HitAction, HitRegion},
    layout::LayoutCursor,
    Border, Color, CornerRadius, Id, Layout, Padding, Rect, Vec2,
};

/// Anchored overlay above every window, independent of the parent's clip.
/// One popup owns input at a time. Outside presses and Escape restore focus;
/// the outside press is consumed before reaching lower content.
pub struct Popup {
    source: Id,
    anchor: Rect,
    size: Vec2,
    gap: Option<f32>,
    padding: Option<Padding>,
    rounding: Option<CornerRadius>,
    fill: Option<Color>,
    border: Option<Border>,
    style: super::theme::PopupStyle,
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
            gap: None,
            padding: None,
            rounding: None,
            fill: None,
            border: None,
            style: Default::default(),
            return_focus: None,
            key_target: None,
            progress: 1.0,
            animated: false,
        }
    }
    pub fn style(mut self, style: super::theme::PopupStyle) -> Self {
        self.style = style;
        self
    }
    #[track_caller]
    pub fn size(mut self, size: Vec2) -> Self {
        self.size = super::sanitize::size("Popup::size", size).unwrap_or(self.size);
        self
    }
    #[track_caller]
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = super::sanitize::non_negative("Popup::gap", gap).or(self.gap);
        self
    }
    pub fn padding(mut self, padding: Padding) -> Self {
        self.padding = Some(padding);
        self
    }
    pub fn corner_radius(mut self, radius: impl Into<CornerRadius>) -> Self {
        self.rounding = Some(radius.into());
        self
    }
    #[deprecated(
        note = "use `.corner_radius(..)`; one name for the corner radius of every component"
    )]
    pub fn rounding(self, rounding: CornerRadius) -> Self {
        self.corner_radius(rounding)
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
        if !self.anchor.is_finite() {
            // Without a usable anchor there is nothing to place the popup against.
            if *open {
                ui.context.report(
                    crate::DiagnosticKind::PopupWithoutAnchor,
                    Some(id),
                    None,
                    || "popup without anchor: the anchor rect is not finite".into(),
                );
            }
            *open = false;
            return None;
        }
        if ui.context.dismissed_popups.remove(&id) {
            *open = false;
        }
        if !ui.enabled || ui.context.modal_blocks_popup() {
            *open = false;
        }
        if !*open && ui.context.popup.as_ref().is_some_and(|p| p.id == id) {
            ui.context.dismiss_popup(true);
            ui.context.dismissed_popups.remove(&id);
        }
        if !*open && (!self.animated || self.progress <= 0.0) {
            return None;
        }
        let style = ui.style().clone();
        let mut component = style.popup;
        component.merge(self.style);
        let padding = self
            .padding
            .or(component.padding)
            .unwrap_or(Padding::all(2.0));
        let gap = self.gap.or(component.gap).unwrap_or(4.0);
        let viewport = ui.context.viewport();
        let anchor_clip = ui.clip_rect().intersect(viewport);
        let (full, upward) = place(self.anchor, self.size, viewport, gap);
        if full.is_empty() {
            if *open {
                ui.context.report(
                    crate::DiagnosticKind::PopupWithoutAnchor,
                    Some(id),
                    Some(self.anchor),
                    || "popup without room: it cannot be placed next to its anchor".into(),
                );
            }
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
        ui.context.push_popup_layer(id);
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
        let mut body =
            super::appearance::Appearance::new(style.window_fill, style.border, style.text_color);
        body.rounding = CornerRadius::all(4.0);
        body.blur = style.blur_radius;
        body.shadow = style.elevation;
        body.opacity = style.opacity;
        body.apply(component.surface);
        if let Some(v) = self.fill {
            body.fill = crate::Gradient::new(v, v);
        }
        if let Some(v) = self.border {
            body.border = v;
        }
        if let Some(v) = self.rounding {
            body.rounding = v;
        }
        let mut paint = Vec::new();
        body.paint_shadow(rect, body.rounding, &mut paint);
        body.paint_body(rect, body.rounding, &style, body.blur, &mut paint);
        ui.context.paint_blur(
            id.with("blur"),
            id,
            rect.intersect(viewport),
            crate::Blur::new(rect)
                .radius(body.blur)
                .corner_radius(body.rounding),
        );
        ui.context
            .paint(id.with("body"), id, rect.intersect(viewport), paint);
        let mut child_style = style.clone();
        child_style.text_color = body.text_color;
        let mut child = Ui {
            flow: None,
            context: ui.context,
            window: id,
            scope: id,
            sequence: 0,
            clip: padding.inset(rect).intersect(viewport),
            layout: LayoutCursor::new(
                padding.inset(full),
                Layout::Vertical,
                component.spacing.unwrap_or(2.0),
            ),
            enabled: *open && ui.enabled,
            backdrop_blur: body.blur,
            hover_style: ui.hover_style,
            local_style: Some(std::sync::Arc::new(child_style)),
            local_style_revision: ui.local_style_revision,
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
