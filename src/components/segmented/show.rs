use std::{
    collections::{hash_map::DefaultHasher, HashSet},
    hash::{Hash, Hasher},
};

use super::layout::{self, snap, Item, Metrics, Request};
use super::options::{SegmentOption, SegmentWidth, SegmentedOrientation, SegmentedVariant};
use super::{input, paint, SegmentedControl};
use crate::{
    components::{
        appearance::{alpha, resolve_control, Appearance},
        font_size,
        theme::{ControlState, ControlStyle, SurfaceStyle},
        visible_label, HoverStyle, Response, Sense, Tooltip, Ui, Widget,
    },
    context::invalid_value,
    AccessOrientation, AccessRole, Border, Color, CornerRadius, Vec2,
};

impl<T: PartialEq + Hash + Clone> Widget for SegmentedControl<'_, T> {
    fn ui(self, ui: &mut Ui<'_>) -> Response {
        let Self {
            selected,
            mut options,
            cfg,
        } = self;
        let style = ui.style().clone();
        let outlined = cfg.variant == SegmentedVariant::Outlined;
        let mut comp = if outlined {
            style.segmented_outlined
        } else {
            style.segmented
        };
        comp.merge(cfg.style);
        let mark = outlined && comp.mark.unwrap_or(true);
        let enabled_all = cfg.enabled && ui.is_enabled();
        let scale = match ui.context.scale_factor() {
            s if s.is_finite() && s > 0.0 => s,
            _ => 1.0,
        };
        let gid = match cfg.id {
            Some(id) => ui.scope.with(("segmented", id)),
            None => ui.auto_id("segmented"),
        };

        let before = options.len();
        let mut seen = HashSet::new();
        options.retain(|o| seen.insert(o.key()));
        if options.len() != before {
            invalid_value(
                "SegmentedControl::new",
                format!(
                    "{} options repeat an earlier value; ignored",
                    before - options.len()
                ),
            );
        }
        if options.is_empty() {
            invalid_value("SegmentedControl::new", "no options".to_owned());
            let rect = ui.allocate_space(Vec2::ZERO);
            return ui.response(gid, rect, false);
        }

        // Metrics, snapped to physical pixels so edges never straddle a pixel.
        let finite = |v: Option<f32>, d: f32| v.filter(|v| v.is_finite()).map_or(d, |v| v.max(0.0));
        let factor = cfg.size.factor();
        let text_scale = factor.sqrt();
        let padding = snap(
            finite(comp.padding, if outlined { 1.0 } else { 3.0 }),
            scale,
        );
        let total_h = snap(
            (finite(Some(style.control_height), 32.0) * factor).max(1.0),
            scale,
        );
        let metrics = Metrics {
            font: font_size(comp.font_size.unwrap_or(style.font_size) * text_scale),
            weight: comp.font_weight.unwrap_or(style.typography.weights.control),
            pad_x: finite(comp.segment_padding, 12.0),
            icon: finite(comp.icon_size, 16.0) * text_scale,
            icon_gap: finite(comp.icon_gap, 6.0),
            seg_h: (total_h - 2.0 * padding).max(1.0),
            padding,
            gap: snap(finite(comp.gap, if outlined { 0.0 } else { 2.0 }), scale),
            scale,
        };
        let vertical = cfg.orientation == SegmentedOrientation::Vertical;
        let available = ui.available_width();
        let width = match cfg.width {
            SegmentWidth::Fill if !available.is_finite() => SegmentWidth::Equal,
            other => other,
        };
        let items: Vec<Item> = options
            .iter()
            .map(|o| Item {
                text: visible_label(&o.label).to_owned(),
                has_icon: o.icon.is_some(),
            })
            .collect();
        let plan = layout::plan(
            ui.context,
            &items,
            &metrics,
            &Request {
                width,
                min_width: cfg.min_width.unwrap_or(0.0),
                vertical,
                icon_only: cfg.icon_only,
                reserve_mark: mark && !cfg.icon_only,
                available,
            },
        );
        let bounds = ui.allocate_space(plan.size);
        let rects = paint::segment_rects(bounds, &plan.widths, &metrics, vertical);

        // Roving focus: only the active segment is a Tab stop. Keys move it before the hit
        // regions are registered, so the target is focusable in this very pass.
        let keys: Vec<_> = options.iter().map(|o| (gid, "segment", o.key())).collect();
        let ids: Vec<_> = keys.iter().map(|k| ui.interact_id(*k)).collect();
        let enabled: Vec<bool> = options.iter().map(|o| enabled_all && o.enabled).collect();
        let selected_idx = options.iter().position(|o| o.value == *selected);
        let mut selection = selected_idx;
        let mut changed = false;
        let mut focus = ui
            .context
            .focused()
            .and_then(|f| ids.iter().position(|id| *id == f))
            .filter(|i| enabled[*i]);
        if let Some(from) = focus {
            let to = input::target(&ui.context.input().keys_pressed, vertical, from, &enabled);
            if let Some(to) = to {
                focus = Some(to);
                ui.context.request_focus(ids[to]);
                if cfg.follow_focus {
                    *selected = options[to].value.clone();
                    selection = Some(to);
                    changed = true;
                }
            }
        }
        let active = focus.or_else(|| {
            selected_idx
                .filter(|i| enabled[*i])
                .or_else(|| enabled.iter().position(|e| *e))
        });
        let responses: Vec<Response> = (0..options.len())
            .map(|i| {
                let sense = if active == Some(i) {
                    Sense::CLICK | Sense::FOCUS
                } else {
                    Sense::CLICK
                };
                let allowed = cfg.enabled && options[i].enabled;
                ui.add_enabled_ui(allowed, |ui| ui.interact(rects[i], keys[i], sense))
            })
            .collect();
        if !changed {
            let clicked = responses
                .iter()
                .enumerate()
                .position(|(i, r)| r.clicked() && enabled[i] && selection != Some(i));
            if let Some(i) = clicked {
                *selected = options[i].value.clone();
                selection = Some(i);
                changed = true;
            }
        }

        if ui.context.a11y_on() {
            // One value out of several: a radio group, whatever it looks like. A segment
            // drawn as an icon alone is named by its label or, without one, its tooltip.
            let count = options.len();
            let scope = ui.a11y_begin(gid.with("group"), AccessRole::RadioGroup, |node| {
                node.disabled(!enabled_all).orientation(if vertical {
                    AccessOrientation::Vertical
                } else {
                    AccessOrientation::Horizontal
                });
            });
            for i in 0..count {
                ui.a11y(ids[i], rects[i], AccessRole::RadioButton, |node| {
                    let name = match (items[i].text.as_str(), &options[i].tooltip) {
                        ("", Some(tooltip)) => tooltip.as_str(),
                        (text, _) => text,
                    };
                    node.label(name)
                        .toggled(selection == Some(i))
                        .position_in_set(i, count)
                        .disabled(!enabled[i])
                        .clicks(ids[i]);
                });
            }
            ui.a11y_end(scope, Some(bounds));
        }

        // The plate: status border and ring, disabled dimming and theme fill in one track.
        let mut whole = ui.response(gid.with("group"), bounds, enabled_all);
        whole.changed = changed;
        whole.has_focus = responses.iter().any(|r| r.has_focus);
        whole.focus_visible = responses.iter().any(|r| r.focus_visible);
        whole.hovered |= responses.iter().any(|r| r.hovered);
        let status = ui.field_status(cfg.status);
        let mut base = Appearance::new(style.button_fill, Border::NONE, style.text_color);
        base.opacity = style.opacity;
        base.rounding = style.rounding;
        base.status = ui.status_color(status);
        let state = ControlState {
            enabled: enabled_all,
            status,
            ..ControlState::default()
        };
        let control = ControlStyle {
            idle: comp.track,
            disabled: SurfaceStyle {
                opacity: Some(0.5),
                ..SurfaceStyle::default()
            },
            ..ControlStyle::default()
        };
        let mut track = ui.animate_control(
            whole,
            HoverStyle::NONE,
            false,
            control,
            state,
            base,
            style.button_hovered,
        );
        if let Some(radius) = cfg.rounding {
            track.rounding = radius;
        }
        // A pill radius is "as round as fits": never more than half the short side.
        let cap = bounds.size().x.min(bounds.size().y) * 0.5;
        let cap = |r: f32| r.min(cap);
        track.rounding = CornerRadius {
            top_left: cap(track.rounding.top_left),
            top_right: cap(track.rounding.top_right),
            bottom_right: cap(track.rounding.bottom_right),
            bottom_left: cap(track.rounding.bottom_left),
        };
        let mut paint_list = Vec::new();
        track.paint_shadow(bounds, track.rounding, &mut paint_list);
        track.paint_body(bounds, track.rounding, &style, 0.0, &mut paint_list);
        ui.context
            .paint(gid.with("track"), ui.window, ui.clip, paint_list);

        // The thumb slides in plate-local coordinates, so moving the container never drags
        // it. The channel is keyed by the option set and the pixel geometry: a first frame,
        // a different option set or a resize places it without animation.
        let end_radius = (track.rounding.top_left - padding).max(0.0);
        let thumb_radius = comp.thumb.rounding.unwrap_or(CornerRadius::all(end_radius));
        let count = rects.len();
        if outlined {
            // Material 3: no sliding thumb. Each segment fades its tonal (selected) or state
            // layer (hover, press) fill in place; only the row's ends are round.
            let tonal = comp.thumb.fill.map_or(style.selected_fill, |g| g.start);
            for i in 0..count {
                let r = responses[i];
                let target = if selection == Some(i) {
                    tonal
                } else if r.enabled && r.pressed {
                    alpha(style.text_color, 0.12)
                } else if r.enabled && r.hovered {
                    alpha(style.text_color, 0.08)
                } else {
                    alpha(style.text_color, 0.0)
                };
                let fill = ui
                    .transition((gid, "fill", keys[i].2), target, style.motion.hover.clone())
                    .value;
                let rounding = paint::end_rounding(i, count, end_radius, vertical);
                paint::paint_fill(
                    ui,
                    ids[i].with("fill"),
                    rects[i],
                    rounding,
                    alpha(fill, track.opacity),
                );
            }
            if let Some(divider) = comp.divider {
                let divider = Border {
                    color: alpha(divider.color, track.opacity),
                    ..divider
                };
                paint::paint_dividers(
                    ui,
                    gid.with("dividers"),
                    &rects,
                    divider,
                    vertical,
                    metrics.gap,
                );
            }
        } else if let Some(i) = selection {
            let mut hasher = DefaultHasher::new();
            ids.hash(&mut hasher);
            vertical.hash(&mut hasher);
            for w in &plan.widths {
                ((w * scale).round() as i64).hash(&mut hasher);
            }
            ((metrics.seg_h * scale).round() as i64).hash(&mut hasher);
            let layout_key = hasher.finish();
            let spring = style.motion.spring;
            let at = ui
                .spring_transition(
                    (gid, "thumb-at", layout_key),
                    rects[i].min - bounds.min,
                    spring,
                )
                .value
                .value;
            let size = ui
                .spring_transition((gid, "thumb-size", layout_key), rects[i].size(), spring)
                .value
                .value;
            let mut thumb = Appearance::new(style.button_fill, Border::NONE, Color::TRANSPARENT);
            thumb.apply(comp.thumb);
            thumb.opacity *= track.opacity;
            thumb.rounding = thumb_radius;
            let rect = crate::Rect::from_min_size(bounds.min + at, size.max(Vec2::ZERO));
            paint::paint_thumb(ui, gid.with("thumb"), rect, thumb);
        }
        if let Some(i) = active.filter(|i| responses[*i].focus_visible) {
            let ring = if outlined {
                paint::end_rounding(i, count, end_radius, vertical)
            } else {
                thumb_radius
            };
            paint::paint_focus_ring(ui, gid.with("ring"), rects[i], ring, style.focus_border);
        }

        for (i, option) in options.into_iter().enumerate() {
            let response = responses[i];
            let SegmentOption { icon, tooltip, .. } = option;
            let selected_now = selection == Some(i);
            let state = ControlState {
                enabled: response.enabled,
                hovered: response.enabled && response.hovered,
                pressed: response.enabled && response.pressed,
                selected: selected_now,
                ..ControlState::default()
            };
            let raised = state.hovered || state.pressed || selected_now;
            let idle = if raised {
                style.text_color
            } else {
                style.muted_text
            };
            let color = resolve_control(
                &style,
                Appearance::new(Color::TRANSPARENT, Border::NONE, idle),
                HoverStyle::NONE,
                true,
                comp.segment,
                state,
                idle,
            )
            .text_color;
            let color = ui
                .transition((gid, "color", keys[i].2), color, style.motion.hover.clone())
                .value;
            let color = alpha(color, track.opacity);
            let drawn = plan.labels[i].as_ref();
            let lead = match icon {
                _ if mark && selected_now => paint::Lead::Check,
                Some(icon) => paint::Lead::Icon(icon),
                None => paint::Lead::None,
            };
            paint::paint_content(ui, ids[i], rects[i], lead, drawn, color, &metrics);
            let cut = drawn.is_none_or(|l| l.truncated);
            let tip = tooltip
                .or_else(|| (cut && !items[i].text.is_empty()).then(|| items[i].text.clone()));
            if let Some(tip) = tip {
                Tooltip::new(tip).show(ui, response);
            }
        }
        whole
    }
}
