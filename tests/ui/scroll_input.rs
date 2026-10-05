use crate::prelude::*;
use std::time::Duration;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{DeviceId, ElementState, MouseButton, WindowEvent},
    keyboard::KeyCode,
};
use zaxis::Instant;
use zaxis::{vec2, Rect, ScrollArea, Window};

fn setup(scale: f64) -> Context {
    let mut c = Context::new();
    c.set_viewport(
        PhysicalSize::new((800.0 * scale) as u32, (600.0 * scale) as u32),
        scale,
    );
    let mut style = c.style().clone();
    style.motion.reduced_motion = true;
    c.set_style(style);
    c
}
fn draw(c: &mut Context, at: Instant, middle: bool) -> (Id, Rect) {
    let mut result = None;
    c.run_at(at, |c| {
        Window::new("Middle")
            .default_size(vec2(600.0, 500.0))
            .show(c, |ui| {
                let out = ScrollArea::vertical()
                    .id_source("items")
                    .middle_mouse_scroll(middle)
                    .max_height(160.0)
                    .show(ui, |ui| {
                        ui.allocate_space(vec2(300.0, 800.0));
                    });
                result = Some((out.id, out.viewport));
            })
    });
    result.unwrap()
}
fn pointer(c: &mut Context, p: Vec2) {
    c.on_window_event(&WindowEvent::CursorMoved {
        device_id: DeviceId::dummy(),
        position: PhysicalPosition::new(
            (p.x * c.probe().scale) as f64,
            (p.y * c.probe().scale) as f64,
        ),
    });
}
fn middle(c: &mut Context, state: ElementState) -> bool {
    c.on_window_event(&WindowEvent::MouseInput {
        device_id: DeviceId::dummy(),
        state,
        button: MouseButton::Middle,
    })
    .consumed
}
#[test]
fn latched_middle_scroll_is_fractional_dpi_independent_and_sleeps_in_dead_zone() {
    for scale in [1.0, 1.25, 2.0] {
        let mut c = setup(scale);
        let start = c.probe().frame_time + Duration::from_secs(1);
        let (id, viewport) = draw(&mut c, start, true);
        pointer(&mut c, viewport.center());
        assert!(middle(&mut c, ElementState::Pressed));
        assert!(middle(&mut c, ElementState::Released));
        draw(&mut c, start, true);
        assert!(c.is_auto_scrolling());
        assert!(c.next_repaint().is_none());
        assert!(!c.wants_animation_frame());
        assert!(!c.needs_repaint_at(start));
        pointer(&mut c, viewport.center() + vec2(0.0, 40.0));
        draw(&mut c, start + Duration::from_millis(16), true);
        assert_eq!(c.next_repaint(), Some(start + Duration::from_millis(32)));
        assert!(c.wants_animation_frame());
        draw(&mut c, start + Duration::from_millis(32), true);
        assert!((c.probe().scrolling.states[&id].offset.y - 4.096).abs() < 0.001);
        // Active movement uses a deadline, never an immediate busy redraw loop.
        assert!(!c.needs_repaint_at(start + Duration::from_millis(32)));
        pointer(&mut c, viewport.center() - vec2(0.0, 40.0));
        draw(&mut c, start + Duration::from_millis(48), true);
        assert_eq!(c.probe().scrolling.states[&id].offset, Vec2::ZERO);
        pointer(&mut c, viewport.center() + vec2(4.0, 4.0));
        draw(&mut c, start + Duration::from_millis(64), true);
        assert!(c.next_repaint().is_none());
        assert!(!c.wants_animation_frame());
        assert!(
            c.on_key_event(KeyCode::Escape, ElementState::Pressed, false)
                .consumed
        );
        assert!(!c.is_auto_scrolling());
    }
}
#[test]
fn held_middle_gesture_bubbles_remainder_and_releases_outside() {
    let mut c = setup(1.0);
    let start = c.probe().frame_time + Duration::from_secs(1);
    let mut ids = None;
    let draw_nested = |c: &mut Context, at, init: bool, ids: &mut Option<_>| {
        c.run_at(at, |c| {
            Window::new("Nested middle").show(c, |ui| {
                let outer = ScrollArea::vertical()
                    .id_source("outer")
                    .max_height(240.0)
                    .show(ui, |ui| {
                        let mut area = ScrollArea::vertical().id_source("inner").max_height(100.0);
                        if init {
                            area = area.scroll_offset(vec2(0.0, 303.0));
                        }
                        let inner = area.show(ui, |ui| {
                            ui.allocate_space(vec2(250.0, 400.0));
                        });
                        *ids = Some((inner.id, inner.viewport));
                        ui.allocate_space(vec2(250.0, 800.0));
                    });
                assert!(ui
                    .context()
                    .probe()
                    .scrolling
                    .states
                    .contains_key(&outer.id));
            })
        });
    };
    draw_nested(&mut c, start, true, &mut ids);
    let (id, viewport) = ids.unwrap();
    pointer(&mut c, viewport.center());
    assert!(middle(&mut c, ElementState::Pressed));
    pointer(&mut c, viewport.center() + vec2(300.0, 100.0));
    draw_nested(&mut c, start, false, &mut ids);
    draw_nested(&mut c, start + Duration::from_millis(16), false, &mut ids);
    let state = &c.probe().scrolling.states[&id];
    assert_eq!(state.offset.y, state.max_offset().y);
    let parent = state.parent.unwrap();
    assert!((c.probe().scrolling.states[&parent].offset.y - (736.0 * 0.016 - 1.0)).abs() < 0.001);
    assert!(middle(&mut c, ElementState::Released));
    assert!(!c.is_auto_scrolling());
    assert!(c.next_repaint().is_none());
}
#[test]
fn middle_boundaries_cancel_and_removal_preserve_unrelated_deadlines() {
    let mut c = setup(1.0);
    let start = c.probe().frame_time + Duration::from_secs(1);
    let (id, viewport) = draw(&mut c, start, true);
    let max = c.probe().scrolling.states[&id].max_offset().y;
    c.probe_mut()
        .scrolling
        .states
        .get_mut(&id)
        .unwrap()
        .offset
        .y = max;
    pointer(&mut c, viewport.center());
    assert!(middle(&mut c, ElementState::Pressed));
    middle(&mut c, ElementState::Released);
    pointer(&mut c, viewport.center() + vec2(0.0, 40.0));
    draw(&mut c, start, true);
    assert!(c.is_auto_scrolling());
    assert!(c.next_repaint().is_none());
    *c.probe_mut().next_repaint = Some(start + Duration::from_secs(2));
    pointer(&mut c, viewport.center() - vec2(0.0, 40.0));
    draw(&mut c, start + Duration::from_millis(16), true);
    assert_eq!(c.next_repaint(), Some(start + Duration::from_millis(32)));
    assert!(middle(&mut c, ElementState::Pressed));
    assert!(!c.is_auto_scrolling());
    assert_eq!(c.next_repaint(), Some(start + Duration::from_secs(2)));
    middle(&mut c, ElementState::Released);
    middle(&mut c, ElementState::Pressed);
    middle(&mut c, ElementState::Released);
    c.run_at(start + Duration::from_millis(32), |_| {});
    assert!(!c.is_auto_scrolling());
    assert_eq!(c.next_repaint(), Some(start + Duration::from_secs(2)));
}
#[test]
fn middle_opt_out_disabled_focus_loss_and_other_clicks_stop_the_mode() {
    let mut c = setup(1.0);
    let start = c.probe().frame_time + Duration::from_secs(1);
    let (_, viewport) = draw(&mut c, start, false);
    pointer(&mut c, viewport.center());
    assert!(!middle(&mut c, ElementState::Pressed));
    middle(&mut c, ElementState::Released);
    draw(&mut c, start, true);
    for stop in [
        WindowEvent::Focused(false),
        WindowEvent::CursorLeft {
            device_id: DeviceId::dummy(),
        },
        WindowEvent::MouseInput {
            device_id: DeviceId::dummy(),
            state: ElementState::Pressed,
            button: MouseButton::Left,
        },
    ] {
        c.on_window_event(&WindowEvent::Focused(true));
        pointer(&mut c, viewport.center());
        assert!(middle(&mut c, ElementState::Pressed));
        middle(&mut c, ElementState::Released);
        c.on_window_event(&stop);
        assert!(!c.is_auto_scrolling());
        assert!(c.probe().capture.is_none());
    }
    pointer(&mut c, viewport.center());
    middle(&mut c, ElementState::Pressed);
    middle(&mut c, ElementState::Released);
    draw(&mut c, start, false);
    assert!(!c.is_auto_scrolling());
}
#[test]
fn shader_hints_have_separate_commands_and_keep_idle_geometry_cached() {
    let mut c = setup(1.0);
    let start = c.probe().frame_time + Duration::from_secs(1);
    draw(&mut c, start, true);
    let hints: Vec<_> = c
        .draw_data()
        .commands
        .iter()
        .filter(|command| command.scroll_hint)
        .collect();
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0].indices.len(), 6);
    for &index in
        &c.draw_data().indices[hints[0].indices.start as usize..hints[0].indices.end as usize]
    {
        let vertex = c.draw_data().vertices[index as usize];
        assert_eq!(&vertex.color[..3], &[0.0; 3]);
        assert!((vertex.color[3] - 16.0 / 255.0).abs() < 0.001);
    }
    let stats = c.cache_stats();
    let revision = c.draw_data().revision;
    draw(&mut c, start + Duration::from_secs(1), true);
    assert_eq!(
        c.cache_stats().tessellated_elements,
        stats.tessellated_elements
    );
    assert_eq!(c.draw_data().revision, revision);
    assert!(!c.needs_repaint_at(start + Duration::from_secs(1)));
}
