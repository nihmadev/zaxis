use crate::prelude::*;
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    time::Duration,
};
use winit::{
    dpi::PhysicalSize,
    event::ElementState,
    keyboard::{KeyCode, ModifiersState},
};
use zaxis::Instant;

fn setup() -> Context {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(1000, 900), 1.0);
    let mut t = Theme::dark();
    t.overrides.motion = Some(MotionStyle {
        reduced_motion: true,
        ..Default::default()
    });
    t.overrides.text_edit_blink_interval = Some(Duration::ZERO);
    c.set_theme(t);
    c
}
fn solid(c: Color) -> Gradient {
    Gradient::new(c, c)
}

#[test]
fn explicit_zero_overrides_survive_palette_and_density_changes() {
    let mut t = Theme::light();
    t.overrides.number.height = Some(41.0);
    t.overrides.button.surface.idle = SurfaceStyle {
        fill: Some(solid(Color::TRANSPARENT)),
        border: Some(Border::NONE),
        rounding: Some(CornerRadius::ZERO),
        blur: Some(0.0),
        opacity: Some(0.0),
        shadow: Some(Shadow {
            color: Color::TRANSPARENT,
            offset: Vec2::ZERO,
            blur_radius: 0.0,
            spread: 0.0,
        }),
        ..Default::default()
    };
    let a = t.resolve();
    let b = t
        .accent(Color::rgb(255, 0, 130))
        .density(Density::Compact)
        .resolve();
    assert_eq!(a.button, b.button);
    assert_eq!(a.number.height, b.number.height);
    assert_ne!(a.text_edit_height, b.text_edit_height);
    assert_eq!(b.button.surface.idle.fill, Some(solid(Color::TRANSPARENT)));
    assert_eq!(b.button.surface.idle.border, Some(Border::NONE));
    assert_eq!(b.button.surface.idle.rounding, Some(CornerRadius::ZERO));
}

#[test]
fn state_priority_combines_selected_focus_and_disables_hover() {
    use zaxis::components::appearance::{resolve_control, Appearance};
    let s = Theme::light().resolve();
    let base = Appearance::new(s.button_fill, s.border, s.text_color);
    let control = ControlStyle {
        selected: SurfaceStyle::fill(Color::rgb(0, 0, 200)),
        hover: SurfaceStyle::fill(Color::rgb(0, 100, 200)),
        pressed: SurfaceStyle::fill(Color::rgb(0, 200, 0)),
        disabled: SurfaceStyle {
            foreground: Some(Color::gray(110)),
            ..SurfaceStyle::fill(Color::gray(220))
        },
        ..Default::default()
    };
    let mut state = ControlState {
        enabled: true,
        hovered: true,
        selected: true,
        focus: true,
        ..Default::default()
    };
    let a = resolve_control(
        &s,
        base,
        HoverStyle::NONE,
        false,
        control,
        state,
        s.button_hovered,
    );
    assert_eq!(a.fill.start, Color::rgb(0, 100, 200));
    assert_eq!(a.border, s.focus_border);
    state.pressed = true;
    let a = resolve_control(
        &s,
        base,
        HoverStyle::NONE,
        false,
        control,
        state,
        s.button_hovered,
    );
    assert_eq!(a.fill.start, Color::rgb(0, 200, 0));
    assert_eq!(a.border, s.focus_border);
    state.enabled = false;
    state.focus = false;
    let a = resolve_control(
        &s,
        base,
        HoverStyle::fill(Color::BLACK),
        true,
        control,
        state,
        s.button_hovered,
    );
    assert_eq!(a.fill.start, Color::gray(220));
    assert_eq!(a.text_color, Color::gray(110));
}

#[test]
fn builtin_contrast_on_real_adjacent_surfaces() {
    for t in [Theme::dark(), Theme::light(), Theme::high_contrast()] {
        let p = t.palette;
        for surface in [
            p.background,
            p.surface,
            p.surface_raised,
            p.surface_control,
            p.hover,
            p.pressed,
        ] {
            assert!(
                contrast_ratio(p.foreground, surface) >= 4.5,
                "foreground {surface:?}"
            );
            assert!(contrast_ratio(p.muted, surface) >= 4.5, "muted {surface:?}");
        }
        for (foreground, background) in [
            (p.on_accent, p.accent),
            (p.on_selected, p.selected),
            (p.on_success, p.success),
            (p.on_warning, p.warning),
            (p.on_error, p.error),
        ] {
            assert!(
                contrast_ratio(foreground, background) >= 4.5,
                "{foreground:?} on {background:?}"
            );
        }
        for surface in [p.surface, p.surface_control, p.hover, p.selected] {
            assert!(contrast_ratio(p.focus, surface) >= 3.0, "focus {surface:?}");
        }
    }
}

