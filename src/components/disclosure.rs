//! Shared row composition; action callbacks are measured and placed exactly once.
mod style;
use super::{appearance::Appearance, Image, Response, Ui};
use crate::{
    context::{HitAction, HitRegion, Paint},
    layout::LayoutCursor,
    Border, Color, ControlState, Id, ImageSource, Layout, Padding, Rect, Shape, Vec2,
};
pub use style::{CollapsingStyle, DisclosureStyle, TreeStyle};

pub(super) struct Header<'a> {
    pub id: Id,
    pub label: &'a str,
    pub icon: Option<&'a ImageSource>,
    pub branch: bool,
    pub open: bool,
    pub enabled: bool,
    pub selected: bool,
    pub focus: bool,
    pub indent: f32,
    pub action: HitAction,
    pub chevron_action: Option<HitAction>,
    pub style: &'a DisclosureStyle,
}
pub(super) struct HeaderOutput<R> {
    pub response: Response,
    pub chevron: Rect,
    pub actions: R,
    pub action_ids: Vec<Id>,
}

pub(super) fn child<'a>(ui: &'a mut Ui<'_>, scope: Id, bounds: Rect, layout: Layout) -> Ui<'a> {
    let spacing = ui.style().spacing.max(0.0);
    Ui {
        context: ui.context,
        window: ui.window,
        scope,
        sequence: 0,
        clip: ui.clip.intersect(bounds),
        layout: LayoutCursor::new(bounds, layout, spacing),
        enabled: ui.enabled,
        backdrop_blur: ui.backdrop_blur,
        hover_style: ui.hover_style,
        flow: None,
        local_style: ui.local_style.clone(),
        local_style_revision: ui.local_style_revision,
    }
}

