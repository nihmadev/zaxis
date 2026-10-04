//! Autoscroll speed, time independence, DPI, limits, nesting and wheel input.
use super::*;
use winit::event::{DeviceId, MouseScrollDelta, TouchPhase, WindowEvent};

struct List {
    context: Context,
    start: Instant,
    ms: f64,
    nested: bool,
    wide: bool,
    height: f32,
    viewport: Rect,
    offset: Vec2,
    inner: Vec2,
    source: Rect,
}

fn list(scale: f64, nested: bool, wide: bool) -> List {
    let mut context = Context::new();
    context.set_viewport(
        PhysicalSize::new((800.0 * scale) as u32, (600.0 * scale) as u32),
        scale,
    );
    let mut list = List {
        context,
        start: Instant::now(),
        ms: 0.0,
        nested,
        wide,
        height: 120.0,
        viewport: Rect::default(),
        offset: Vec2::ZERO,
        inner: Vec2::ZERO,
        source: Rect::default(),
    };
    list.frame(0.0);
    list.frame(16.0);
    list
}

impl List {
    fn frame(&mut self, dt: f64) {
        self.ms += dt;
        let now = self.start + Duration::from_secs_f64(self.ms / 1000.0);
        let (nested, wide, height) = (self.nested, self.wide, self.height);
        let (mut viewport, mut offset, mut inner, mut source) =
            (Rect::default(), Vec2::ZERO, Vec2::ZERO, Rect::default());
        self.context.run_at(now, |c| {
            Root::new().padding(Padding::all(0.0)).show(c, |ui| {
                source = ui
                    .drag_source(Id::new("item"), Item(1), |ui| ui.button("Drag"))
                    .inner
                    .rect;
                let rows = |ui: &mut crate::Ui<'_>, n: usize| {
                    for i in 0..n {
                        ui.label(format!("Row {i} with some text in it"));
                    }
                    if wide {
                        ui.allocate_space(Vec2::new(1200.0, 1.0));
                    }
                };
                let area = if wide {
                    ScrollArea::both()
                } else {
                    ScrollArea::vertical()
                };
                if nested {
                    let outer = ScrollArea::vertical()
                        .id_source("outer")
                        .max_height(80.0)
                        .show(ui, |ui| {
                            let i = ScrollArea::vertical()
                                .id_source("inner")
                                .max_height(80.0)
                                .show(ui, |ui| rows(ui, 20));
                            inner = i.offset;
                            ui.add_space(300.0);
                        });
                    viewport = outer.viewport;
                    offset = outer.offset;
                } else {
                    let out = area
                        .id_source("list")
                        .max_height(height)
                        .content_width(1200.0)
                        .show(ui, |ui| rows(ui, 40));
                    viewport = out.viewport;
                    offset = out.offset;
                }
            });
        });
        (self.viewport, self.offset, self.inner, self.source) = (viewport, offset, inner, source);
    }

    fn drag_to(&mut self, p: Vec2) {
        let from = self.source.center();
        self.context.move_pointer(from);
        self.context.primary_button(ElementState::Pressed);
        self.context.move_pointer(from + Vec2::new(0.0, 8.0));
        self.frame(16.0);
        self.frame(16.0);
        self.context.move_pointer(p);
    }

    fn run(&mut self, seconds: f64, step: f64) {
        for _ in 0..(seconds * 1000.0 / step).round() as usize {
            self.frame(step);
        }
    }
}

fn near_bottom(l: &List, inset: f32) -> Vec2 {
    Vec2::new(l.viewport.min.x + 20.0, l.viewport.max.y - inset)
}

#[test]
fn speed_grows_with_proximity_and_does_not_depend_on_frame_rate_or_dpi() {
    let mut reached = Vec::new();
    for (scale, step, inset) in [
        (1.0, 16.0, 3.0),
        (1.0, 16.0, 26.0),
        (1.0, 50.0, 3.0),
        (2.0, 16.0, 3.0),
        (1.5, 33.0, 3.0),
    ] {
        let mut l = list(scale, false, false);
        let p = near_bottom(&l, inset);
        l.drag_to(p);
        l.frame(16.0);
        let before = l.offset.y;
        l.run(0.5, step);
        reached.push(l.offset.y - before);
    }
    assert!(
        reached[0] > reached[1] * 1.5,
        "closer to the edge is faster: {reached:?}"
    );
    assert!(
        reached[1] > 20.0,
        "inside the band it still moves: {reached:?}"
    );
    for other in [reached[2], reached[3], reached[4]] {
        assert!(
            (other - reached[0]).abs() < reached[0] * 0.08,
            "{reached:?}"
        );
    }
}

