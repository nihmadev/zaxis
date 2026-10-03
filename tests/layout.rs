use zaxis::winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{DeviceId, ElementState, Ime, MouseButton, WindowEvent},
};
use zaxis::{
    vec2, Align, Button, Color, Context, Id, Padding, Rect, Response, Shape, TextEdit, Ui, Window,
};

fn setup() -> Context {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(800, 600), 1.0);
    let mut style = c.style().clone();
    style.motion.reduced_motion = true;
    c.set_style(style);
    c
}
fn draw<R>(c: &mut Context, width: f32, build: impl FnOnce(&mut Ui<'_>) -> R) -> R {
    let mut result = None;
    c.run(|c| {
        Window::new("layout")
            .id(Id::new("layout"))
            .default_position(vec2(20.0, 20.0))
            .default_size(vec2(700.0, 500.0))
            .resizable(false)
            .draggable(false)
            .padding(Padding::all(0.0))
            .show(c, |ui| {
                result = Some(ui.with_width(width, build));
            });
    });
    result.unwrap()
}
fn visible(c: &Context, r: Response) -> Rect {
    c.visual_rect(r.id, r.rect)
}
fn click(c: &mut Context, p: zaxis::Vec2) {
    c.on_window_event(&WindowEvent::CursorMoved {
        device_id: DeviceId::dummy(),
        position: PhysicalPosition::new(p.x as f64, p.y as f64),
    });
    for state in [ElementState::Pressed, ElementState::Released] {
        c.on_window_event(&WindowEvent::MouseInput {
            device_id: DeviceId::dummy(),
            state,
            button: MouseButton::Left,
        });
    }
}
fn near(a: f32, b: f32) {
    assert!((a - b).abs() < 0.02, "{a} != {b}");
}

#[test]
fn spacer_places_trailing_button_on_first_pass_and_routes_click_and_hover() {
    let mut c = setup();
    let build = |ui: &mut Ui<'_>| {
        ui.horizontal_aligned(Align::Center, |ui| {
            let label = ui.label("Name");
            ui.spacer();
            (label, ui.button("Edit"))
        })
    };
    let (label, button) = draw(&mut c, 320.0, build);
    near(visible(&c, button).max.x, label.rect.min.x + 320.0);
    near(
        visible(&c, label).center().y,
        visible(&c, button).center().y,
    );
    assert!(!c.needs_repaint());
    let point = visible(&c, button).center();
    click(&mut c, point);
    let (_, r) = draw(&mut c, 320.0, build);
    assert!(r.clicked() && r.hovered);
    draw(&mut c, 320.0, build);
    let stats = c.cache_stats();
    draw(&mut c, 320.0, build);
    assert_eq!(
        c.cache_stats().tessellated_elements,
        stats.tessellated_elements
    );
    assert!(!c.needs_repaint());
}

#[test]
fn fill_reserves_following_button_shares_space_resizes_and_settles() {
    let mut c = setup();
    let mut query = String::new();
    let mut calls = 0;
    let mut build = |ui: &mut Ui<'_>| {
        ui.horizontal(|ui| {
            let label = ui.label("Search:");
            let edit = ui.fill(|ui| {
                calls += 1;
                ui.text_edit(&mut query)
            });
            let button = ui.button("Clear");
            (label, edit, button)
        })
    };
    draw(&mut c, 360.0, &mut build);
    assert!(c.needs_repaint());
    let (label, edit, button) = draw(&mut c, 360.0, &mut build);
    let gap = c.style().spacing;
    near(edit.rect.min.x, label.rect.max.x + gap);
    near(visible(&c, button).min.x, edit.rect.max.x + gap);
    near(visible(&c, button).max.x, label.rect.min.x + 360.0);
    assert!(!c.needs_repaint());
    let (_, wider, _) = draw(&mut c, 480.0, &mut build);
    near(wider.rect.size().x, edit.rect.size().x + 120.0);
    assert!(!c.needs_repaint());
    drop(build);
    assert_eq!(calls, 3);
    let (a, b, start) = draw(&mut c, 400.0, |ui| {
        ui.horizontal(|ui| {
            let start = ui.label("Left");
            let a = ui.fill(|ui| ui.button("A"));
            let b = ui.fill(|ui| ui.button("B"));
            (a, b, start)
        })
    });
    let ar = visible(&c, a);
    let br = visible(&c, b);
    near(
        br.min.x - ar.min.x,
        (400.0 - start.rect.size().x - gap * 2.0) / 2.0 + gap,
    );
}

