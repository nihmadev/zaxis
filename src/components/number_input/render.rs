use super::*;
use crate::TextEdit;
use crate::{
    context::{HitAction, HitRegion, NumberInputEvent, Paint, TextEditInput},
    Border, Shape, Vec2,
};
use winit::keyboard::{KeyCode, ModifiersState};

fn multiplier(mods: ModifiersState, style: &NumberStyle) -> f64 {
    if mods.shift_key() {
        style.shift_multiplier
    } else if mods.control_key() {
        style.ctrl_multiplier
    } else {
        1.0
    }
}
fn begin<T: Numeric>(state: &mut NumberState, value: T) {
    state.editing = true;
    state.invalid = false;
    state.buffer = value.to_string();
    state.snapshot = state.buffer.clone();
    state.origin = state.buffer.clone();
    state.units = 0.0;
}
fn step<T: Numeric>(
    state: &mut NumberState,
    value: &mut T,
    options: &NumberOptions<'_, T>,
    units: f64,
) {
    let min = *options.range.start();
    let max = *options.range.end();
    if state.origin.is_empty() || state.last_step != options.step.to_string() {
        state.origin = value.to_string();
        state.units = 0.0;
    }
    let origin = state.origin.parse::<T>().unwrap_or(*value);
    state.last_step = options.step.to_string();
    state.units += units;
    *value = origin.offset(options.step, state.units, min, max);
    // At endpoints, reverse motion should respond immediately.
    if (*value == min && units < 0.0) || (*value == max && units > 0.0) {
        state.origin = value.to_string();
        state.units = 0.0;
    }
    state.invalid = false;
}
fn arrow(key: KeyCode) -> f64 {
    match key {
        KeyCode::ArrowUp | KeyCode::ArrowRight => 1.0,
        KeyCode::ArrowDown | KeyCode::ArrowLeft => -1.0,
        KeyCode::PageUp => 10.0,
        KeyCode::PageDown => -10.0,
        _ => 0.0,
    }
}
fn commit<T: Numeric>(buffer: &str, value: &mut T, options: &NumberOptions<'_, T>) -> bool {
    if let Some(next) = T::parse(buffer) {
        *value = next.normalized(*options.range.start(), *options.range.end());
        true
    } else {
        false
    }
}

