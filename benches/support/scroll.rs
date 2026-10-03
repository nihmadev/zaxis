//! Real scroll input and content, with assertions outside the timed pass.
use crate::scene::Case;
use std::time::{Duration, Instant};
use zaxis::winit::{
    dpi::PhysicalPosition,
    event::{DeviceId, ElementState, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent},
};
use zaxis::{vec2, Context, Rect, ScrollArea, Slider, Vec2, Window};

#[derive(Default)]
pub struct Probe {
    viewport: Rect,
    offset: Vec2,
    previous: Vec2,
    rows: usize,
    clock: Option<Instant>,
    middle_started: bool,
    revision: u64,
    tessellations: u64,
}
impl Probe {
    pub fn input(&mut self, context: &mut Context, case: Case, step: usize) {
        self.previous = self.offset;
        self.revision = context.draw_data().revision;
        self.tessellations = context.cache_stats().tessellated_elements;
        if case == Case::ScrollCached {
            context.request_repaint();
            return;
        }
        let direction = if step % 64 < 32 { 1.0 } else { -1.0 };
        let point = self.viewport.center();
        move_to(context, point);
        if case == Case::ScrollMiddle {
            if !self.middle_started {
                for state in [ElementState::Pressed, ElementState::Released] {
                    assert!(
                        context
                            .on_window_event(&WindowEvent::MouseInput {
                                device_id: DeviceId::dummy(),
                                state,
                                button: MouseButton::Middle,
                            })
                            .consumed
                    );
                }
                self.middle_started = true;
            }
            move_to(context, point + vec2(0.0, direction * 80.0));
        } else {
            let scale = context.scale_factor() as f64;
            assert!(
                context
                    .on_window_event(&WindowEvent::MouseWheel {
                        device_id: DeviceId::dummy(),
                        delta: MouseScrollDelta::PixelDelta(PhysicalPosition::new(
                            0.0,
                            -f64::from(direction) * 17.375 * scale
                        )),
                        phase: TouchPhase::Moved,
                    })
                    .consumed
            );
        }
    }
    pub fn build(
        &mut self,
        context: &mut Context,
        case: Case,
        step: usize,
        labels: &[String],
        checks: &mut [bool],
        values: &mut [f32],
    ) {
        let start = *self.clock.get_or_insert_with(Instant::now);
        // Deterministic integration: CPU/GPU runs measure the same scroll motion.
        context.run_at(start + Duration::from_millis(step as u64 * 16), |context| {
            Window::new("Scroll benchmark")
                .default_size(vec2(700.0, 680.0))
                .show(context, |ui| {
                    let out = ScrollArea::vertical()
                        .id_source("settings")
                        .max_height(540.0)
                        .show(ui, |ui| {
                            if case == Case::ScrollNested {
                                let inner = ScrollArea::vertical()
                                    .id_source("nested")
                                    .max_height(260.0)
                                    .show_rows(ui, 26.0, labels.len() * 50, |ui, index| {
                                        ui.label(format!("Nested row {index}"));
                                    });
                                self.viewport = inner.viewport;
                                self.offset = inner.offset;
                                self.rows = inner.inner.len();
                                ui.allocate_space(vec2(300.0, 1400.0));
                            } else if case == Case::ScrollRows {
                                let inner = ScrollArea::vertical()
                                    .id_source("rows")
                                    .max_height(500.0)
                                    .show_rows(ui, 26.0, labels.len() * 50, |ui, index| {
                                        ui.label(format!("Virtual row {index}"));
                                    });
                                self.viewport = inner.viewport;
                                self.offset = inner.offset;
                                self.rows = inner.inner.len();
                            } else {
                                for (index, label) in labels.iter().enumerate() {
                                    ui.push_id(index, |ui| {
                                        ui.checkbox(&mut checks[index], label);
                                        ui.add(Slider::new(&mut values[index], 0.0..=1.0));
                                    });
                                }
                            }
                        });
                    if !matches!(case, Case::ScrollNested | Case::ScrollRows) {
                        self.viewport = out.viewport;
                        self.offset = out.offset;
                    }
                });
        });
    }
    pub fn verify(&self, context: &Context, case: Case) {
        assert!(self.offset.is_finite());
        assert!(context.input().scroll_delta == Vec2::ZERO);
        if matches!(case, Case::ScrollNested | Case::ScrollRows) {
            assert!(
                self.rows > 0 && self.rows <= 21,
                "virtualization built hidden rows"
            );
        }
        if case == Case::ScrollCached {
            assert_eq!(context.draw_data().revision, self.revision);
            assert_eq!(
                context.cache_stats().tessellated_elements,
                self.tessellations
            );
        } else if case == Case::ScrollMiddle {
            assert!(context.is_auto_scrolling());
        } else {
            assert_ne!(self.offset, self.previous, "wheel did not move content");
        }
    }
}
fn move_to(context: &mut Context, point: Vec2) {
    let scale = context.scale_factor();
    context.on_window_event(&WindowEvent::CursorMoved {
        device_id: DeviceId::dummy(),
        position: PhysicalPosition::new((point.x * scale) as f64, (point.y * scale) as f64),
    });
}
