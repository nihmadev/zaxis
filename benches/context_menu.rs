//! Release CPU/UI stress, with real event routing and geometry generation.
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use zaxis::{
    winit::{
        dpi::{PhysicalPosition, PhysicalSize},
        event::{DeviceId, ElementState, MouseScrollDelta, TouchPhase, WindowEvent},
        keyboard::KeyCode,
    },
    *,
};

fn pointer(c: &mut Context, p: Vec2) {
    c.on_window_event(&WindowEvent::CursorMoved {
        device_id: DeviceId::dummy(),
        position: PhysicalPosition::new(p.x as f64, p.y as f64),
    });
}
fn percentile(samples: &[f64], p: f64) -> f64 {
    samples[((samples.len() - 1) as f64 * p).round() as usize]
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let value = |key: &str| args.windows(2).find(|a| a[0] == key).map(|a| a[1].clone());
    let iterations = value("--iterations")
        .map(|s| s.parse().unwrap())
        .unwrap_or(120usize);
    let output =
        PathBuf::from(value("--output").unwrap_or("target/context-menu-benchmark.json".into()));
    let mut results = Vec::new();
    for count in [10, 100, 1000, 10000] {
        for case in ["idle", "hover", "keys", "scroll"] {
            let items: Vec<_> = (0..count)
                .map(|i| {
                    if i % 10 == 9 {
                        ContextMenuItem::separator()
                    } else {
                        ContextMenuItem::new(i, format!("Action {i:05}"))
                            .icon("+")
                            .right_text(format!("Ctrl+{}", i % 9))
                            .enabled(i % 11 != 5)
                    }
                })
                .collect();
            let mut c = Context::new();
            c.set_viewport(PhysicalSize::new(800, 600), 1.0);
            let clock = Instant::now();
            let mut samples = Vec::new();
            let mut animated = 0;
            let mut max_vertices = 0;
            let mut tessellations = 0;
            let mut popup_rect = None;
            for step in 0..iterations + 20 {
                let before = c.cache_stats().tessellated_elements;
                let start = Instant::now();
                if let Some(rect) = popup_rect {
                    let rect: Rect = rect;
                    match case {
                        "hover" => pointer(
                            &mut c,
                            rect.min + vec2(50.0, 15.0 + (step % 8) as f32 * 25.0),
                        ),
                        "keys" => {
                            c.on_key_event(
                                if step % 32 < 16 {
                                    KeyCode::ArrowDown
                                } else {
                                    KeyCode::ArrowUp
                                },
                                ElementState::Pressed,
                                false,
                            );
                        }
                        "scroll" => {
                            pointer(&mut c, rect.min + vec2(50.0, 50.0));
                            c.on_window_event(&WindowEvent::MouseWheel {
                                device_id: DeviceId::dummy(),
                                phase: TouchPhase::Moved,
                                delta: MouseScrollDelta::PixelDelta(PhysicalPosition::new(
                                    0.0,
                                    if step % 64 < 32 { -23.0 } else { 23.0 },
                                )),
                            });
                        }
                        _ => {}
                    }
                }
                c.run_at(clock + Duration::from_millis(step as u64 * 16), |c| {
                    Root::new().show(c, |ui| {
                        let target = ui.button("Target");
                        let mut menu = ContextMenu::new("bench", &items);
                        if step == 0 {
                            menu = menu.open_at(vec2(100.0, 100.0));
                        }
                        let result = menu.show(ui, target);
                        assert!(result.open && result.selected.is_none());
                        popup_rect = result.rect;
                    });
                });
                let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                if step >= 20 {
                    samples.push(elapsed);
                    animated += usize::from(c.wants_animation_frame());
                    max_vertices = max_vertices.max(c.draw_data().vertices.len());
                    tessellations += c.cache_stats().tessellated_elements - before;
                }
            }
            samples.sort_by(f64::total_cmp);
            let p50 = percentile(&samples, 0.5);
            let p95 = percentile(&samples, 0.95);
            let p99 = percentile(&samples, 0.99);
            println!("{case:6} {count:5}: p50 {p50:.3} ms p95 {p95:.3} ms p99 {p99:.3} ms; vertices {max_vertices}; animation frames {animated}/{iterations}");
            results.push(serde_json::json!({"case":case,"items":count,"p50_ms":p50,"p95_ms":p95,"p99_ms":p99,
                "max_vertices":max_vertices,"tessellations":tessellations,"animation_frames":animated}));
        }
    }
    std::fs::write(
        output,
        serde_json::to_string_pretty(
            &serde_json::json!({"scope":"release CPU events + UI + geometry; no GPU timings",
        "iterations":iterations,"warmup":20,"results":results}),
        )
        .unwrap(),
    )
    .unwrap();
}
