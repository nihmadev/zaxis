//! Drawing of the strip, the tabs and their parts. Lines and fills sit on whole physical
//! pixels, so nothing shimmers at fractional scale factors.

use super::style::Look;
use crate::components::segmented::layout::Label;
use crate::{
    components::Ui, context::Paint, Border, Color, CornerRadius, FontWeight, Id, ImageSource,
    Interpolate, Rect, Shape, Vec2,
};

fn rect(rect: Rect, fill: Color, rounding: CornerRadius) -> Paint {
    Paint::Shape(Shape::Rect {
        rect,
        rounding,
        fill,
        border: Border::NONE,
    })
}

fn snapped(r: Rect, scale: f32) -> Rect {
    let s = |v: f32| (v * scale).round() / scale;
    Rect::from_min_max(
        Vec2::new(s(r.min.x), s(r.min.y)),
        Vec2::new(s(r.max.x), s(r.max.y)),
    )
}

/// The band behind the row and the hairline under all of it.
pub(super) fn strip(ui: &mut Ui<'_>, id: Id, bounds: Rect, look: &Look) {
    let mut paints = Vec::new();
    if let Some(fill) = look.strip_fill {
        paints.push(rect(bounds, fill, CornerRadius::ZERO));
    }
    let line = Rect::from_min_max(
        Vec2::new(bounds.min.x, bounds.max.y - look.line),
        bounds.max,
    );
    paints.push(rect(line, look.baseline, CornerRadius::ZERO));
    ui.context.paint(id, ui.window, ui.clip, paints);
}

/// What one tab looks like right now; every amount is already eased.
pub(super) struct View<'a> {
    pub id: Id,
    /// Where the tab is drawn: its slot, shifted while a gap opens beside it.
    pub slot: Rect,
    pub look: &'a Look,
    /// 1 for the active tab, 0 for the others.
    pub selected: f32,
    pub hover: f32,
    pub pressed: bool,
    pub dim: f32,
    pub focus: Option<Border>,
}

impl View<'_> {
    fn fade(&self, color: Color) -> Color {
        color.with_opacity(self.dim)
    }

    /// The hover plate, inset from the slot so it never touches the hairline.
    fn plate(&self) -> Rect {
        let (inset, scale) = (self.look.plate_inset, self.look.scale);
        snapped(
            Rect::from_min_max(self.slot.min + inset, self.slot.max - inset),
            scale,
        )
    }

    /// The shape that marks the active tab and the plate, for the focus ring.
    fn outline(&self) -> (Rect, CornerRadius) {
        (self.plate(), CornerRadius::all(self.look.plate_radius))
    }
}

/// Surface of a tab: the active folder, the hover plate, or the underline's mark.
pub(super) fn body(ui: &mut Ui<'_>, v: &View<'_>) {
    let look = v.look;
    let mut paints = Vec::new();
    if look.folder {
        if v.hover > 0.0 && v.selected < 1.0 {
            let fill = if v.pressed {
                look.plate_pressed
            } else {
                look.plate
            };
            let alpha = v.hover * (1.0 - v.selected);
            paints.push(rect(
                v.plate(),
                v.fade(fill).with_opacity(alpha),
                CornerRadius::all(look.plate_radius),
            ));
        }
        if v.selected > 0.0 {
            // The tab runs on below the strip, so its bottom edge is clipped away and the
            // fill covers the hairline: the page flows into the tab.
            let r = look.top_radius(v.slot.size().x);
            let body = Rect::from_min_max(
                v.slot.min,
                Vec2::new(v.slot.max.x, v.slot.max.y + look.line),
            );
            paints.push(Paint::Shape(Shape::Rect {
                rect: body,
                rounding: CornerRadius {
                    top_left: r,
                    top_right: r,
                    bottom_right: 0.0,
                    bottom_left: 0.0,
                },
                fill: v.fade(look.page).with_opacity(v.selected),
                border: Border::new(look.line, v.fade(look.frame).with_opacity(v.selected)),
            }));
        }
    } else {
        if v.hover > 0.0 && v.selected < 1.0 {
            let fill = if v.pressed {
                look.plate_pressed
            } else {
                look.plate
            };
            paints.push(rect(
                v.slot,
                v.fade(fill).with_opacity(v.hover * (1.0 - v.selected)),
                CornerRadius::ZERO,
            ));
        }
        if v.selected > 0.0 {
            let thick = (look.indicator_width * look.scale).round().max(1.0) / look.scale;
            let mark =
                Rect::from_min_max(Vec2::new(v.slot.min.x, v.slot.max.y - thick), v.slot.max);
            paints.push(rect(
                mark,
                v.fade(look.indicator).with_opacity(v.selected),
                CornerRadius::ZERO,
            ));
        }
    }
    if let Some(border) = v.focus {
        let (ring, rounding) = if look.folder {
            v.outline()
        } else {
            (v.slot.shrink(2.0), CornerRadius::all(look.plate_radius))
        };
        paints.push(Paint::Shape(Shape::Rect {
            rect: ring,
            rounding,
            fill: Color::TRANSPARENT,
            border,
        }));
    }
    if !paints.is_empty() {
        ui.context
            .paint(v.id.with("body"), ui.window, ui.clip, paints);
    }
}

/// Where the close button's hit area sits in a slot.
pub(super) fn close_rect(slot: Rect, look: &Look) -> Rect {
    let right = slot.max.x - look.pad_x + look.close_overhang();
    let top = slot.min.y + (slot.size().y - look.line - look.close_size) * 0.5;
    snapped(
        Rect::from_min_size(
            Vec2::new(right - look.close_size, top),
            Vec2::splat(look.close_size),
        ),
        look.scale,
    )
}