#[test]
fn it_stops_at_the_limit_and_the_instant_the_drag_ends() {
    let mut l = list(1.0, false, false);
    let p = near_bottom(&l, 3.0);
    l.drag_to(p);
    l.run(0.2, 16.0);
    let moving = l.offset.y;
    assert!(moving > 0.0 && l.context.wants_animation_frame());
    l.context.primary_button(ElementState::Released);
    l.frame(16.0);
    l.frame(16.0);
    let stopped = l.offset.y;
    l.run(0.3, 16.0);
    assert_eq!(l.offset.y, stopped);
    assert!(!l.context.wants_animation_frame() && l.context.drag.deadline.is_none());

    let mut l = list(1.0, false, false);
    let p = near_bottom(&l, 3.0);
    l.drag_to(p);
    l.run(6.0, 16.0);
    let max = l.context.scrolling.states[&l.context.scrolling.previous_order[0]].max_offset();
    assert_eq!(l.offset.y, max.y);
    assert!(!l.context.wants_animation_frame(), "nothing left to scroll");
    assert!(l.context.drag.deadline.is_none());
}

#[test]
fn both_axes_scroll_and_pointer_beyond_the_window_counts_as_the_edge() {
    let mut l = list(1.0, false, true);
    let corner = Vec2::new(l.viewport.max.x - 3.0, l.viewport.max.y - 3.0);
    l.drag_to(corner);
    l.run(0.3, 16.0);
    assert!(l.offset.x > 0.0 && l.offset.y > 0.0, "{:?}", l.offset);

    let mut l = list(1.0, false, false);
    l.height = 600.0; // the area now reaches the bottom of the window
    l.frame(16.0);
    l.frame(16.0);
    let below = Vec2::new(l.viewport.min.x + 20.0, 5_000.0);
    l.drag_to(below);
    l.run(0.3, 16.0);
    assert!(l.offset.y > 100.0, "{:?}", l.offset);
}

#[test]
fn the_innermost_container_scrolls_first_then_hands_over_at_its_limit() {
    let mut l = list(1.0, true, false);
    let p = near_bottom(&l, 3.0);
    l.drag_to(p);
    l.run(0.4, 16.0);
    assert!(l.inner.y > 0.0, "inner container moves first");
    assert_eq!(l.offset.y, 0.0, "outer is untouched meanwhile");
    l.run(6.0, 16.0);
    assert!(l.inner.y > 100.0);
    assert!(
        l.offset.y > 0.0,
        "outer takes over once the inner one is at its limit"
    );
}

#[test]
fn the_wheel_keeps_working_during_a_drag() {
    let mut l = list(1.0, false, false);
    let middle = Vec2::new(l.viewport.min.x + 20.0, l.viewport.center().y);
    l.drag_to(middle);
    l.frame(16.0);
    assert_eq!(l.offset.y, 0.0, "no autoscroll away from the edges");
    l.context.on_window_event(&WindowEvent::MouseWheel {
        device_id: DeviceId::dummy(),
        delta: MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition::new(0.0, -40.0)),
        phase: TouchPhase::Moved,
    });
    l.frame(16.0);
    assert!(l.offset.y > 0.0);
    assert!(l.context.dragging().is_some());
}

#[test]
fn indicator_follows_scrolled_bounds_without_jumping() {
    // Rows are targets; while the list scrolls under a stationary pointer the
    // reported hover target is always one that is really under it.
    let mut l = list(1.0, false, false);
    let p = near_bottom(&l, 12.0);
    l.drag_to(p);
    for _ in 0..30 {
        l.frame(16.0);
        if let Some(s) = &l.context.drag.session {
            if let Some(h) = s.hover {
                assert!(h.rect.contains(p) || h.rect.intersect(l.viewport).contains(p));
            }
        }
    }
    assert!(l.offset.y > 0.0);
}
