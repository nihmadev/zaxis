//! Building blocks of the modal surface: absolutely placed regions, the
//! corner close button and the shared paint of chrome.
use super::Ui;
use crate::{
    components::{
        appearance::Appearance, theme::ModalStyle, Button, ButtonVariant, ScrollStyle, Style,
    },
    context::Paint,
    layout::LayoutCursor,
    Align, Id, Layout, Padding, Rect, Shape, Vec2,
};

/// Run `build` in a vertical flow laid out inside `rect`, returning the extent used.
pub(super) fn region<R>(
    ui: &mut Ui<'_>,
    rect: Rect,
    spacing: f32,
    scope: Id,
    build: impl FnOnce(&mut Ui<'_>) -> R,
) -> (R, Vec2) {
    let mut child = Ui {
        flow: None,
        context: &mut *ui.context,
        window: ui.window,
        scope,
        sequence: 0,
        clip: ui.clip,
        layout: LayoutCursor::new(rect, Layout::Vertical, spacing),
        enabled: ui.enabled,
        backdrop_blur: ui.backdrop_blur,
        hover_style: ui.hover_style,
        local_style: ui.local_style.clone(),
        local_style_revision: ui.local_style_revision,
    };
    child.begin_layout(Align::Start);
    let result = build(&mut child);
    child.finish_layout();
    (result, child.layout.used)
}

/// Height an item took in the current flow, without the spacing after it.
pub(super) fn measure_item(ui: &mut Ui<'_>, build: impl FnOnce(&mut Ui<'_>)) -> f32 {
    let start = ui.layout.cursor.y;
    ui.vertical(build);
    (ui.layout.cursor.y - start - ui.layout.spacing).max(0.0)
}

/// Ghost button with a cross glyph. True when it was activated.
pub(super) fn close_button(ui: &mut Ui<'_>, rect: Rect) -> bool {
    let size = rect.size();
    let (clicked, color) = region(ui, rect, 0.0, ui.scope.with("close"), |c| {
        let node = c.context.a11y_len();
        let response = c.add(
            Button::new("")
                .id_source("close")
                .variant(ButtonVariant::Ghost)
                .min_size(size)
                .width(size.x)
                .padding(crate::Padding::all(0.0)),
        );
        if let Some(node) = c.context.a11y_node_mut(node) {
            node.label("Close");
        }
        let style = c.style();
        let color = if response.hovered || response.has_focus {
            style.text_color
        } else {
            style.muted_text
        };
        (response.clicked, color)
    })
    .0;
    let c = rect.center();
    let r = size.x.min(size.y) * 0.22;
    for (a, b) in [
        (Vec2::new(-r, -r), Vec2::new(r, r)),
        (Vec2::new(-r, r), Vec2::new(r, -r)),
    ] {
        ui.context.paint(
            ui.scope.with(("close-glyph", a.x as i32, a.y as i32)),
            ui.window,
            rect,
            vec![Paint::Shape(Shape::Line {
                start: c + a,
                end: c + b,
                width: 1.5,
                color,
            })],
        );
    }
    clicked
}

pub(super) fn paint_surface(
    ui: &mut Ui<'_>,
    id: Id,
    rect: Rect,
    clip: Rect,
    body: Appearance,
    rounding: crate::CornerRadius,
) {
    let mut paint = Vec::new();
    body.paint_shadow(rect, rounding, &mut paint);
    body.paint_body(rect, rounding, ui.style(), body.blur, &mut paint);
    ui.context.paint_blur(
        id.with("surface-blur"),
        ui.window,
        rect.intersect(clip),
        crate::Blur::new(rect)
            .radius(body.blur)
            .corner_radius(rounding),
    );
    ui.context.paint(id.with("surface"), ui.window, clip, paint);
}

/// The few theme values the modal reads.
pub(super) struct Look {
    pub(super) modal: ModalStyle,
    pub(super) presence: crate::TweenOptions,
    pub(super) window_fill: crate::Color,
    pub(super) border: crate::Border,
    pub(super) text_color: crate::Color,
    pub(super) rounding: crate::CornerRadius,
    pub(super) blur_radius: f32,
    pub(super) elevation: crate::Shadow,
    pub(super) opacity: f32,
    pub(super) window_padding: Padding,
    pub(super) spacing: f32,
    pub(super) scroll: ScrollStyle,
    pub(super) control_height: f32,
}
impl Look {
    pub(super) fn of(style: &Style) -> Self {
        Self {
            modal: style.modal,
            presence: style.motion.presence.clone(),
            window_fill: style.window_fill,
            border: style.border,
            text_color: style.text_color,
            rounding: style.rounding,
            blur_radius: style.blur_radius,
            elevation: style.elevation,
            opacity: style.opacity,
            window_padding: style.window_padding,
            spacing: style.spacing,
            scroll: style.scroll,
            control_height: style.control_height,
        }
    }
}