/// The close button's state on screen.
pub(super) struct Close {
    pub rect: Rect,
    /// How much of it shows: 0 hidden, 1 fully visible.
    pub shown: f32,
    pub hover: f32,
    pub pressed: bool,
    pub enabled: bool,
}

/// Icon, label and close button of a tab.
pub(super) fn content(
    ui: &mut Ui<'_>,
    v: &View<'_>,
    icon: Option<ImageSource>,
    label: Option<&Label>,
    text: Color,
    weight: FontWeight,
    close: Option<Close>,
) {
    let look = v.look;
    let scale = look.scale;
    let center = v.slot.min.y + (v.slot.size().y - look.line) * 0.5;
    let mut x = v.slot.min.x + look.pad_x;
    let color = v.fade(text);
    if let Some(icon) = icon {
        let side = look.icon;
        let glyph = Rect::from_min_size(
            Vec2::new(
                (x * scale).round() / scale,
                ((center - side * 0.5) * scale).round() / scale,
            ),
            Vec2::splat(side),
        );
        ui.paint_image_in(v.id.with("icon"), icon, glyph, color);
        x += side + if label.is_some() { look.icon_gap } else { 0.0 };
    }
    let mut paints = Vec::new();
    if let Some(label) = label {
        let optical = ui
            .context
            .centered_line_offset(&label.text, look.font, weight);
        paints.push(Paint::Text {
            text: label.text.clone(),
            position: Vec2::new(
                (x * scale).round() / scale,
                ((center - label.size.y * 0.5 + optical) * scale).round() / scale,
            ),
            size: look.font,
            weight,
            wrap_width: f32::INFINITY,
            color,
        });
    }
    if let Some(close) = close.filter(|c| c.shown > 0.0) {
        let alpha = close.shown * v.dim;
        if close.hover > 0.0 && close.enabled {
            let plate = if close.pressed {
                look.close_plate_pressed
            } else {
                look.close_plate
            };
            paints.push(rect(
                close.rect,
                plate.with_opacity(close.hover * alpha),
                CornerRadius::all(look.plate_radius),
            ));
        }
        let ink = if close.enabled {
            look.close_idle.interpolate(&look.close_hover, close.hover)
        } else {
            look.text_disabled
        };
        let half = look.close_glyph * 0.5;
        let middle = close.rect.center();
        let width = (look.close_glyph * 0.12).max(1.0);
        for (a, b) in [
            (Vec2::new(-half, -half), Vec2::new(half, half)),
            (Vec2::new(half, -half), Vec2::new(-half, half)),
        ] {
            paints.push(Paint::Shape(Shape::Line {
                start: middle + a,
                end: middle + b,
                width,
                color: ink.with_opacity(alpha),
            }));
        }
    }
    if !paints.is_empty() {
        ui.context
            .paint(v.id.with("content"), ui.window, ui.clip, paints);
    }
}

/// Hairlines between neighbouring tabs; `shown[i]` is how visible the one after tab `i` is.
pub(super) fn dividers(ui: &mut Ui<'_>, id: Id, slots: &[Rect], shown: &[f32], look: &Look) {
    let mut paints = Vec::new();
    for (slot, shown) in slots.iter().zip(shown) {
        if *shown <= 0.0 {
            continue;
        }
        let height = (look.divider_height * look.scale).round() / look.scale;
        let top = slot.min.y + (slot.size().y - look.line - height) * 0.5;
        let x = slot.max.x - look.line * 0.5;
        let line = snapped(
            Rect::from_min_size(
                Vec2::new(x - look.line * 0.5, top),
                Vec2::new(look.line, height),
            ),
            look.scale,
        );
        paints.push(rect(
            line,
            look.divider.with_opacity(*shown),
            CornerRadius::ZERO,
        ));
    }
    if !paints.is_empty() {
        ui.context.paint(id, ui.window, ui.clip, paints);
    }
}

/// The mark of where a dragged tab will land, centered in the gap.
pub(super) fn insertion(ui: &mut Ui<'_>, id: Id, x: f32, strip: Rect, shown: f32, look: &Look) {
    let thick = (look.insertion_width * look.scale).round().max(1.0) / look.scale;
    let inset = look.plate_inset.y;
    let line = snapped(
        Rect::from_min_size(
            Vec2::new(x - thick * 0.5, strip.min.y + inset),
            Vec2::new(thick, strip.size().y - look.line - 2.0 * inset),
        ),
        look.scale,
    );
    ui.context.paint(
        id,
        ui.window,
        ui.clip,
        vec![rect(
            line,
            look.insertion.with_opacity(shown),
            CornerRadius::all(thick * 0.5),
        )],
    );
}

/// The inset outline of an area that would take the dragged tab, eased in by `amount`:
/// it starts a few pixels inside and settles at the area's edge.
pub(super) fn drop_area(ui: &mut Ui<'_>, id: Id, area: Rect, amount: f32, look: &Look) {
    if amount <= 0.003 || area.is_empty() {
        return;
    }
    let inset = 5.0 - 3.5 * amount.clamp(0.0, 1.0);
    let outline = area.shrink(inset);
    ui.context.paint(
        id,
        ui.window,
        ui.clip,
        vec![Paint::Shape(Shape::Rect {
            rect: outline,
            rounding: look.drop_radius,
            fill: look.drop_fill.with_opacity(amount),
            border: Border::new(
                look.drop_border.width,
                look.drop_border.color.with_opacity(amount),
            ),
        })],
    );
}
