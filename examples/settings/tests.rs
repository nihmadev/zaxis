use super::*;
use zaxis::winit::event::WindowEvent;
use zaxis::winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{DeviceId, ElementState, MouseButton},
};
use zaxis::Instant;

#[test]
#[ignore = "interaction profiling; run with --ignored --nocapture"]
fn profile_interaction_cache() {
    let mut context = Context::new();
    context.set_viewport(PhysicalSize::new(1000, 900), 1.0);
    let mut model = SettingsModel::default();
    context.run(|context| show_ui(context, &mut model));
    for interaction in ["slider", "tabs", "drag"] {
        if interaction == "drag" {
            context.on_window_event(&WindowEvent::CursorMoved {
                device_id: DeviceId::dummy(),
                position: PhysicalPosition::new(100.0, 48.0),
            });
            context.on_window_event(&WindowEvent::MouseInput {
                device_id: DeviceId::dummy(),
                state: ElementState::Pressed,
                button: MouseButton::Left,
            });
        }
        let before = context.cache_stats();
        let start = Instant::now();
        for frame in 0..200 {
            match interaction {
                "slider" => model.draft.autosave_minutes = (frame % 30 + 1) as u32,
                "tabs" => model.section = Section::ALL[frame % 3],
                "drag" => {
                    context.on_window_event(&WindowEvent::CursorMoved {
                        device_id: DeviceId::dummy(),
                        position: PhysicalPosition::new(100.0 + (frame % 100) as f64, 48.0),
                    });
                }
                _ => unreachable!(),
            }
            context.run(|context| show_ui(context, &mut model));
        }
        let elapsed = start.elapsed();
        let after = context.cache_stats();
        println!(
            "{interaction}: {:.3} ms/pass, {:.1} tessellated/pass, {:.1} reused/pass, {} mesh rebuilds, {} vertex bytes + {} index bytes/frame",
            elapsed.as_secs_f64() * 1000.0 / 200.0,
            (after.tessellated_elements - before.tessellated_elements) as f64 / 200.0,
            (after.reused_elements - before.reused_elements) as f64 / 200.0,
            after.geometry_rebuilds - before.geometry_rebuilds,
            context.draw_data().vertices.len() * std::mem::size_of::<zaxis::Vertex>(),
            context.draw_data().indices.len() * std::mem::size_of::<u32>(),
        );
    }
}

#[cfg(target_os = "windows")]
#[test]
#[ignore = "GPU drag benchmark; opens a native window"]
#[allow(deprecated)]
fn profile_blur_drag() {
    use std::sync::Arc;
    use zaxis::winit::platform::windows::EventLoopBuilderExtWindows;
    let event_loop = zaxis::winit::event_loop::EventLoop::builder()
        .with_any_thread(true)
        .build()
        .unwrap();
    let window = Arc::new(
        event_loop
            .create_window(
                zaxis::winit::window::Window::default_attributes()
                    .with_title("zaxis blur drag benchmark")
                    .with_inner_size(zaxis::winit::dpi::LogicalSize::new(820.0, 780.0)),
            )
            .unwrap(),
    );
    let mut renderer = pollster::block_on(zaxis::Renderer::new_with_presentation_mode(
        Arc::clone(&window),
        PresentationMode::Immediate,
    ))
    .unwrap();
    let mut context = Context::new();
    context.set_viewport(window.inner_size(), window.scale_factor());
    let mut model = SettingsModel::default();
    model.section = Section::Appearance;
    context.run(|context| show_ui(context, &mut model));
    context.on_window_event(&WindowEvent::CursorMoved {
        device_id: DeviceId::dummy(),
        position: PhysicalPosition::new(100.0, 48.0),
    });
    context.on_window_event(&WindowEvent::MouseInput {
        device_id: DeviceId::dummy(),
        state: ElementState::Pressed,
        button: MouseButton::Left,
    });
    println!(
        "GPU: {:?}, DPI: {}",
        renderer.adapter_info(),
        window.scale_factor()
    );
    for radius in [0, 1, 12, 32] {
        model.draft.blur_enabled = radius > 0;
        model.draft.blur_radius = radius;
        let mut timings = Vec::new();
        let mut cpu = 0.0;
        for n in 0..150 {
            context.on_window_event(&WindowEvent::CursorMoved {
                device_id: DeviceId::dummy(),
                position: PhysicalPosition::new(100.0 + (n % 80) as f64, 48.0 + (n % 16) as f64),
            });
            let start = Instant::now();
            context.run(|context| show_ui(context, &mut model));
            let ui_time = start.elapsed().as_secs_f64() * 1000.0;
            let start = Instant::now();
            assert_eq!(
                renderer
                    .render(context.draw_data(), context.style().background)
                    .unwrap(),
                zaxis::RenderStatus::Presented
            );
            if n >= 30 {
                cpu += ui_time;
                timings.push(start.elapsed().as_secs_f64() * 1000.0);
            }
        }
        let average = timings.iter().sum::<f64>() / timings.len() as f64;
        timings.sort_by(f64::total_cmp);
        println!("sigma {radius:2}: UI {:.3} ms, render/present {average:.3} ms, p95 {:.3} ms, {} blur regions",
            cpu / timings.len() as f64, timings[timings.len()*95/100],
            context.draw_data().commands.iter().filter(|c| c.blur.is_some()).count());
    }
}

