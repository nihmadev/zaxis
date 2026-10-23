//! Wrap through the public API: packing, placement of paint and input, lifecycle.
use crate::prelude::*;
use winit::{dpi::PhysicalSize, event::ElementState};
use zaxis::{Align, Padding, Rect, Root, Sense, Wrap, WrapOutput};

mod align;
mod content;
mod items;
mod lifecycle;
mod nesting;
mod packing;
mod regressions;
mod text_input;

pub(crate) fn setup() -> Context {
    setup_at(1.0)
}

pub(crate) fn setup_at(dpi: f64) -> Context {
    let mut c = Context::new();
    c.set_viewport(
        PhysicalSize::new((800.0 * dpi) as u32, (600.0 * dpi) as u32),
        dpi,
    );
    let mut style = c.style().clone();
    style.motion.reduced_motion = true;
    style.text_edit_blink_interval = std::time::Duration::ZERO;
    c.set_style(style);
    c
}

/// A custom control of a given size with a hit region, like any external widget.
pub(crate) fn block(ui: &mut Ui<'_>, key: &'static str, width: f32, height: f32) -> Response {
    let rect = ui.allocate_space(vec2(width, height));
    ui.interact(rect, key, Sense::CLICK | Sense::FOCUS)
}

pub(crate) fn root<R>(c: &mut Context, build: impl FnOnce(&mut Ui<'_>) -> R) -> R {
    let mut out = None;
    c.run(|c| {
        Root::new().padding(Padding::all(0.0)).show(c, |ui| {
            out = Some(build(ui));
        })
    });
    out.unwrap()
}

/// One pass of a wrap of `sizes` (keys `"0"`, `"1"`, ...) at `width`, with the gap given.
pub(crate) fn wrap_blocks(
    c: &mut Context,
    width: f32,
    gap: Vec2,
    sizes: &[(f32, f32)],
) -> (WrapOutput<Vec<Id>>, Vec<Rect>) {
    let out = root(c, |ui| {
        Wrap::new("w").width(width).spacing(gap).show(ui, |ui| {
            sizes
                .iter()
                .enumerate()
                .map(|(i, &(w, h))| {
                    let key: &'static str = Box::leak(i.to_string().into_boxed_str());
                    block(ui, key, w, h).id
                })
                .collect::<Vec<_>>()
        })
    });
    let rects = out.inner.iter().map(|&id| placed(c, id)).collect();
    (out, rects)
}

/// The rectangle the last pass published for `id`: where input reaches it.
pub(crate) fn placed(c: &Context, id: Id) -> Rect {
    c.probe()
        .previous_hits
        .iter()
        .find(|hit| hit.id == id)
        .unwrap_or_else(|| panic!("no hit region for {id:?}"))
        .rect
}

/// Press and release the pointer on the middle of what `id` shows.
pub(crate) fn click_on(c: &mut Context, id: Id) {
    let p = placed(c, id).center();
    click_at(c, p);
}

pub(crate) fn click_at(c: &mut Context, p: Vec2) {
    c.move_pointer(p);
    c.primary_button(ElementState::Pressed);
    c.primary_button(ElementState::Released);
}

pub(crate) fn near(a: Rect, b: Rect) {
    let d = (a.min - b.min)
        .abs()
        .max_element()
        .max((a.max - b.max).abs().max_element());
    assert!(d < 0.01, "{a:?} != {b:?}");
}

pub(crate) fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_min_size(vec2(x, y), vec2(w, h))
}
