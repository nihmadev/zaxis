//! `Ui::interact`: a custom widget goes through the same clip, scroll, disabled,
//! capture, layer, popup, keyboard and repaint rules as built-in controls.
use crate::prelude::*;
use winit::{dpi::PhysicalSize, event::ElementState, keyboard::KeyCode};
use zaxis::{Color, Column, Grid, Id, Popup, Rect, Response, ScrollArea, Sense, Shape, Ui, Window};

fn setup() -> Context {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(800, 600), 1.0);
    c
}

/// A minimal external widget: allocate, interact, paint.
fn pad(ui: &mut Ui<'_>, key: &str, sense: Sense) -> Response {
    let rect = ui.allocate_space(Vec2::new(120.0, 40.0));
    let response = ui.interact(rect, key, sense);
    let fill = if response.pressed {
        Color::rgb(200, 80, 80)
    } else if response.hovered {
        Color::rgb(90, 140, 220)
    } else {
        Color::gray(60)
    };
    ui.paint(Shape::rect(rect, fill));
    response
}
fn click(c: &mut Context, p: Vec2) {
    c.move_pointer(p);
    c.primary_button(ElementState::Pressed);
    c.primary_button(ElementState::Released);
}
fn key(c: &mut Context, code: KeyCode) {
    c.on_key_event(code, ElementState::Pressed, false);
    c.on_key_event(code, ElementState::Released, false);
}

fn single(c: &mut Context, sense: Sense, enabled: bool) -> Response {
    let mut out = None;
    c.run(|c| {
        Window::new("Pad").show(c, |ui| {
            ui.add_enabled_ui(enabled, |ui| out = Some(pad(ui, "pad", sense)));
        });
    });
    out.unwrap()
}

#[test]
fn senses_select_which_events_a_custom_widget_reports() {
    let mut c = setup();
    let at = single(&mut c, Sense::CLICK, true).rect.center();
    click(&mut c, at);
    let r = single(&mut c, Sense::CLICK, true);
    assert!(r.clicked() && r.enabled);
    assert!(
        !single(&mut c, Sense::CLICK, true).clicked(),
        "once per input"
    );
    // Hover only: reports hover but a press reaches nothing and clicks nothing.
    single(&mut c, Sense::HOVER, true);
    c.move_pointer(at);
    assert!(single(&mut c, Sense::HOVER, true).hovered);
    click(&mut c, at);
    let r = single(&mut c, Sense::HOVER, true);
    assert!(!r.clicked() && !r.pressed);
    // Drag only: no click, but drag events.
    single(&mut c, Sense::DRAG, true);
    c.move_pointer(at);
    c.primary_button(ElementState::Pressed);
    c.move_pointer(at + Vec2::new(30.0, 0.0));
    let r = single(&mut c, Sense::DRAG, true);
    assert!(r.drag_started() && r.drag_delta() == Vec2::new(30.0, 0.0) && r.pressed);
    c.primary_button(ElementState::Released);
    let r = single(&mut c, Sense::DRAG, true);
    assert!(r.drag_stopped() && !r.clicked());
}

#[test]
fn a_sense_set_to_none_is_passive() {
    let mut c = setup();
    let r = single(&mut c, Sense::NONE, true);
    click(&mut c, r.rect.center());
    let r = single(&mut c, Sense::NONE, true);
    assert!(!r.clicked() && !r.pressed && !r.has_focus);
}

#[test]
fn capture_continues_outside_and_a_release_outside_is_no_click() {
    let mut c = setup();
    let at = single(&mut c, Sense::CLICK | Sense::DRAG, true)
        .rect
        .center();
    c.move_pointer(at);
    c.primary_button(ElementState::Pressed);
    c.move_pointer(Vec2::new(700.0, 500.0));
    let r = single(&mut c, Sense::CLICK | Sense::DRAG, true);
    assert!(
        r.pressed && r.drag_started(),
        "captured while the pointer is far away"
    );
    assert_eq!(r.drag_delta(), Vec2::new(700.0, 500.0) - at);
    c.primary_button(ElementState::Released);
    let r = single(&mut c, Sense::CLICK | Sense::DRAG, true);
    assert!(r.drag_stopped() && !r.clicked() && !r.pressed);
}

