//! Resolved look and the primitives the list draws itself: row surfaces, check boxes, section
//! headers, separators and the sticky header.
use super::ListBoxStyle;
use crate::{
    components::{font_size, Style, Ui},
    context::{HitAction, HitRegion, Paint},
    Border, Color, CornerRadius, FontWeight, Id, Padding, Rect, Shape, Vec2,
};

/// Style fields with every fallback applied, snapped to the physical pixel grid.
#[derive(Clone)]
pub(super) struct Look {
    pub row: f32,
    pub min_row: f32,
    pub header: f32,
    pub separator: f32,
    pub margin: f32,
    pub pad: Padding,
    pub rounding: CornerRadius,
    pub idle: Color,
    pub hover: Color,
    pub selected: Color,
    pub selected_inactive: Color,
    pub ring: Border,
    pub stripe: Color,
    pub divider: Border,
    pub text: Color,
    pub selected_text: Color,
    pub muted: Color,
    pub disabled: Color,
    pub header_fill: Color,
    pub header_text: Color,
    pub header_font: f32,
    pub font: f32,
    pub weight: FontWeight,
    pub check: f32,
    pub accent: Color,
    pub on_accent: Color,
    pub scale: f32,
}

impl Look {
    pub fn resolve(
        style: &Style,
        s: &ListBoxStyle,
        density: f32,
        measured: Option<f32>,
        scale: f32,
    ) -> Self {
        let snap = |v: f32| ((v * scale).round() / scale).max(1.0 / scale);
        let base = s.row_height.unwrap_or(style.control_height) * density;
        let row = snap(measured.map_or(base, |m| m * density).max(1.0));
        Self {
            row,
            min_row: snap(base.max(1.0)),
            header: snap(s.header_height.map_or(row, |h| h.max(1.0))),
            separator: snap(s.separator_height.unwrap_or(9.0).max(1.0)),
            margin: s.row_margin.unwrap_or(1.0).max(0.0),
            pad: s.row_padding.unwrap_or(Padding::symmetric(8.0, 2.0)),
            rounding: s.row_rounding.unwrap_or(style.rounding),
            idle: s.idle.unwrap_or(Color::TRANSPARENT),
            hover: s
                .hover
                .unwrap_or_else(|| style.button_hovered.with_opacity(0.55)),
            selected: s.selected.unwrap_or(style.selected_fill),
            // Same as `selected` unless styled: focus moves through a frame without an owner
            // when a row is pressed, and a dimmer color would flash on every selected row.
            selected_inactive: s
                .selected_inactive
                .unwrap_or(s.selected.unwrap_or(style.selected_fill)),
            ring: s.active_ring.unwrap_or(style.focus_border),
            stripe: s.stripe.unwrap_or(Color::TRANSPARENT),
            divider: s.divider.unwrap_or(Border::NONE),
            text: s.text.unwrap_or(style.text_color),
            selected_text: s.selected_text.unwrap_or(style.selected_text),
            muted: s.muted_text.unwrap_or(style.muted_text),
            disabled: s.disabled_text.unwrap_or(style.disabled_text),
            header_fill: s.header_fill.unwrap_or(style.window_fill),
            header_text: s.header_text.unwrap_or(style.muted_text),
            header_font: font_size(s.header_font_size.unwrap_or(style.typography.small)),
            font: font_size(s.font_size.unwrap_or(style.typography.small)),
            weight: s.font_weight.unwrap_or(style.typography.weights.body),
            check: s.check_size.unwrap_or(16.0).clamp(8.0, 64.0),
            accent: style.accent,
            on_accent: style.on_accent,
            scale,
        }
    }
    pub fn highlight(&self, row: Rect) -> Rect {
        let inset = Vec2::splat(self.margin);
        Rect::from_min_max(row.min + inset, (row.max - inset).max(row.min + inset))
    }
}

fn shape(ui: &mut Ui<'_>, rect: Rect, fill: Color, radius: CornerRadius, border: Border) {
    if fill.0[3] > 0 || border.width > 0.0 {
        ui.paint(Shape::rect(rect, fill).corner_radius(radius).border(border));
    }
}

