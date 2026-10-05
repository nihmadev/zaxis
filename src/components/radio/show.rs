use std::collections::HashSet;

use super::layout::{self, snap, Item, Metrics};
use super::paint::{self, Texts};
use super::{input, RadioGroup};
use crate::{
    components::{
        appearance::{alpha, Appearance},
        font_size,
        theme::{ControlPaint, ControlState, PaintPart},
        visible_label, HoverStyle, Response, Sense, Tooltip, Ui, Widget,
    },
    context::invalid_value,
    AccessRole, Border, Color, CornerRadius, Id, Rect, Vec2,
};
use std::hash::Hash;

impl<T: PartialEq + Hash> Widget for RadioGroup<'_, T> {
    fn ui(self, ui: &mut Ui<'_>) -> Response {
        let Self {
            selected,
            mut options,
            cfg,
            painter,
        } = self;
        let style = ui.style().clone();
        let mut comp = style.radio;
        comp.merge(cfg.style);
        let enabled_all = cfg.enabled && ui.is_enabled();
        let scale = match ui.context.scale_factor() {
            s if s.is_finite() && s > 0.0 => s,
            _ => 1.0,
        };
        let gid = match cfg.id {
            Some(id) => ui.scope.with(("radio", id)),
            None => ui.auto_id("radio"),
        };

        let before = options.len();
        let mut seen = HashSet::new();
        options.retain(|o| seen.insert(o.key()));
        if options.len() != before {
            invalid_value(
                "RadioGroup::new",
                format!(
                    "{} options repeat an earlier value; ignored",
                    before - options.len()
                ),
            );
        }
        if options.is_empty() {
            invalid_value("RadioGroup::new", "no options".to_owned());
            let rect = ui.allocate_space(Vec2::ZERO);
            return ui.response(gid, rect, false);
        }

        // Metrics, snapped to physical pixels so edges never straddle a pixel.
        let finite = |v: Option<f32>, d: f32| v.filter(|v| v.is_finite()).map_or(d, |v| v.max(0.0));
        let factor = cfg.size.factor();
        let font = font_size(comp.font_size.unwrap_or(style.font_size) * factor.sqrt());
        let m = Metrics {
            font,
            desc_font: font_size(font * 0.9),
            weight: comp.font_weight.unwrap_or(style.typography.weights.control),
            d: snap((finite(comp.size, 18.0) * factor).clamp(8.0, 128.0), scale),
            inset: snap(finite(comp.halo_size, 3.0) * factor, scale),
            label_gap: snap(cfg.label_gap.or(comp.gap).unwrap_or(10.0), scale),
            row_gap: snap(cfg.row_gap.or(comp.row_gap).unwrap_or(2.0), scale),
            column_gap: snap(cfg.column_gap.or(comp.column_gap).unwrap_or(20.0), scale),
            min_height: cfg
                .min_row_height
                .or(comp.row_height)
                .unwrap_or(style.control_height * factor)
                .max(0.0),
            line_gap: snap(2.0 * factor, scale),
            scale,
        };
        let items: Vec<Item> = options
            .iter()
            .map(|o| Item {
                label: visible_label(&o.label).to_owned(),
                description: o.description.clone(),
            })
            .collect();
        let cols = input::columns(cfg.layout, options.len());
        let plan = layout::plan(ui.context, &items, &m, cols, ui.available_width());
        let bounds = ui.allocate_space(plan.size);
        let rows: Vec<Rect> = plan
            .cells
            .iter()
            .map(|c| c.rect.translate(bounds.min))
            .collect();

        // Roving focus: only the active option is a Tab stop. Keys move it before the hit
        // regions are registered, so the target is focusable in this very pass.
        let keys: Vec<Id> = options.iter().map(|o| o.key()).collect();
        let ids: Vec<Id> = keys
            .iter()
            .map(|k| ui.interact_id((gid, "option", *k)))
            .collect();
        let enabled: Vec<bool> = options.iter().map(|o| enabled_all && o.enabled).collect();
        let selected_idx = options.iter().position(|o| o.value == *selected);
        let mut chosen = None;
        let mut focus = ui
            .context
            .focused_widget
            .and_then(|f| ids.iter().position(|id| *id == f))
            .filter(|i| enabled[*i]);
        if let Some(from) = focus {
            let pressed = &ui.context.input().keys_pressed;
            if let Some(to) = input::target(pressed, cfg.layout, from, &enabled) {
                focus = Some(to);
                ui.context.request_focus(ids[to]);
                if cfg.follow_focus && selected_idx != Some(to) {
                    chosen = Some(to);
                }
            }
        }
        let active = focus
            .or_else(|| selected_idx.filter(|i| enabled[*i]))
            .or_else(|| enabled.iter().position(|e| *e));
        let responses: Vec<Response> = (0..options.len())
            .map(|i| {
                let sense = if active == Some(i) {
                    Sense::CLICK | Sense::FOCUS
                } else {
                    Sense::CLICK
                };
                let allowed = cfg.enabled && options[i].enabled;
                ui.add_enabled_ui(allowed, |ui| {
                    ui.interact(rows[i], (gid, "option", keys[i]), sense)
                })
            })
            .collect();
        if chosen.is_none() {
            chosen = responses
                .iter()
                .enumerate()
                .position(|(i, r)| r.clicked() && enabled[i] && selected_idx != Some(i));
        }
        let selection = chosen.or(selected_idx);
        if ui.context.a11y_on() {
            // A lone `radio_value` is a button on its own; several options are a group whose
            // focused member is the one holding the roving Tab stop.
            let count = options.len();
            let group = (count > 1).then(|| {
                ui.a11y_begin(gid.with("group"), AccessRole::RadioGroup, |node| {
                    node.disabled(!enabled_all);
                })
            });
            for i in 0..count {
                ui.a11y(ids[i], rows[i], AccessRole::RadioButton, |node| {
                    node.label(items[i].label.as_str())
                        .description(items[i].description.as_str())
                        .toggled(selection == Some(i))
                        .disabled(!enabled[i])
                        .clicks(ids[i]);
                    if count > 1 {
                        node.position_in_set(i, count);
                    }
                });
            }
            if let Some(scope) = group {
                ui.a11y_end(scope, Some(bounds));
            }
        }

        let mut whole = ui.response(gid.with("group"), bounds, enabled_all);
        whole.changed = chosen.is_some();
        whole.has_focus = responses.iter().any(|r| r.has_focus);
        whole.focus_visible = responses.iter().any(|r| r.focus_visible);
        whole.hovered |= responses.iter().any(|r| r.hovered);

        let status = ui.field_status(cfg.status);
        let status_color = ui.status_color(status);
        let spring = comp.dot_spring.unwrap_or(style.motion.spring);
        let outline_width = finite(comp.border_width, 1.5);
        let dot_size = finite(comp.dot_size, 8.0) * factor;
        let halo_size = m.inset;
        let clip = ui.clip_rect();
        for i in 0..options.len() {
            let r = responses[i];
            let on = enabled[i];
            let picked = selection == Some(i);
            let state = ControlState {
                enabled: on,
                hovered: on && r.hovered,
                pressed: on && r.pressed,
                selected: picked,
                focus: false,
                status,
            };
            let at = paint::indicator_rect(rows[i], &plan.cells[i], &m);
            let part_response = |name: &str| Response {
                id: ids[i].with(name),
                rect: at,
                ..r
            };

            // Ring: muted, darker on hover, accent when selected, faded when disabled.
            let outline = if !on {
                style.disabled_text
            } else if picked {
                style.accent
            } else if state.hovered || state.pressed {
                style.text_color
            } else {
                style.muted_text
            };
            let mut base = Appearance::new(
                Color::TRANSPARENT,
                Border::new(outline_width, outline),
                style.text_color,
            );
            base.rounding = paint::round(at);
            base.opacity = style.opacity;
            base.status = status_color;
            let body = ui.animate_control(
                part_response("body"),
                HoverStyle::NONE,
                false,
                comp.body,
                state,
                base,
                style.button_hovered,
            );

            // Halo: off by default (Material 3 draws none); `RadioStyle::halo` turns it on.
            let glow = 0.0;
            let mut halo = Appearance::new(alpha(style.accent, glow), Border::NONE, style.accent);
            halo.opacity = style.opacity;
            let halo = ui.animate_control(
                part_response("halo"),
                HoverStyle::NONE,
                false,
                comp.halo,
                state,
                halo,
                style.button_hovered,
            );

            // Dot: always the accent color, scaled by a spring so it grows (and may
            // overshoot) when selected and shrinks when another option takes over.
            let mut dot = Appearance::new(
                if on {
                    style.accent
                } else {
                    style.disabled_text
                },
                Border::NONE,
                style.accent,
            );
            dot.opacity = style.opacity;
            let dot = ui.animate_control(
                part_response("dot"),
                HoverStyle::NONE,
                false,
                comp.dot,
                ControlState {
                    selected: true,
                    hovered: false,
                    pressed: false,
                    status: Default::default(),
                    ..state
                },
                dot,
                style.button_hovered,
            );
            let grow = ui
                .spring_transition(
                    (gid, "dot", keys[i]),
                    if picked { 1.0_f32 } else { 0.0 },
                    spring,
                )
                .value
                .value
                .max(0.0);

            let mut list = Vec::new();
            let info = ControlPaint {
                bounds: at,
                part: PaintPart::RadioIndicator,
                style: body.surface(),
                state,
                value: f32::from(picked),
            };
            paint::part(painter.as_ref(), clip, info, &mut list, |out| {
                let outer = paint::grow(at, halo_size);
                if halo.fill.start.0[3] > 0 {
                    halo.paint_body(outer, paint::round(outer), &style, 0.0, out);
                }
                body.paint_body(at, body.rounding, &style, 0.0, out);
            });
            let side = dot_size.min(m.d) * grow;
            let dot_rect = paint::grow(Rect::from_min_max(at.center(), at.center()), side * 0.5);
            let info = ControlPaint {
                bounds: dot_rect,
                part: PaintPart::RadioDot,
                style: dot.surface(),
                value: grow,
                ..info
            };
            paint::part(painter.as_ref(), clip, info, &mut list, |out| {
                if side > 0.01 {
                    let rounding = CornerRadius::all(side * 0.5);
                    dot.paint_body(dot_rect, rounding, &style, 0.0, out);
                }
            });
            ui.context
                .paint(ids[i].with("indicator"), ui.window, ui.clip, list);
            if r.focus_visible && on {
                let ring = comp.body.focus.border.unwrap_or(style.focus_border);
                paint::paint_focus_ring(ui, ids[i].with("ring"), at, ring);
            }

            let text_color = if on {
                style.text_color
            } else {
                style.disabled_text
            };
            let sub_color = if on {
                style.muted_text
            } else {
                style.disabled_text
            };
            let label_color = ui
                .transition(
                    (gid, "label", keys[i]),
                    text_color,
                    style.motion.hover.clone(),
                )
                .value;
            let texts = Texts {
                label: &items[i].label,
                description: &items[i].description,
                label_color: alpha(label_color, style.opacity),
                description_color: alpha(sub_color, style.opacity),
            };
            paint::paint_texts(ui, ids[i].with("text"), rows[i], &plan.cells[i], &texts, &m);
            if let Some(tip) = options[i].tooltip.clone() {
                Tooltip::new(tip).show(ui, r);
            }
        }
        if let Some(i) = chosen {
            *selected = options.swap_remove(i).value;
        }
        whole
    }
}