pub(super) fn header<R>(
    ui: &mut Ui<'_>,
    h: Header<'_>,
    actions: impl FnOnce(&mut Ui<'_>) -> R,
) -> HeaderOutput<R> {
    let style = ui.style().clone();
    let s = h.style;
    let height = dimension(s.height.unwrap_or(style.control_height));
    let rect = ui.allocate_space(Vec2::new(ui.available_width().max(0.0), height));
    let padding = s.padding.unwrap_or(Padding::symmetric(6.0, 2.0));
    let mut inside = padding.inset(rect).intersect(rect);
    inside.max = inside.max.max(inside.min);
    inside.min.x = (inside.min.x + h.indent.max(0.0)).min(inside.max.x);
    let side = dimension(s.chevron_size.unwrap_or(12.0))
        .min(inside.size().y)
        .min(inside.size().x);
    let gap = dimension(s.icon_gap.unwrap_or(6.0));
    let chevron = Rect::from_min_size(
        Vec2::new(inside.min.x, inside.center().y - side * 0.5),
        Vec2::splat(side),
    );
    let left = (inside.min.x + side + gap).min(inside.max.x);
    let action_bounds = Rect::from_min_max(Vec2::new(left, inside.min.y), inside.max);
    ui.context.begin_placement(ui.window);
    ui.context.visual_depth += 1;
    let (result, used) = {
        let mut a = child(ui, h.id.with("actions"), action_bounds, Layout::Horizontal);
        a.enabled &= h.enabled;
        a.begin_layout(crate::Align::Center);
        let result = actions(&mut a);
        a.finish_layout();
        (result, a.layout.used)
    };
    ui.context.visual_depth -= 1;
    let placement = ui.context.end_placement();
    let action_ids = ui.context.placement_hit_ids(&placement);
    let width = used.x.max(0.0).min(action_bounds.size().x);
    let action_left = inside.max.x - width;
    let text_right = if width > 0.0 {
        (action_left - gap).max(left)
    } else {
        inside.max.x
    };
    let activation = Rect::from_min_max(
        rect.min,
        Vec2::new(
            if width > 0.0 { action_left } else { rect.max.x },
            rect.max.y,
        ),
    );
    let mut response = ui.response(h.id, activation, h.enabled);
    response.focus_visible |= h.focus && ui.enabled;
    response.has_focus |= h.focus && ui.enabled;
    ui.context.register_hit(HitRegion {
        id: h.id,
        window: ui.window,
        rect: activation,
        clip: ui.clip,
        action: if h.enabled && ui.enabled {
            h.action
        } else {
            HitAction::Block
        },
    });
    if let Some(action) = h
        .chevron_action
        .filter(|_| h.branch && h.enabled && ui.enabled)
    {
        ui.context.register_hit(HitRegion {
            id: h.id.with("chevron-hit"),
            window: ui.window,
            rect: Rect::from_min_max(activation.min, Vec2::new(left, activation.max.y)),
            clip: ui.clip,
            action,
        });
    }
    let mut base = Appearance::new(
        if h.selected {
            style.selected_fill
        } else {
            Color::TRANSPARENT
        },
        Border::NONE,
        if h.selected {
            style.selected_text
        } else {
            style.text_color
        },
    );
    base.rounding = style.rounding;
    base.opacity = style.opacity;
    let mut control = s.surface;
    if control.pressed.fill.is_none() {
        control.pressed.fill = Some(crate::Gradient::new(
            style.button_pressed,
            style.button_pressed,
        ));
    }
    let appearance = ui.animate_control(
        response,
        super::HoverStyle {
            fill: Some(super::HoverFill::Theme),
            ..super::HoverStyle::NONE
        },
        false,
        control,
        ControlState::from_response(response, h.selected),
        base,
        style.button_hovered,
    );
    let mut paint = Vec::new();
    appearance.paint_shadow(rect, appearance.rounding, &mut paint);
    appearance.paint_body(
        rect,
        appearance.rounding,
        &style,
        appearance.blur,
        &mut paint,
    );
    ui.context
        .paint(h.id.with("surface"), ui.window, ui.clip, paint);
    if appearance.blur > 0.0 {
        ui.context.paint_blur(
            h.id.with("blur"),
            ui.window,
            ui.clip,
            crate::Blur::new(rect)
                .radius(appearance.blur)
                .corner_radius(appearance.rounding),
        );
    }
    if h.branch {
        let motion = if style.motion.reduced_motion {
            crate::TweenOptions::new(std::time::Duration::ZERO)
        } else {
            s.motion.clone().unwrap_or(style.motion.expand.clone())
        };
        let open = h.open ^ (h.action == HitAction::Activate && response.clicked());
        let angle = ui.transition_property(
            h.id.with("chevron-angle"),
            rect,
            if open {
                std::f32::consts::FRAC_PI_2
            } else {
                0.0
            },
            motion,
        );
        let c = chevron.center();
        let rotate = |p: Vec2| {
            c + Vec2::new(
                p.x * angle.cos() - p.y * angle.sin(),
                p.x * angle.sin() + p.y * angle.cos(),
            )
        };
        let a = rotate(Vec2::new(-side * 0.2, -side * 0.3));
        let b = rotate(Vec2::new(side * 0.2, 0.0));
        let d = rotate(Vec2::new(-side * 0.2, side * 0.3));
        let lines = [(a, b), (b, d)].map(|(start, end)| {
            Paint::Shape(Shape::Line {
                start,
                end,
                width: dimension(s.chevron_stroke.unwrap_or(1.5)),
                color: appearance.text_color,
            })
        });
        ui.context
            .paint(h.id.with("chevron"), ui.window, ui.clip, lines.to_vec());
    }
    let mut text_left = left;
    if let Some(icon) = h.icon {
        let size = dimension(s.icon_size.unwrap_or(16.0))
            .min(inside.size().y)
            .min((text_right - left).max(0.0));
        let icon_rect = Rect::from_min_size(
            Vec2::new(left, inside.center().y - size * 0.5),
            Vec2::splat(size),
        );
        let mut icon_ui = child(ui, h.id.with("icon"), icon_rect, Layout::Vertical);
        icon_ui.add(Image::new(icon).size(Vec2::splat(size)).placeholder(false));
        text_left = (left + size + gap).min(text_right);
    }
    let font = super::font_size(s.font_size.unwrap_or(style.typography.small));
    let size = ui.context.measure_text(h.label, font, f32::INFINITY);
    let text_clip = Rect::from_min_max(
        Vec2::new(text_left, rect.min.y),
        Vec2::new(text_right, rect.max.y),
    );
    let offset = ui.context.centered_line_offset(h.label, font);
    ui.context.paint(
        h.id.with("label"),
        ui.window,
        ui.clip.intersect(text_clip),
        vec![Paint::Text {
            text: h.label.to_owned(),
            position: Vec2::new(text_left, rect.center().y - size.y * 0.5 + offset),
            size: font,
            wrap_width: f32::INFINITY,
            color: appearance.text_color,
        }],
    );
    ui.context.place(
        placement,
        Vec2::new(action_left - action_bounds.min.x, 0.0),
        ui.clip.intersect(rect),
    );
    HeaderOutput {
        response,
        chevron,
        actions: result,
        action_ids,
    }
}
pub(super) fn dimension(n: f32) -> f32 {
    assert!(
        n.is_finite() && n >= 0.0,
        "disclosure dimensions must be finite and nonnegative"
    );
    n
}
