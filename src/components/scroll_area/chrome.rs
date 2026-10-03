use super::{ScrollArea, Ui};
use crate::{
    context::{HitAction, HitRegion, Paint},
    Border, Color, CornerRadius, Id, Padding, Rect, Vec2,
};

/// Scroll chrome in logical pixels, based on the quiet two-column Rayfield layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollStyle {
    pub thumb: crate::ControlStyle,

    pub padding: Padding,
    pub spacing: f32,
    pub bar_width: f32,
    pub bar_margin: f32,
    pub min_thumb_length: f32,
    pub thumb_color: Color,
    pub thumb_hovered: Color,
    pub hint_size: f32,
    pub hint_color: Color,
}
impl Default for ScrollStyle {
    fn default() -> Self {
        Self {
            thumb: Default::default(),

            padding: Padding::all(2.0),
            spacing: 8.0,
            bar_width: 2.0,
            bar_margin: 6.0,
            min_thumb_length: 18.0,
            thumb_color: Color::gray(159),
            thumb_hovered: Color::gray(230),
            hint_size: 26.0,
            hint_color: Color::rgba(0, 0, 0, 16),
        }
    }
}
impl ScrollStyle {
    /// Dropdown proportions: four-pixel bar and two-pixel row gaps.
    pub fn compact() -> Self {
        Self {
            bar_width: 4.0,
            spacing: 2.0,
            ..Self::default()
        }
    }
}

impl ScrollArea {
    pub(super) fn paint_chrome(
        &self,
        ui: &mut Ui<'_>,
        id: Id,
        outer: Rect,
        viewport: Rect,
        content: Vec2,
        offset: Vec2,
        style: ScrollStyle,
    ) {
        let max = (content - viewport.size()).max(Vec2::ZERO);
        for axis in 0..2 {
            if !self.axes[axis] || max[axis] <= 0.0 || viewport.is_empty() {
                continue;
            }
            let cross = 1 - axis;
            let length = viewport.size()[axis];
            if self.bars && style.bar_width > 0.0 {
                let thumb_length = (length * length / content[axis])
                    .max(style.min_thumb_length.max(0.0))
                    .min(length);
                let travel = length - thumb_length;
                ui.context.scrolling.states.get_mut(&id).unwrap().travel[axis] = travel;
                let mut min = viewport.min;
                min[axis] += travel * offset[axis] / max[axis];
                min[cross] = outer.max[cross]
                    - if cross == 0 {
                        style.padding.right
                    } else {
                        style.padding.bottom
                    }
                    - style.bar_width;
                let mut size = Vec2::splat(style.bar_width);
                size[axis] = thumb_length;
                let thumb = Rect::from_min_size(min, size);
                let thumb_id = id.with(("thumb", axis));
                let mut hit = thumb;
                hit.min[cross] -= style.bar_margin.max(0.0) * 0.5;
                let response = ui.response(thumb_id, hit, ui.enabled);
                let effective = ui.style().clone();
                let mut normal = super::super::appearance::Appearance::new(
                    style.thumb_color,
                    Border::NONE,
                    effective.text_color,
                );
                normal.rounding = CornerRadius::all(style.bar_width * 0.5);
                normal.opacity = effective.opacity;
                if response.pressed {
                    normal.fill = crate::Gradient::new(style.thumb_hovered, style.thumb_hovered);
                }
                let appearance = ui.animate_control(
                    response,
                    crate::HoverStyle::fill(style.thumb_hovered),
                    false,
                    style.thumb,
                    crate::ControlState::from_response(response, false),
                    normal,
                    style.thumb_hovered,
                );
                let mut paint = Vec::new();
                appearance.paint_shadow(thumb, appearance.rounding, &mut paint);
                appearance.paint_body(
                    thumb,
                    appearance.rounding,
                    &effective,
                    appearance.blur,
                    &mut paint,
                );
                if appearance.blur > 0.0 {
                    ui.context.paint_blur(
                        thumb_id.with("blur"),
                        ui.window,
                        ui.clip,
                        crate::Blur::new(thumb)
                            .radius(appearance.blur)
                            .corner_radius(appearance.rounding),
                    );
                }
                ui.context.paint(thumb_id, ui.window, ui.clip, paint);
                if ui.enabled {
                    ui.context.register_hit(HitRegion {
                        id: thumb_id,
                        window: ui.window,
                        rect: hit,
                        clip: ui.clip.intersect(outer),
                        action: HitAction::ScrollThumb { area: id, axis },
                    });
                }
            }
            if self.hints && style.hint_size > 0.0 && offset[axis] < max[axis] - 1.0 {
                let mut hint = viewport;
                hint.min[axis] = (hint.max[axis] - style.hint_size).max(hint.min[axis]);
                ui.context.paint(
                    id.with(("hint", axis)),
                    ui.window,
                    ui.clip.intersect(viewport),
                    vec![Paint::ScrollHint {
                        rect: hint,
                        axis,
                        color: style.hint_color,
                    }],
                );
            }
        }
    }
}