#[test]
fn scopes_restore_on_return_and_unwind_and_follow_all_containers() {
    let mut c = setup();
    let original = c.style().clone();
    let local = Theme::light();
    let patch = StyleOverrides {
        text_color: Some(Color::rgb(140, 0, 100)),
        ..Default::default()
    };
    c.run(|c| {
        Root::new().show(c, |ui| {
            ui.with_theme(&local, |ui| {
                assert_eq!(ui.style().window_fill, local.palette.surface);
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        assert_eq!(ui.style().text_color, local.palette.foreground);
                    })
                });
                ui.with_style(&patch, |ui| {
                    ui.vertical(|ui| assert_eq!(ui.style().text_color, patch.text_color.unwrap()));
                    Grid::new("g").column(Column::remainder("a")).show(ui, |g| {
                        g.row(0, |row| {
                            row.cell(|ui| {
                                assert_eq!(ui.style().text_color, patch.text_color.unwrap())
                            })
                        })
                    });
                    ScrollArea::vertical().max_height(50.0).show(ui, |ui| {
                        assert_eq!(ui.style().text_color, patch.text_color.unwrap());
                        ui.presence("p", true, |ui| {
                            assert_eq!(ui.style().text_color, patch.text_color.unwrap());
                            ui.label("inside");
                        });
                    });
                    let anchor = Rect::from_min_size(vec2(20.0, 20.0), vec2(100.0, 25.0));
                    let mut open = true;
                    Popup::new("pop", anchor).show(ui, &mut open, |ui| {
                        assert_eq!(ui.style().text_color, patch.text_color.unwrap())
                    });
                });
                assert_eq!(ui.style().text_color, local.palette.foreground);
                let r = catch_unwind(AssertUnwindSafe(|| {
                    ui.with_style(&patch, |_| -> () {
                        panic!("expected");
                    })
                }));
                assert!(r.is_err());
                assert_eq!(ui.style().text_color, local.palette.foreground);
            });
            assert_eq!(ui.style(), &original);
            assert_eq!(ui.context().style(), &original);
        })
    });
    assert_eq!(c.style(), &original);
}

#[test]
fn setters_and_custom_paint_settle_without_cache_churn() {
    let mut c = setup();
    let theme = c.theme().unwrap().clone();
    let mut checked = true;
    let mut value = 0.4;
    let draw = |c: &mut Context, checked: &mut bool, value: &mut f32| {
        c.run(|c| {
            Root::new().show(c, |ui| {
                ui.with_theme(&Theme::light(), |ui| {
                    ui.add(Button::new("Paint").painter(PaintMode::Replace, |p, info| {
                        p.paint(Shape::rect(info.bounds, Color::gray(180)))
                    }));
                    ui.add(
                        Checkbox::new(checked, "Check").painter(PaintMode::After, |p, info| {
                            if info.part == PaintPart::CheckboxIndicator {
                                p.paint(Shape::rect(
                                    info.bounds.shrink(7.0),
                                    Color::rgb(0, 50, 100),
                                ));
                            }
                        }),
                    );
                    ui.add(
                        Slider::new(value, 0.0..=1.0).painter(PaintMode::Replace, |p, info| {
                            p.paint(Shape::rect(info.bounds, Color::rgb(100, 120, 150)))
                        }),
                    );
                });
            })
        })
    };
    draw(&mut c, &mut checked, &mut value);
    draw(&mut c, &mut checked, &mut value);
    let before = c.cache_stats();
    let revision = c.draw_data().revision;
    let style_revision = c.probe().style_revision;
    let local_serial = c.probe().local_style_serial;
    for _ in 0..5 {
        c.set_theme(theme.clone());
        draw(&mut c, &mut checked, &mut value);
    }
    assert_eq!(c.probe().style_revision, style_revision);
    assert_eq!(c.probe().local_style_serial, local_serial);
    assert_eq!(c.draw_data().revision, revision);
    assert_eq!(
        c.cache_stats().tessellated_elements,
        before.tessellated_elements
    );
    assert!(!c.needs_repaint());
    let style = c.style().clone();
    c.set_style(style.clone());
    assert!(!c.needs_repaint());
    assert!(c.theme().is_none());
    assert_eq!(c.style(), &style);
}