pub(super) fn show<T: Numeric>(
    ui: &mut Ui<'_>,
    value: &mut T,
    mut options: NumberOptions<'_, T>,
    drag: bool,
) -> Response {
    options.enabled &= ui.is_enabled();
    let id = match options.id {
        Some(id) => ui.scope.with(("number", id)),
        None => ui.auto_id((
            if drag { "drag-value" } else { "number-input" },
            options.source,
        )),
    };
    let mut style = options
        .style
        .clone()
        .unwrap_or_else(|| ui.style().number.clone());
    style.width = options.width.unwrap_or(style.width);
    style.sensitivity = options.sensitivity.unwrap_or(style.sensitivity);
    assert!(
        style.width.is_finite()
            && style.width > 0.0
            && style.height.is_finite()
            && style.height > 0.0
            && style.font_size.is_finite()
            && style.font_size > 0.0
    );
    assert!([
        style.sensitivity,
        style.shift_multiplier,
        style.ctrl_multiplier
    ]
    .iter()
    .all(|n| n.is_finite() && *n > 0.0));
    let before = *value;
    let mut state = ui.context.numbers.remove(&id).unwrap_or_default();
    let was_editing = state.editing;
    state.last_frame = ui.context.frame;
    let focused = ui.context.has_focus(id);
    let external = state.last_value != value.to_string();
    if external || !options.enabled {
        state.buffer = value.to_string();
        state.snapshot = state.buffer.clone();
        state.origin = state.buffer.clone();
        state.units = 0.0;
        state.invalid = false;
        if !options.enabled {
            state.editing = false;
            state.pointer = None;
        }
    }
    if !drag && focused && !state.editing && options.enabled {
        begin(&mut state, *value);
    }
    let events = ui.context.take_number_input(id);
    if drag && options.enabled {
        for event in events {
            match event {
                NumberInputEvent::Pointer(p, 1, _) => {
                    state.pointer = Some(p);
                    state.distance = 0.0;
                    state.origin = value.to_string();
                    state.units = 0.0;
                    state.snapshot = value.to_string();
                }
                NumberInputEvent::Pointer(p, phase, mods) => {
                    if let Some(previous) = state.pointer {
                        let dx = p.x - previous.x;
                        let prior_distance = state.distance;
                        state.distance += dx.abs();
                        state.pointer = Some(p);
                        if state.distance > 3.0 {
                            let motion = if prior_distance <= 3.0 {
                                state.distance - 3.0
                            } else {
                                dx.abs()
                            };
                            step(
                                &mut state,
                                value,
                                &options,
                                f64::from(motion * dx.signum())
                                    * style.sensitivity
                                    * multiplier(mods, &style),
                            );
                        }
                        if phase == 2 {
                            state.pointer = None;
                            if state.distance <= 3.0 {
                                begin(&mut state, *value);
                            }
                        }
                    }
                }
                NumberInputEvent::Key(KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::F2, _) => {
                    begin(&mut state, *value)
                }
                NumberInputEvent::Key(KeyCode::Escape, _) => {
                    if state.pointer.take().is_some() {
                        if let Ok(original) = state.snapshot.parse::<T>() {
                            *value = original;
                        }
                        state.origin = value.to_string();
                        state.units = 0.0;
                    }
                }
                NumberInputEvent::Key(key, mods) => {
                    let units = arrow(key) * multiplier(mods, &style);
                    if units != 0.0 {
                        step(&mut state, value, &options, units);
                    } else if matches!(key, KeyCode::Home | KeyCode::End) {
                        *value = if key == KeyCode::Home {
                            *options.range.start()
                        } else {
                            *options.range.end()
                        };
                        state.origin = value.to_string();
                        state.units = 0.0;
                    }
                }
            }
        }
    }
    let mut submitted = false;
    let mut cancelled = false;
    let mut response = if !drag || state.editing {
        let mut buffer = std::mem::take(&mut state.buffer);
        if !state.editing {
            buffer = options.display(*value);
        }
        let select_all = state.editing && !was_editing;
        let mut handler = |buffer: &mut String, event: &TextEditInput| {
            match event {
                TextEditInput::Focus(true) if !state.editing => {
                    begin(&mut state, *value);
                    *buffer = state.buffer.clone();
                    false
                }
                TextEditInput::Focus(false) => {
                    if state.editing {
                        commit(buffer, value, &options);
                    }
                    state.editing = false;
                    state.invalid = false;
                    *buffer = options.display(*value);
                    false
                }
                TextEditInput::Key(KeyCode::Enter | KeyCode::NumpadEnter, _) => {
                    if commit(buffer, value, &options) {
                        submitted = true;
                        state.invalid = false;
                        state.snapshot = value.to_string();
                        state.origin = state.snapshot.clone();
                        state.units = 0.0;
                        *buffer = value.to_string();
                        if drag {
                            state.editing = false;
                        }
                    } else {
                        state.invalid = true;
                    }
                    true
                }
                TextEditInput::Key(KeyCode::Escape, _) => {
                    if let Ok(original) = state.snapshot.parse::<T>() {
                        *value = original;
                    }
                    *buffer = value.to_string();
                    state.invalid = false;
                    state.origin = buffer.clone();
                    state.units = 0.0;
                    state.snapshot = buffer.clone();
                    cancelled = true;
                    if drag {
                        state.editing = false;
                    }
                    true
                }
                TextEditInput::Key(key @ (KeyCode::ArrowUp | KeyCode::ArrowDown), mods) => {
                    // Incomplete drafts must not turn into zero on a step.
                    if commit(buffer, value, &options) {
                        if state.origin.parse::<T>().ok().is_none_or(|origin| {
                            !origin
                                .offset(
                                    options.step,
                                    state.units,
                                    *options.range.start(),
                                    *options.range.end(),
                                )
                                .same(*value)
                        }) {
                            state.origin = value.to_string();
                            state.units = 0.0;
                        }
                        step(
                            &mut state,
                            value,
                            &options,
                            arrow(*key) * multiplier(*mods, &style),
                        );
                        *buffer = value.to_string();
                    } else {
                        state.invalid = true;
                    }
                    true
                }
                _ if drag && !state.editing => true,
                _ => {
                    state.invalid = false;
                    false
                }
            }
        };
        let mut editor = TextEdit::new(&mut buffer)
            .enabled(options.enabled)
            .width(style.width)
            .height(style.height)
            .padding(style.padding)
            .rounding(style.rounding)
            .font_size(style.font_size);
        editor.exact_id = Some(id);
        editor.select_all = select_all;
        editor.affixes = (options.prefix.clone(), options.suffix.clone());
        editor.event_handler = Some(&mut handler);
        editor = editor.fill(style.fill);
        editor.hovered_fill = Some(style.hovered);
        let response = editor.ui(ui);
        state.buffer = buffer;
        if state.editing && T::parse(&state.buffer).is_none() {
            state.invalid = true;
        }
        response
    } else {
        let rect = ui.allocate_space(Vec2::new(
            style.width.min(ui.available_width()),
            style.height,
        ));
        let response = ui.response(id, rect, options.enabled);
        ui.context.register_hit(HitRegion {
            id,
            window: ui.window,
            rect,
            clip: ui.clip,
            action: if options.enabled {
                HitAction::DragValue
            } else {
                HitAction::Block
            },
        });
        let shown = format!(
            "{}{}{}",
            options.prefix,
            options.display(*value),
            options.suffix
        );
        let inner = style.padding.inset(rect);
        let text_height = ui
            .context
            .measure_text(&shown, style.font_size, f32::INFINITY)
            .y;
        ui.context.paint(
            id.with("body"),
            ui.window,
            ui.clip,
            vec![Paint::Shape(
                Shape::rect(
                    rect,
                    if response.hovered && options.enabled {
                        style.hovered
                    } else {
                        style.fill
                    },
                )
                .corner_radius(style.rounding)
                .border(if response.focus_visible {
                    ui.style().focus_border
                } else {
                    Border::NONE
                })
                .into(),
            )],
        );
        ui.context.paint(
            id.with("text"),
            ui.window,
            ui.clip.intersect(inner),
            vec![Paint::Text {
                text: shown,
                position: Vec2::new(inner.min.x, inner.center().y - text_height * 0.5),
                size: style.font_size,
                wrap_width: f32::INFINITY,
                color: if options.enabled {
                    ui.style().text_color
                } else {
                    ui.style().muted_text
                },
            }],
        );
        response
    };
    if state.invalid && options.enabled {
        ui.context.paint(
            id.with("invalid"),
            ui.window,
            ui.clip,
            vec![Paint::Shape(
                Shape::rect(response.rect, Color::TRANSPARENT)
                    .corner_radius(style.rounding)
                    .border(Border::new(1.0, style.invalid))
                    .into(),
            )],
        );
    }
    response.changed = !before.same(*value);
    response.submitted = submitted;
    response.lost_focus |= state.focused && !focused;
    state.focused = focused;
    state.last_value = value.to_string();
    if submitted || cancelled {
        ui.context.request_repaint();
    }
    ui.context.numbers.insert(id, state);
    response
}
