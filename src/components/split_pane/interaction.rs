use super::{allocation, SplitBoundaryOutput, SplitPanel, SplitSize, SplitStyle, Ui};
use crate::{Id, Layout, Rect, Vec2};
use std::collections::HashMap;
use winit::keyboard::{KeyCode, ModifiersState};

pub(crate) enum SplitInput {
    Begin(Vec2, bool),
    Move(Vec2),
    End(Vec2),
    Key(KeyCode, ModifiersState),
    /// A size for the panel before the boundary, from assistive technology; clamped like
    /// any other resize.
    Set(f32),
}
pub(super) struct Drag {
    pub id: Id,
    pair: (Id, Id),
    pointer: f32,
    last_pointer: f32,
    size: f32,
}
#[derive(Default)]
pub(crate) struct SplitState {
    pub last_frame: u64,
    pub prefs: HashMap<Id, SplitSize>,
    pub sizes: HashMap<Id, f32>,
    pub axis: Option<Layout>,
    pub(super) allocation: Option<(Id, Vec<f32>)>,
    pub(super) drag: Option<Drag>,
}
pub(super) fn boundary(id: Id, a: Id, b: Id) -> Id {
    id.with(("boundary", a, b))
}

pub(super) fn inputs(
    ui: &mut Ui<'_>,
    id: Id,
    axis: Layout,
    panels: &[SplitPanel],
    sizes: &mut [f32],
    state: &mut SplitState,
    style: SplitStyle,
    enabled: bool,
    double_reset: bool,
) -> (Vec<SplitBoundaryOutput>, bool) {
    let mut ended = false;
    if state.drag.as_ref().is_some_and(|d| {
        state.axis != Some(axis)
            || !enabled
            || !panels
                .windows(2)
                .any(|p| (p[0].id, p[1].id) == d.pair && p[0].resizable_after)
    }) {
        ui.context
            .cancel_split_capture(state.drag.as_ref().unwrap().id);
        state.drag = None;
        ended = true;
    }
    let mut outputs = Vec::new();
    for i in 0..panels.len().saturating_sub(1) {
        let a = panels[i].id;
        let b = panels[i + 1].id;
        let bid = boundary(id, a, b);
        let enabled = enabled && panels[i].resizable_after;
        let mut out = SplitBoundaryOutput {
            id: bid,
            before: a,
            after: b,
            bounds: Rect::default(),
            enabled,
            hovered: false,
            focused: false,
            resize_started: false,
            changed: false,
            resize_ended: false,
            cancelled: false,
        };
        let mut events = ui.context.take_split_input(bid);
        super::access::requests(ui, bid, axis, &mut events);
        if enabled {
            if let Some(d) = state.drag.as_mut().filter(|d| d.id == bid) {
                // Authoritative external changes, constraints, or viewport changes
                // replace the baseline at the last consumed pointer, never at the new one.
                if [a, b].iter().enumerate().any(|(n, id)| {
                    state
                        .sizes
                        .get(id)
                        .is_some_and(|old| (*old - sizes[i + n]).abs() > 0.01)
                }) {
                    d.pointer = d.last_pointer;
                    d.size = sizes[i];
                }
            }
            for event in events {
                let before = (sizes[i], sizes[i + 1]);
                match event {
                    SplitInput::Begin(p, double) => {
                        out.resize_started = true;
                        if double && double_reset {
                            reset_pair(panels, sizes, i);
                        }
                        let pointer = axis.main(p);
                        state.drag = Some(Drag {
                            id: bid,
                            pair: (a, b),
                            pointer,
                            last_pointer: pointer,
                            size: sizes[i],
                        });
                    }
                    SplitInput::Move(p) | SplitInput::End(p) => {
                        let end = matches!(event, SplitInput::End(_));
                        if let Some(d) = state.drag.as_mut().filter(|d| d.id == bid) {
                            let pointer = axis.main(p);
                            // Move in physical-pixel steps, including an enclosing visual
                            // scale. Fractional raster phases otherwise make borders and
                            // translated text shimmer during a slow drag.
                            let transform = ui.context.input_transforms.get(&bid);
                            let scale =
                                ui.context.scale_factor() * transform.map_or(1.0, |t| t.scale);
                            let delta = ((pointer as f64 - d.pointer as f64) * scale as f64)
                                .round()
                                / scale as f64;
                            allocation::resize(panels, sizes, i, (d.size as f64 + delta) as f32);
                            d.last_pointer = pointer;
                            if end {
                                state.drag = None;
                                out.resize_ended = true;
                            }
                        }
                    }
                    SplitInput::Key(key, mods) => {
                        let step = if mods.shift_key() {
                            style.keyboard_large_step
                        } else {
                            style.keyboard_step
                        };
                        let delta = match (axis, key) {
                            (Layout::Horizontal, KeyCode::ArrowLeft)
                            | (Layout::Vertical, KeyCode::ArrowUp) => -step,
                            (Layout::Horizontal, KeyCode::ArrowRight)
                            | (Layout::Vertical, KeyCode::ArrowDown) => step,
                            _ => 0.0,
                        };
                        if matches!(key, KeyCode::Home | KeyCode::Backspace) {
                            reset_pair(panels, sizes, i);
                        } else {
                            allocation::resize(panels, sizes, i, sizes[i] + delta);
                        }
                        rebase(state, bid, sizes[i]);
                    }
                    SplitInput::Set(size) => {
                        if size.is_finite() {
                            allocation::resize(panels, sizes, i, size);
                        }
                        rebase(state, bid, sizes[i]);
                    }
                }
                if before != (sizes[i], sizes[i + 1]) {
                    out.changed = true;
                }
            }
        }
        if state.drag.as_ref().is_some_and(|d| d.id == bid) && !ui.context.active(bid) {
            state.drag = None;
            out.resize_ended = true;
            out.cancelled = true;
        }
        if out.changed {
            remember(panels, sizes, state);
        }
        outputs.push(out);
    }
    (outputs, ended)
}
/// An adjustment that did not come from the pointer rebases the drag that holds `boundary`.
fn rebase(state: &mut SplitState, boundary: Id, size: f32) {
    if let Some(d) = state.drag.as_mut().filter(|d| d.id == boundary) {
        d.pointer = d.last_pointer;
        d.size = size;
    }
}
fn reset_pair(panels: &[SplitPanel], sizes: &mut [f32], i: usize) {
    // Reset the ratio within this pair; all other panels keep their allocation.
    let initial: Vec<_> = panels.iter().map(|p| p.initial).collect();
    let defaults = allocation::resolve(panels, &initial, sizes.iter().sum());
    let pair = sizes[i] + sizes[i + 1];
    let base = defaults[i] + defaults[i + 1];
    let desired = if base > 0.0 {
        pair * defaults[i] / base
    } else {
        pair * 0.5
    };
    allocation::resize(panels, sizes, i, desired);
}
fn remember(panels: &[SplitPanel], sizes: &[f32], state: &mut SplitState) {
    for (p, &size) in panels.iter().zip(sizes) {
        if p.controlled.is_some() {
            continue;
        }
        let pref = match p.initial {
            SplitSize::Pixels(_) => SplitSize::Pixels(size),
            _ => SplitSize::Weight(size.max(f32::MIN_POSITIVE)),
        };
        state.prefs.insert(p.id, pref);
    }
}