#[test]
fn palette_transition_keeps_target_metrics_and_finishes_repaint() {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(900, 600), 1.0);
    c.set_theme(Theme::dark());
    let start = Instant::now();
    c.run_at(start, |_| {});
    let to = Theme::light().density(Density::Compact);
    c.set_theme_animated(to.clone(), TweenOptions::new(Duration::from_millis(100)));
    assert_eq!(c.style().spacing, to.resolve().spacing);
    assert_ne!(c.style().text_color, to.palette.foreground);
    c.run_at(start + Duration::from_millis(50), |c| {
        Root::new().show(c, |ui| {
            ui.button("palette");
        });
    });
    c.run_at(start + Duration::from_millis(200), |c| {
        Root::new().show(c, |ui| {
            ui.button("palette");
        });
    });
    c.run_at(start + Duration::from_millis(300), |c| {
        Root::new().show(c, |ui| {
            ui.button("palette");
        });
    });
    assert_eq!(c.style(), &to.resolve());
    assert!(!c.needs_repaint());
}

#[test]
fn theme_change_preserves_editor_focus_selection_undo_popup_scroll_and_values() {
    let mut c = setup();
    let mut text = "original".to_owned();
    let mut chosen = Some(1usize);
    let mut offset = Vec2::ZERO;
    let options = (0..12)
        .map(|i| ComboBoxOption::new(i, i, format!("item {i}")))
        .collect::<Vec<_>>();
    let mut editor = Id::new(0);
    let mut combo = Id::new(0);
    let mut scroll = Id::new(0);
    let mut draw = |c: &mut Context, text: &mut String, chosen: &mut Option<usize>| {
        c.run(|c| {
            Root::new().show(c, |ui| {
                editor = ui.add(TextEdit::new(text).id_source("edit")).id;
                scroll = ScrollArea::vertical()
                    .id_source("scroll")
                    .max_height(90.0)
                    .scroll_offset(vec2(0.0, 40.0))
                    .show(ui, |ui| {
                        for i in 0..20 {
                            ui.label(format!("row {i}"));
                        }
                    })
                    .id;
                offset = ui.context().probe().scrolling.states[&scroll].offset;
                combo = ui
                    .add(
                        ComboBox::new(chosen, &options)
                            .id_source("combo")
                            .default_open(true),
                    )
                    .id;
            })
        })
    };
    draw(&mut c, &mut text, &mut chosen);
    drop(draw);
    c.dismiss_popup(true);
    c.request_focus(editor);
    c.key(KeyCode::KeyA, ElementState::Pressed, false); // Control modifier set below.
    c.set_modifiers(ModifiersState::CONTROL);
    c.key(KeyCode::KeyA, ElementState::Pressed, false);
    let draw = |c: &mut Context, text: &mut String, chosen: &mut Option<usize>| {
        c.run(|c| {
            Root::new().show(c, |ui| {
                ui.add(TextEdit::new(text).id_source("edit"));
                ScrollArea::vertical()
                    .id_source("scroll")
                    .max_height(90.0)
                    .show(ui, |ui| {
                        for i in 0..20 {
                            ui.label(format!("row {i}"));
                        }
                    });
                ui.add(ComboBox::new(chosen, &options).id_source("combo"));
            })
        })
    };
    draw(&mut c, &mut text, &mut chosen);
    let selection = c.probe().text_edits[&editor].buffer.selection();
    let focus = c.probe().focused_widget;
    let mut theme = Theme::light();
    theme.overrides.motion = Some(MotionStyle {
        reduced_motion: true,
        ..Default::default()
    });
    theme.overrides.text_edit_blink_interval = Some(Duration::ZERO);
    c.set_theme(theme);
    draw(&mut c, &mut text, &mut chosen);
    assert_eq!(c.probe().focused_widget, focus);
    assert_eq!(c.probe().text_edits[&editor].buffer.selection(), selection);
    assert_eq!(c.probe().scrolling.states[&scroll].offset, offset);
    assert_eq!(chosen, Some(1));
    assert_eq!(text, "original");
    c.set_modifiers(ModifiersState::empty());
    c.on_text_event("changed");
    draw(&mut c, &mut text, &mut chosen);
    assert_eq!(text, "changed");
    c.set_theme(Theme::dark());
    c.set_modifiers(ModifiersState::CONTROL);
    c.key(KeyCode::KeyZ, ElementState::Pressed, false);
    draw(&mut c, &mut text, &mut chosen);
    assert_eq!(text, "original", "undo survives theme changes");
    c.set_modifiers(ModifiersState::empty());
    c.request_focus(combo);
    c.key(KeyCode::ArrowDown, ElementState::Pressed, false);
    draw(&mut c, &mut text, &mut chosen);
    let popup = c.probe().popup.as_ref().unwrap().id;
    c.set_theme(Theme::dark());
    draw(&mut c, &mut text, &mut chosen);
    assert_eq!(c.probe().popup.as_ref().unwrap().id, popup);
    assert_eq!(chosen, Some(1));
}