#[test]
fn custom_geometry_aligns_on_first_pass_and_vertical_spacer_uses_height() {
    let mut c = setup();
    let colors = [Color::rgb(255, 0, 0), Color::rgb(0, 255, 0)];
    let (tall, next) = draw(&mut c, 200.0, |ui| {
        let tall = ui.horizontal_aligned(Align::End, |ui| {
            let mut last = Rect::default();
            for (size, color) in [vec2(40.0, 20.0), vec2(40.0, 50.0)].into_iter().zip(colors) {
                let r = ui.allocate_space(size);
                ui.paint(Shape::rect(r, color));
                last = r;
            }
            last
        });
        (tall, ui.label("Below"))
    });
    let mut ranges = [None, None];
    for (i, color) in colors.into_iter().enumerate() {
        let linear = color.0.map(|v| v as f32 / 255.0);
        let ys: Vec<_> = c
            .draw_data()
            .vertices
            .iter()
            .filter(|v| v.color == linear)
            .map(|v| v.position[1])
            .collect();
        assert!(!ys.is_empty());
        ranges[i] = Some((
            ys.iter().copied().fold(f32::INFINITY, f32::min),
            ys.iter().copied().fold(f32::NEG_INFINITY, f32::max),
        ));
    }
    near(ranges[0].unwrap().1, ranges[1].unwrap().1);
    near(next.rect.min.y, tall.max.y + c.style().spacing);
    let (first, last) = draw(&mut c, 200.0, |ui| {
        ui.with_height(180.0, |ui| {
            let first = ui.button("Top");
            ui.spacer();
            (first, ui.button("Bottom"))
        })
    });
    near(visible(&c, last).max.y, first.rect.min.y + 180.0);
}

#[test]
fn size_constraints_reserve_space_clip_overflow_and_scope_defaults() {
    let mut c = setup();
    let mut query = String::new();
    let (fixed, minimum, maximum, normal) = draw(&mut c, 500.0, |ui| {
        ui.horizontal(|ui| {
            let fixed = ui.with_width(90.0, |ui| ui.text_edit(&mut query));
            let minimum = ui.with_min_width(120.0, |ui| ui.button("Min"));
            let maximum = ui.with_max_width(60.0, |ui| {
                ui.add(Button::new("Overflow").min_size(vec2(140.0, 24.0)))
            });
            let normal = ui.button("Normal");
            (fixed, minimum, maximum, normal)
        })
    });
    let gap = c.style().spacing;
    near(fixed.rect.size().x, 90.0);
    near(minimum.rect.min.x, fixed.rect.max.x + gap);
    near(maximum.rect.min.x, minimum.rect.min.x + 120.0 + gap);
    near(normal.rect.min.x, maximum.rect.min.x + 60.0 + gap);
    click(&mut c, normal.rect.center());
    assert!(draw(&mut c, 500.0, |ui| ui.horizontal(|ui| {
        ui.with_width(90.0, |ui| ui.text_edit(&mut query));
        ui.with_min_width(120.0, |ui| ui.button("Min"));
        ui.with_max_width(60.0, |ui| {
            ui.add(Button::new("Overflow").min_size(vec2(140.0, 24.0)))
        });
        ui.button("Normal")
    }))
    .clicked());
}

#[test]
fn column_alignment_height_limits_multiple_spacers_and_zero_space() {
    let mut c = setup();
    let (center, end, tall, below) = draw(&mut c, 300.0, |ui| {
        let center = ui.vertical_aligned(Align::Center, |ui| ui.button("Center"));
        let end = ui.vertical_aligned(Align::End, |ui| ui.button("End"));
        let tall = ui.with_min_height(80.0, |ui| ui.button("Tall"));
        let below = ui.with_max_height(15.0, |ui| ui.button("Short"));
        (center, end, tall, below)
    });
    let origin = tall.rect.min.x;
    near(visible(&c, center).center().x, origin + 150.0);
    near(visible(&c, end).max.x, origin + 300.0);
    near(below.rect.min.y, tall.rect.min.y + 80.0 + c.style().spacing);
    let (a, b) = draw(&mut c, 300.0, |ui| {
        ui.horizontal(|ui| {
            ui.spacer();
            let a = ui.button("A");
            ui.spacer();
            let b = ui.button("B");
            (a, b)
        })
    });
    near(visible(&c, b).max.x, origin + 300.0);
    near(
        visible(&c, a).min.x - origin - c.style().spacing,
        visible(&c, b).min.x - visible(&c, a).max.x - c.style().spacing * 2.0,
    );
    let mut query = String::new();
    for _ in 0..2 {
        draw(&mut c, 0.0, |ui| {
            ui.horizontal(|ui| {
                ui.spacer();
                ui.fill(|ui| ui.add(TextEdit::new(&mut query)));
                ui.button("Zero");
            })
        });
    }
    assert!(!c.needs_repaint());
    assert!(c
        .draw_data()
        .vertices
        .iter()
        .all(|v| v.position.iter().all(|v| v.is_finite())));
}

#[test]
fn aligned_editor_uses_displayed_pointer_coordinates_and_ime_bounds() {
    let mut c = setup();
    let mut text = "abcdef".to_owned();
    let draw_editor = |c: &mut Context, text: &mut String| {
        draw(c, 400.0, |ui| {
            ui.horizontal_aligned(Align::End, |ui| {
                ui.add(Button::new("Tall").min_size(vec2(50.0, 50.0)));
                ui.spacer();
                ui.add(TextEdit::new(text).id_source("edit").width(100.0))
            })
        })
    };
    let edit = draw_editor(&mut c, &mut text);
    let rect = visible(&c, edit);
    let p = vec2(
        rect.min.x + c.style().text_edit_padding.left + 0.1,
        rect.center().y,
    );
    click(&mut c, p);
    draw_editor(&mut c, &mut text);
    let ime = c.ime_cursor_area().unwrap();
    assert!(rect.contains(ime.center()));
    c.on_window_event(&WindowEvent::Ime(Ime::Commit("X".to_owned())));
    assert!(draw_editor(&mut c, &mut text).changed());
    assert_eq!(text, "Xabcdef");
}