#[test]
fn apply_cancel_and_defaults_keep_committed_values_separate() {
    let mut model = SettingsModel::default();
    model.draft.volume = 80;
    model.draft.preview_color = Color::rgb(12, 34, 56);
    model.draft.autosave = false;
    assert!(model.dirty());
    assert_eq!(model.applied.volume, 50);
    model.apply();
    assert!(!model.dirty());
    model.draft.volume = 20;
    model.draft.preview_color = Color::rgb(255, 0, 0);
    model.cancel();
    assert_eq!(model.draft.volume, 80);
    assert_eq!(model.draft.preview_color, Color::rgb(12, 34, 56));
    assert!(!model.dirty());
    model.reset();
    assert_eq!(model.draft, Settings::default());
    assert_eq!(model.applied.volume, 80);
    assert!(model.dirty());
}

#[test]
fn all_sections_fit_the_minimum_viewport_at_two_dpi_scales() {
    for scale in [1.0, 2.0] {
        let mut context = Context::new();
        let mut style = context.style().clone();
        style.motion.reduced_motion = true;
        context.set_style(style);
        context.set_viewport(
            PhysicalSize::new((760.0 * scale) as u32, (720.0 * scale) as u32),
            scale,
        );
        let mut model = SettingsModel::default();
        for section in Section::ALL {
            model.section = section;
            assert!(context.run(|context| show_ui(context, &mut model)));
            let data = context.draw_data();
            assert!(!data.vertices.is_empty());
            // The last draw command is the footer text, after all page controls.
            let footer = data.commands.last().unwrap();
            for index in &data.indices[footer.indices.start as usize..footer.indices.end as usize] {
                let position = data.vertices[*index as usize].position;
                assert!(
                    footer.clip_rect.contains(vec2(position[0], position[1])),
                    "clipped footer in {section:?} at DPI {scale}"
                );
            }
            assert!(!model.dirty());
            // Card frames settle after one scheduled redraw; then the page must idle.
            for _ in 0..3 {
                if !context.needs_repaint() {
                    break;
                }
                context.run(|context| show_ui(context, &mut model));
            }
            let revision = context.draw_data().revision;
            context.run(|context| show_ui(context, &mut model));
            assert_eq!(context.draw_data().revision, revision);
            assert!(!context.needs_repaint());
        }
    }
}

