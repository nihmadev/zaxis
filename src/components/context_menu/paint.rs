use super::{ContextMenuItem, ContextMenuStyle};
use crate::{
    components::{appearance::Appearance, Response, Ui},
    context::{HitAction, HitRegion, Paint},
    Border, Color, CornerRadius, Id, Rect, Shape, Vec2,
};

pub(super) struct Metrics {
    pub width: f32,
    pub height: f32,
    icon: f32,
    right: f32,
    font: f32,
}
pub(super) fn measure(
    ui: &mut Ui<'_>,
    items: &[ContextMenuItem],
    style: &ContextMenuStyle,
) -> Metrics {
    let font = style.font_size;
    let mut m = Metrics {
        width: style.min_width,
        height: 0.0,
        icon: 0.0,
        right: 0.0,
        font,
    };
    let mut label_width: f32 = 0.0;
    for item in items {
        if item.id.is_none() {
            m.height += style.separator_height;
            continue;
        }
        let mut width = |s: &str| {
            if s.is_empty() {
                0.0
            } else {
                ui.context.measure_text(s, font, f32::INFINITY).x
            }
        };
        m.icon = m.icon.max(width(&item.icon));
        if !item.icon_shapes.is_empty() {
            m.icon = m.icon.max(14.0);
        }
        m.right = m.right.max(width(&item.right_text));
        let left = width(&item.left_text);
        label_width =
            label_width.max(width(&item.text) + if left > 0.0 { left + 6.0 } else { 0.0 });
        m.height += style.row_height;
    }
    if m.icon > 0.0 {
        m.icon = m.icon.max(14.0) + 6.0;
    }
    if m.right > 0.0 {
        m.right += 18.0;
    }
    m.width = m.width.max(12.0 + m.icon + label_width + m.right + 2.0);
    m.width = m.width.ceil();
    m
}

fn text(ui: &mut Ui<'_>, id: Id, rect: Rect, caption: &str, font: f32, color: Color, right: bool) {
    if caption.is_empty() || rect.is_empty() {
        return;
    }
    let size = ui.context.measure_text(caption, font, f32::INFINITY);
    let offset = ui.context.centered_line_offset(caption, font);
    let x = if right {
        rect.max.x - size.x
    } else {
        rect.min.x
    };
    ui.context.paint(
        id,
        ui.window,
        ui.clip.intersect(rect),
        vec![Paint::Text {
            text: caption.to_owned(),
            position: Vec2::new(x, rect.center().y - size.y * 0.5 + offset),
            size: font,
            wrap_width: f32::INFINITY,
            color,
        }],
    );
}

pub(super) fn row(
    ui: &mut Ui<'_>,
    menu: Id,
    item: &ContextMenuItem,
    active: Option<Id>,
    m: &Metrics,
    style: &ContextMenuStyle,
) -> (Option<Response>, bool) {
    let rect = ui.allocate_space(Vec2::new(
        ui.available_width(),
        if item.id.is_some() {
            style.row_height
        } else {
            style.separator_height
        },
    ));
    let Some(item_id) = item.id else {
        let foreground = ui.style().text_color.0;
        let color = Color::rgba(foreground[0], foreground[1], foreground[2], 24);
        ui.paint(Shape::Line {
            start: Vec2::new(rect.min.x + 5.0, rect.center().y),
            end: Vec2::new((rect.max.x - 5.0).max(rect.min.x + 5.0), rect.center().y),
            width: 1.0,
            color,
        });
        return (None, false);
    };
    let id = menu.with(("item", item_id));
    let response = ui.response(id, rect, item.enabled);
    ui.context.register_hit(HitRegion {
        id,
        window: ui.window,
        rect,
        clip: ui.clip,
        action: if response.enabled {
            HitAction::Activate
        } else {
            HitAction::Block
        },
    });
    let effective = ui.style().clone();
    let highlighted = response.enabled && active.map_or(response.hovered, |a| a == item_id);
    let mut base = Appearance::new(
        if highlighted {
            effective.button_hovered
        } else {
            Color::TRANSPARENT
        },
        Border::NONE,
        if response.enabled {
            effective.text_color
        } else {
            Color::rgba(
                effective.text_color.0[0],
                effective.text_color.0[1],
                effective.text_color.0[2],
                90,
            )
        },
    );
    base.rounding = CornerRadius::all(3.0);
    base.opacity = effective.opacity;
    // Menu selection must follow the pointer in this pass, without a trailing tween.
    base.apply(style.item.idle);
    base.apply(if !response.enabled {
        style.item.disabled
    } else if response.pressed {
        style.item.pressed
    } else if highlighted {
        style.item.hover
    } else {
        Default::default()
    });
    let appearance = base;
    let mut paint = Vec::new();
    appearance.paint_body(
        rect,
        appearance.rounding,
        &effective,
        appearance.blur,
        &mut paint,
    );
    ui.context
        .paint(id.with("body"), ui.window, ui.clip.intersect(rect), paint);
    let color = appearance.text_color;
    let muted = if highlighted {
        color
    } else if response.enabled {
        Color::rgba(color.0[0], color.0[1], color.0[2], 160)
    } else {
        color
    };
    let min = rect.min.x + 6.0;
    let max = (rect.max.x - 6.0).max(min);
    let left = if item.left_text.is_empty() {
        0.0
    } else {
        ui.context
            .measure_text(&item.left_text, m.font, f32::INFINITY)
            .x
            + 6.0
    };
    let segment = |a: f32, b: f32| {
        Rect::from_min_max(
            Vec2::new(a.min(max), rect.min.y),
            Vec2::new(b.min(max).max(a.min(max)), rect.max.y),
        )
    };
    text(
        ui,
        id.with("icon"),
        segment(min, min + m.icon - 6.0),
        &item.icon,
        m.font,
        color,
        false,
    );
    if !item.icon_shapes.is_empty() {
        let icon = Rect::from_min_size(Vec2::new(min, rect.center().y - 7.0), Vec2::splat(14.0));
        ui.context.paint(
            id.with("vector-icon"),
            ui.window,
            ui.clip.intersect(icon),
            vec![Paint::Visual {
                paint: item.icon_shapes.iter().cloned().map(Paint::Shape).collect(),
                transform: crate::Transform::around(Vec2::ZERO, 14.0 / 18.0, icon.min),
                opacity: effective.opacity * if response.enabled { 1.0 } else { 0.4 },
            }],
        );
    }
    text(
        ui,
        id.with("left"),
        segment(min + m.icon, min + m.icon + left - 6.0),
        &item.left_text,
        m.font,
        muted,
        false,
    );
    text(
        ui,
        id.with("label"),
        segment(
            min + m.icon + left,
            (max - m.right).max(min + m.icon + left),
        ),
        &item.text,
        m.font,
        color,
        false,
    );
    text(
        ui,
        id.with("right"),
        segment((max - m.right + 18.0).max(min), max),
        &item.right_text,
        m.font,
        muted,
        true,
    );
    (Some(response), response.clicked())
}
