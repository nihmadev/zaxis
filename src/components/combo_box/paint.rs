use super::{ComboBoxStyle, Response, Ui};
use crate::{
    context::{HitAction, HitRegion, Paint},
    Border, Color, Id, Rect, Shape, TweenOptions, Vec2,
};

fn text(ui: &mut Ui<'_>, id: Id, rect: Rect, caption: &str, size: f32, color: Color) {
    let y = ui.context.centered_line_offset(caption, size);
    let height = ui.context.measure_text(caption, size, f32::INFINITY).y;
    ui.context.paint(
        id,
        ui.window,
        ui.clip.intersect(rect),
        vec![Paint::Text {
            text: caption.to_owned(),
            position: Vec2::new(rect.min.x, rect.center().y - height * 0.5 + y),
            size,
            wrap_width: f32::INFINITY,
            color,
        }],
    );
}
fn line(start: Vec2, end: Vec2, color: Color) -> Paint {
    Paint::Shape(Shape::Line {
        start,
        end,
        width: 1.5,
        color,
    })
}

pub(super) fn trigger(
    ui: &mut Ui<'_>,
    response: Response,
    label: &str,
    allocation: Rect,
    caption: &str,
    progress: f32,
    style: &ComboBoxStyle,
    _motion: TweenOptions,
) {
    let rect = response.rect;
    let effective = ui.style().clone();
    let mut base = crate::components::appearance::Appearance::new(
        style.trigger_fill,
        if response.hovered && response.enabled {
            style.hover_border
        } else {
            style.border
        },
        style.text,
    );
    base.rounding = style.rounding;
    base.blur = 0.0;
    base.opacity = effective.opacity;
    let appearance = ui.animate_control(
        response,
        crate::HoverStyle::NONE,
        false,
        style.trigger,
        crate::ControlState::from_response(response, progress > 0.0),
        base,
        style.trigger_fill,
    );
    let mut paint = Vec::new();
    appearance.paint_shadow(rect, appearance.rounding, &mut paint);
    appearance.paint_body(
        rect,
        appearance.rounding,
        &effective,
        appearance.blur,
        &mut paint,
    );
    let (_, filter) = ui.resolved_blur(appearance.blur, style.trigger.has_blur_override());
    ui.context.paint_blur(
        response.id.with("blur"),
        ui.window,
        ui.clip,
        crate::Blur::new(rect)
            .radius(filter)
            .corner_radius(appearance.rounding),
    );
    ui.context
        .paint(response.id.with("body"), ui.window, ui.clip, paint);
    if !label.is_empty() {
        text(
            ui,
            response.id.with("label"),
            Rect::from_min_size(
                allocation.min,
                Vec2::new(allocation.size().x, style.label_height),
            ),
            label,
            style.label_font_size,
            style.muted_text,
        );
    }
    let inner = style.trigger_padding.inset(rect);
    text(
        ui,
        response.id.with("caption"),
        Rect::from_min_max(
            inner.min,
            Vec2::new((inner.max.x - 20.0).max(inner.min.x), inner.max.y),
        ),
        caption,
        style.font_size,
        if response.enabled {
            appearance.text_color
        } else {
            style.disabled_text
        },
    );
    let center = Vec2::new(rect.max.x - 14.0, rect.center().y);
    let angle = std::f32::consts::PI * progress;
    let rotate = |p: Vec2| {
        center
            + Vec2::new(
                p.x * angle.cos() - p.y * angle.sin(),
                p.x * angle.sin() + p.y * angle.cos(),
            )
    };
    let color = if response.enabled {
        style.muted_text
    } else {
        style.disabled_text
    };
    ui.context.paint(
        response.id.with("chevron"),
        ui.window,
        ui.clip.intersect(rect),
        vec![
            line(
                rotate(Vec2::new(-4.0, -2.0)),
                rotate(Vec2::new(0.0, 2.0)),
                color,
            ),
            line(
                rotate(Vec2::new(0.0, 2.0)),
                rotate(Vec2::new(4.0, -2.0)),
                color,
            ),
        ],
    );
}

pub(super) fn option(
    ui: &mut Ui<'_>,
    id: Id,
    label: &str,
    selected: bool,
    active: bool,
    enabled: bool,
    style: &ComboBoxStyle,
) -> bool {
    let rect = ui.allocate_space(Vec2::new(ui.available_width(), style.row_height));
    let response = ui.response(id, rect, enabled);
    ui.context.register_hit(HitRegion {
        id,
        window: ui.window,
        rect,
        clip: ui.clip,
        action: if enabled && ui.enabled {
            HitAction::Activate
        } else {
            HitAction::Block
        },
    });
    let effective = ui.style().clone();
    let mut base = crate::components::appearance::Appearance::new(
        if selected || (response.enabled && (response.hovered || active)) {
            style.active_fill
        } else {
            Color::TRANSPARENT
        },
        Border::NONE,
        if !enabled {
            style.disabled_text
        } else if selected {
            style.text
        } else {
            style.muted_text
        },
    );
    base.rounding = style.rounding;
    base.opacity = effective.opacity;
    let mut state = crate::ControlState::from_response(response, selected);
    state.focus = active && response.enabled;
    let appearance = ui.animate_control(
        response,
        crate::HoverStyle::NONE,
        false,
        style.option,
        state,
        base,
        style.active_fill,
    );
    let mut paint = Vec::new();
    appearance.paint_shadow(rect, appearance.rounding, &mut paint);
    appearance.paint_body(
        rect,
        appearance.rounding,
        &effective,
        appearance.blur,
        &mut paint,
    );
    ui.context.paint(id.with("body"), ui.window, ui.clip, paint);
    let color = appearance.text_color;
    text(
        ui,
        id.with("caption"),
        Rect::from_min_max(
            rect.min + Vec2::new(6.0, 0.0),
            Vec2::new((rect.max.x - 22.0).max(rect.min.x + 6.0), rect.max.y),
        ),
        label,
        style.font_size,
        color,
    );
    if selected {
        let c = Vec2::new(rect.max.x - 10.0, rect.center().y);
        ui.context.paint(
            id.with("check"),
            ui.window,
            ui.clip.intersect(rect),
            vec![
                line(
                    c + Vec2::new(-4.0, 0.0),
                    c + Vec2::new(-1.0, 3.0),
                    style.check_color,
                ),
                line(
                    c + Vec2::new(-1.0, 3.0),
                    c + Vec2::new(5.0, -3.0),
                    style.check_color,
                ),
            ],
        );
    }
    response.clicked()
}

pub(super) fn empty(ui: &mut Ui<'_>, style: &ComboBoxStyle) {
    let rect = ui.allocate_space(Vec2::new(ui.available_width(), style.row_height));
    let id = ui.next_id("empty");
    text(
        ui,
        id,
        rect,
        "No matches",
        style.font_size,
        style.muted_text,
    );
}