pub(super) struct RowPaint {
    pub selected: bool,
    pub hovered: bool,
    pub striped: bool,
    pub list_focused: bool,
}
pub(super) fn row_surface(ui: &mut Ui<'_>, row: Rect, look: &Look, state: RowPaint) {
    let fill = if state.selected {
        if state.list_focused {
            look.selected
        } else {
            look.selected_inactive
        }
    } else if state.hovered {
        look.hover
    } else if state.striped {
        look.stripe
    } else {
        look.idle
    };
    shape(ui, look.highlight(row), fill, look.rounding, Border::NONE);
    if look.divider.width > 0.0 {
        let line = Rect::from_min_size(
            Vec2::new(row.min.x, row.max.y - look.divider.width),
            Vec2::new(row.size().x, look.divider.width),
        );
        ui.paint(Shape::rect(line, look.divider.color));
    }
}
pub(super) fn ring(ui: &mut Ui<'_>, row: Rect, look: &Look) {
    shape(
        ui,
        look.highlight(row),
        Color::TRANSPARENT,
        look.rounding,
        look.ring,
    );
}

/// A check box of `look.check` at `left`, vertically centered in `row`.
pub(super) fn check_box(
    ui: &mut Ui<'_>,
    row: Rect,
    left: f32,
    checked: bool,
    color: Color,
    look: &Look,
) {
    let side = look.check;
    let rect = Rect::from_min_size(
        Vec2::new(left, row.center().y - side * 0.5),
        Vec2::splat(side),
    );
    let radius = CornerRadius::all((side * 0.22).max(2.0));
    if checked {
        shape(ui, rect, look.accent, radius, Border::NONE);
        let (c, k) = (rect.center(), side);
        let points = [
            c + Vec2::new(-0.25 * k, 0.0),
            c + Vec2::new(-0.07 * k, 0.18 * k),
            c + Vec2::new(0.27 * k, -0.2 * k),
        ];
        for pair in points.windows(2) {
            ui.paint(Shape::Line {
                start: pair[0],
                end: pair[1],
                width: (side * 0.12).max(1.5),
                color: look.on_accent,
            });
        }
    } else {
        shape(
            ui,
            rect,
            Color::TRANSPARENT,
            radius,
            Border::new(1.5, color),
        );
    }
}

/// Text on one line, vertically centered in `area` and clipped to it.
pub(super) fn text(
    ui: &mut Ui<'_>,
    text: &str,
    area: Rect,
    (size, weight, color): (f32, FontWeight, Color),
    center: bool,
) {
    if text.is_empty() || area.is_empty() {
        return;
    }
    let measured = ui.context.measure_text(text, size, weight, f32::INFINITY);
    let offset = ui.context.centered_line_offset(text, size, weight);
    let x = if center {
        area.center().x - measured.x * 0.5
    } else {
        area.min.x
    };
    let id = ui.next_id("text");
    ui.context.paint(
        id,
        ui.window,
        ui.clip.intersect(area),
        vec![Paint::Text {
            text: text.to_owned(),
            position: Vec2::new(x, area.center().y - measured.y * 0.5 + offset),
            size,
            weight,
            wrap_width: f32::INFINITY,
            color,
        }],
    );
}

pub(super) fn header(ui: &mut Ui<'_>, row: Rect, label: &str, look: &Look) {
    ui.paint(Shape::rect(row, look.header_fill));
    let area = Rect::from_min_max(
        Vec2::new(row.min.x + look.margin + look.pad.left, row.min.y),
        Vec2::new(row.max.x - look.margin - look.pad.right, row.max.y),
    );
    let font = (look.header_font, look.weight, look.header_text);
    text(ui, label, area, font, false);
}

pub(super) fn separator(ui: &mut Ui<'_>, row: Rect, look: &Look) {
    let thickness = 1.0 / look.scale;
    let y = (row.center().y * look.scale).floor() / look.scale;
    let color = if look.divider.width > 0.0 {
        look.divider.color
    } else {
        look.muted.with_opacity(0.35)
    };
    let inset = look.margin + look.pad.left;
    ui.paint(Shape::rect(
        Rect::from_min_size(
            Vec2::new(row.min.x + inset, y),
            Vec2::new((row.size().x - 2.0 * inset).max(0.0), thickness),
        ),
        color,
    ));
}

/// The section header pinned over the rows once its own row has scrolled off the top; the
/// next header pushes it out. A block region keeps the rows beneath it from reacting.
pub(super) fn sticky(ui: &mut Ui<'_>, row: Rect, label: &str, look: &Look, id: Id) {
    ui.context.register_hit(HitRegion {
        id,
        window: ui.window,
        rect: row,
        clip: ui.clip,
        action: HitAction::Block,
    });
    header(ui, row, label, look);
}