#[test]
fn tab_focuses_and_enter_or_space_click_once() {
    let mut c = setup();
    single(&mut c, Sense::CLICK | Sense::FOCUS, true);
    key(&mut c, KeyCode::Tab);
    let r = single(&mut c, Sense::CLICK | Sense::FOCUS, true);
    assert!(r.has_focus && r.gained_focus() && r.focus_visible);
    for code in [KeyCode::Enter, KeyCode::Space] {
        key(&mut c, code);
        assert!(
            single(&mut c, Sense::CLICK | Sense::FOCUS, true).clicked(),
            "{code:?}"
        );
        assert!(!single(&mut c, Sense::CLICK | Sense::FOCUS, true).clicked());
    }
    // Without the focus sense Tab never lands here.
    let mut c = setup();
    single(&mut c, Sense::CLICK, true);
    key(&mut c, KeyCode::Tab);
    assert!(!single(&mut c, Sense::CLICK, true).has_focus);
}

#[test]
fn focused_widgets_read_raw_keys_from_the_input_state() {
    let mut c = setup();
    single(&mut c, Sense::FOCUS, true);
    key(&mut c, KeyCode::Tab);
    single(&mut c, Sense::FOCUS, true);
    c.on_key_event(KeyCode::ArrowRight, ElementState::Pressed, false);
    assert!(c.input().keys_pressed.contains(&KeyCode::ArrowRight));
}

#[test]
fn a_click_and_an_event_request_a_follow_up_pass_and_idling_does_not() {
    let mut c = setup();
    let r = single(&mut c, Sense::CLICK, true);
    single(&mut c, Sense::CLICK, true);
    assert!(!c.needs_repaint());
    click(&mut c, r.rect.center());
    let r = single(&mut c, Sense::CLICK, true);
    assert!(r.clicked() && c.needs_repaint());
    single(&mut c, Sense::CLICK, true);
    assert!(!c.needs_repaint());
}

#[test]
fn disabled_scopes_block_input_and_keep_the_response_disabled() {
    let mut c = setup();
    let r = single(&mut c, Sense::CLICK | Sense::DRAG | Sense::FOCUS, false);
    assert!(!r.enabled);
    click(&mut c, r.rect.center());
    let r = single(&mut c, Sense::CLICK | Sense::DRAG | Sense::FOCUS, false);
    assert!(!r.clicked() && !r.pressed && !r.has_focus);
    key(&mut c, KeyCode::Tab);
    assert!(!single(&mut c, Sense::CLICK | Sense::FOCUS, false).has_focus);
    // The region blocks what is under it, like a disabled button.
    let mut under = None;
    c.run(|c| {
        Window::new("Stack").show(c, |ui| {
            ui.add_enabled_ui(false, |ui| {
                let rect = ui.allocate_space(Vec2::new(120.0, 40.0));
                ui.interact(rect, "blocker", Sense::CLICK);
            });
            under = Some(ui.button("After"));
        });
    });
    assert!(under.is_some());
}

fn scrolled(c: &mut Context, offset: f32) -> Vec<Response> {
    let mut out = Vec::new();
    c.run(|c| {
        Window::new("Scroll").show(c, |ui| {
            ScrollArea::vertical()
                .id_source("area")
                .max_height(100.0)
                .scroll_offset(Vec2::new(0.0, offset))
                .show(ui, |ui| {
                    for i in 0..10 {
                        out.push(pad(ui, &format!("row{i}"), Sense::CLICK));
                    }
                });
        });
    });
    out
}

#[test]
fn rows_scrolled_out_of_the_clip_get_no_input_and_visible_ones_follow_the_offset() {
    let mut c = setup();
    let rows = scrolled(&mut c, 0.0);
    let hit = |c: &Context, id: Id| c.probe().previous_hits.iter().find(|h| h.id == id).copied();
    assert!(hit(&c, rows[0].id).is_some());
    // Row 9 lies outside the 100px viewport: it has no region at all, so no input.
    assert!(hit(&c, rows[9].id).is_none());
    click(&mut c, rows[9].rect.center());
    assert!(!scrolled(&mut c, 0.0)[9].clicked());
    // Scroll the second row to the top and click it where it now is.
    let step = rows[0].rect.size().y + 8.0;
    scrolled(&mut c, step);
    let rows = scrolled(&mut c, step);
    let first = hit(&c, rows[0].id);
    let second = hit(&c, rows[1].id).expect("the second row is now visible");
    assert!(first.is_none_or(|h| h.rect.max.y <= second.rect.min.y + 0.5));
    click(&mut c, second.rect.center());
    let rows = scrolled(&mut c, step);
    assert!(rows[1].clicked() && !rows[0].clicked() && !rows[2].clicked());
}