#[test]
fn expanded_internal_picker_fits_and_mode_switches_preserve_color() {
    for scale in [1.0, 2.0] {
        let mut context = Context::new();
        context.set_viewport(
            PhysicalSize::new((760.0 * scale) as u32, (720.0 * scale) as u32),
            scale,
        );
        let mut model = SettingsModel {
            section: Section::Appearance,
            internal_color_picker: true,
            picker_open_by_default: true,
            ..Default::default()
        };
        for internal in [true, false, true] {
            model.internal_color_picker = internal;
            context.run(|context| show_ui(context, &mut model));
            assert!(!model.dirty());
            assert_eq!(model.draft.preview_color, Color::rgb(78, 133, 190));
            if internal {
                let data = context.draw_data();
                let footer = data.commands.last().unwrap();
                for index in
                    &data.indices[footer.indices.start as usize..footer.indices.end as usize]
                {
                    let position = data.vertices[*index as usize].position;
                    assert!(
                        footer.clip_rect.contains(vec2(position[0], position[1])),
                        "internal picker clipped the footer at DPI {scale}"
                    );
                }
            }
        }
    }
}

#[test]
fn disabling_dependencies_preserves_draft_values_across_sections() {
    let mut context = Context::new();
    context.set_viewport(PhysicalSize::new(820, 780), 1.0);
    let mut model = SettingsModel::default();
    model.draft.autosave = false;
    model.draft.autosave_minutes = 17;
    model.draft.notifications = false;
    model.draft.sound = true;
    model.draft.volume = 85;
    for section in Section::ALL {
        model.section = section;
        context.run(|context| show_ui(context, &mut model));
    }
    assert_eq!(model.draft.autosave_minutes, 17);
    assert!(model.draft.sound);
    assert_eq!(model.draft.volume, 85);
    assert_eq!(model.applied, Settings::default());
}

#[test]
fn blur_updates_actual_chrome_and_cancel_restores_it() {
    let mut context = Context::new();
    context.set_viewport(PhysicalSize::new(820, 780), 1.0);
    let mut model = SettingsModel::default();
    model.section = Section::Appearance;
    context.run(|context| show_ui(context, &mut model));
    assert_eq!(
        context
            .draw_data()
            .commands
            .iter()
            .filter(|c| c.blur.is_some())
            .count(),
        1
    );
    model.draft.blur_radius = 24;
    context.run(|context| show_ui(context, &mut model));
    assert_eq!(context.style().blur_radius, 24.0);
    model.apply();
    model.draft.blur_enabled = false;
    context.run(|context| show_ui(context, &mut model));
    assert!(context
        .draw_data()
        .commands
        .iter()
        .all(|c| c.blur.is_none()));
    model.cancel();
    context.run(|context| show_ui(context, &mut model));
    assert_eq!(context.style().blur_radius, 24.0);
    assert!(context
        .draw_data()
        .commands
        .iter()
        .any(|c| c.blur == Some(24.0)));
}

#[test]
fn exit_asks_only_for_unapplied_changes_and_waits_for_the_answer() {
    use zaxis::winit::keyboard::KeyCode;
    let mut context = Context::new();
    context.set_viewport(PhysicalSize::new(1000, 900), 1.0);
    let mut model = SettingsModel::default();
    let mut frames = |context: &mut Context, model: &mut SettingsModel| {
        for _ in 0..4 {
            context.run(|context| show_ui(context, model));
        }
    };
    let mut press = |context: &mut Context, model: &mut SettingsModel, key| {
        context.on_key_event(key, ElementState::Pressed, false);
        context.run(|context| show_ui(context, model));
        context.on_key_event(key, ElementState::Released, false);
        frames(context, model);
    };
    frames(&mut context, &mut model);
    assert!(model.request_close(), "nothing unapplied: exit at once");
    model.draft.autosave_minutes += 1;
    model.draft.confirm_exit = false;
    assert!(model.request_close(), "confirmation is switched off");
    model.draft.confirm_exit = true;
    assert!(!model.request_close(), "unapplied changes ask first");
    frames(&mut context, &mut model);
    assert!(!model.exit_confirmed);
    // Escape cancels and nothing exits.
    press(&mut context, &mut model, KeyCode::Escape);
    assert!(!model.exit_confirm_open && !model.exit_confirmed);
    // Focus starts on Cancel; Tab reaches Exit, Enter confirms exactly once.
    assert!(!model.request_close());
    frames(&mut context, &mut model);
    press(&mut context, &mut model, KeyCode::Tab);
    press(&mut context, &mut model, KeyCode::Enter);
    assert!(model.exit_confirmed);
    assert!(!model.exit_confirm_open);
}