#[test]
fn a_window_above_hides_the_widget_below_from_pointer_and_clicks() {
    let mut c = setup();
    let draw = |c: &mut Context| {
        let mut out = None;
        c.run(|c| {
            Window::new("Lower")
                .default_position(Vec2::new(20.0, 20.0))
                .show(c, |ui| {
                    out = Some(pad(ui, "lower", Sense::CLICK | Sense::HOVER))
                });
            Window::new("Upper")
                .default_position(Vec2::new(10.0, 10.0))
                .default_size(Vec2::new(400.0, 300.0))
                .show(c, |ui| {
                    ui.label("on top");
                });
        });
        out.unwrap()
    };
    let r = draw(&mut c);
    c.move_pointer(r.rect.center());
    let r = draw(&mut c);
    assert!(!r.hovered, "the upper window owns the pointer");
    click(&mut c, r.rect.center());
    assert!(!draw(&mut c).clicked());
}

#[test]
fn a_widget_in_a_grid_cell_is_hit_where_the_cell_places_it() {
    let mut c = setup();
    let draw = |c: &mut Context| {
        let mut out = None;
        c.run(|c| {
            Window::new("Grid").show(c, |ui| {
                Grid::new("g")
                    .columns([Column::fixed("a", 140.0), Column::fixed("b", 140.0)])
                    .show(ui, |grid| {
                        grid.row("r", |row| {
                            row.cell(|ui| {
                                ui.label("left");
                            });
                            row.cell(|ui| out = Some(pad(ui, "cell", Sense::CLICK)));
                        });
                    });
            });
        });
        out.unwrap()
    };
    draw(&mut c);
    let r = draw(&mut c);
    let hit = c
        .probe()
        .previous_hits
        .iter()
        .find(|h| h.id == r.id)
        .copied()
        .expect("the cell registers its region");
    assert!(
        hit.rect.min.x > 100.0,
        "placed in the second column: {:?}",
        hit.rect
    );
    click(&mut c, hit.rect.center());
    assert!(draw(&mut c).clicked());
}

#[test]
fn a_widget_in_a_popup_is_clickable_above_everything_else() {
    let mut c = setup();
    let draw = |c: &mut Context, open: &mut bool| {
        let mut out = None;
        c.run(|c| {
            Window::new("Host").show(c, |ui| {
                let anchor = ui.button("Anchor").rect;
                Popup::new("pop", anchor).show(ui, open, |ui| {
                    out = Some(pad(ui, "in-popup", Sense::CLICK));
                });
            });
        });
        out
    };
    let mut open = true;
    draw(&mut c, &mut open);
    let r = draw(&mut c, &mut open).expect("open popup builds its content");
    assert!(c.probe().popup.is_some());
    click(&mut c, r.rect.center());
    assert!(draw(&mut c, &mut open).unwrap().clicked());
}

#[test]
fn colliding_ids_and_empty_rects_are_diagnosed_without_breaking_input() {
    let mut c = setup();
    c.run(|c| {
        Window::new("Dup").show(c, |ui| {
            pad(ui, "same", Sense::CLICK);
            pad(ui, "same", Sense::CLICK);
            let empty = Rect::from_min_size(Vec2::new(300.0, 300.0), Vec2::new(0.0, 20.0));
            ui.interact(empty, "empty", Sense::CLICK);
        });
    });
    let kinds: Vec<_> = c.diagnostics().iter().map(|d| d.kind).collect();
    assert!(
        kinds.contains(&zaxis::DiagnosticKind::IdCollision),
        "{kinds:?}"
    );
    assert!(
        kinds.contains(&zaxis::DiagnosticKind::NoLayoutSpace),
        "{kinds:?}"
    );
    let _ = Id::new("unused");
}
