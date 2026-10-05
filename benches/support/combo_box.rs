//! Real ComboBox paths: cached/animated popup, keyboard, wheel, filters and live updates.
use crate::scene::Case;
use std::time::Duration;
use zaxis::winit::{
    dpi::PhysicalPosition,
    event::{DeviceId, ElementState, MouseScrollDelta, TouchPhase, WindowEvent},
    keyboard::{KeyCode, ModifiersState},
};
use zaxis::Instant;
use zaxis::{vec2, ComboBox, ComboBoxOption, Context, Response, Window};

pub struct Probe {
    options: Vec<ComboBoxOption<usize>>,
    selected: Option<usize>,
    trigger: Option<Response>,
    clock: Option<Instant>,
    revision: u64,
    tessellations: u64,
    filter_submitted: bool,
}
impl Probe {
    pub fn new(count: usize) -> Self {
        Self {
            options: (0..count)
                .map(|i| {
                    ComboBoxOption::new(i, i, format!("Group {} / Region {i} / Регион {i}", i % 10))
                })
                .collect(),
            selected: Some(count / 2),
            trigger: None,
            clock: None,
            revision: 0,
            tessellations: 0,
            filter_submitted: false,
        }
    }
    pub fn input(&mut self, c: &mut Context, case: Case, step: usize) {
        self.revision = c.draw_data().revision;
        self.tessellations = c.cache_stats().tessellated_elements;
        match case {
            Case::ComboToggle => {
                // Reverse while still animating, including close -> immediate reopen.
                if step.is_multiple_of(4) {
                    key(c, KeyCode::Enter);
                }
                if step % 4 == 2 {
                    key(c, KeyCode::Escape);
                }
            }
            Case::ComboKeys => key(
                c,
                if step % 64 < 32 {
                    KeyCode::ArrowDown
                } else {
                    KeyCode::ArrowUp
                },
            ),
            Case::ComboFilter => {
                c.on_window_event(&WindowEvent::ModifiersChanged(
                    ModifiersState::CONTROL.into(),
                ));
                key(c, KeyCode::KeyA);
                c.on_window_event(&WindowEvent::ModifiersChanged(
                    ModifiersState::empty().into(),
                ));
                self.filter_submitted = c
                    .on_text_event(if step.is_multiple_of(2) {
                        "Group 1"
                    } else {
                        "Group 2"
                    })
                    .consumed;
            }
            Case::ComboUpdates => {
                self.options.reverse();
            }
            Case::ComboScroll => {
                let trigger = self.trigger.unwrap().rect;
                let p = trigger.min + vec2(40.0, 80.0);
                c.on_window_event(&WindowEvent::CursorMoved {
                    device_id: DeviceId::dummy(),
                    position: PhysicalPosition::new(
                        f64::from(p.x * c.scale_factor()),
                        f64::from(p.y * c.scale_factor()),
                    ),
                });
                assert!(
                    c.on_window_event(&WindowEvent::MouseWheel {
                        device_id: DeviceId::dummy(),
                        phase: TouchPhase::Moved,
                        delta: MouseScrollDelta::PixelDelta(PhysicalPosition::new(
                            0.0,
                            if step % 64 < 32 { -17.375 } else { 17.375 }
                        ))
                    })
                    .consumed
                );
            }
            _ => {}
        }
    }
    pub fn build(&mut self, c: &mut Context, case: Case, step: usize) {
        let start = *self.clock.get_or_insert_with(Instant::now);
        if self.trigger.is_none() {
            let mut style = c.style().clone();
            style.text_edit_blink_interval = Duration::ZERO;
            c.set_style(style);
        }
        c.run_at(start + Duration::from_millis(step as u64 * 16), |c| {
            Window::new("Combo benchmark")
                .default_size(vec2(400.0, 360.0))
                .show(c, |ui| {
                    self.trigger = Some(
                        ui.add(
                            ComboBox::new(&mut self.selected, &self.options)
                                .id_source("benchmark")
                                .label("region")
                                .filterable(case == Case::ComboFilter)
                                .default_open(!matches!(
                                    case,
                                    Case::ComboClosed | Case::ComboToggle
                                )),
                        ),
                    );
                });
        });
        if case == Case::ComboToggle {
            c.request_focus(self.trigger.unwrap().id);
        }
    }
    pub fn verify(&self, c: &Context, case: Case, step: usize) {
        assert_eq!(self.selected, Some(self.options.len() / 2));
        assert!(
            c.draw_data().vertices.len() < 3000,
            "popup must virtualize rows"
        );
        if step > 12 && matches!(case, Case::ComboClosed | Case::ComboOpen) {
            assert_eq!(c.draw_data().revision, self.revision);
            assert_eq!(c.cache_stats().tessellated_elements, self.tessellations);
            assert!(c.next_repaint().is_none());
        }
        if step > 12 && case == Case::ComboFilter {
            assert!(self.filter_submitted);
        }
    }
}
fn key(c: &mut Context, key: KeyCode) {
    c.on_key_event(key, ElementState::Pressed, false);
    c.on_key_event(key, ElementState::Released, false);
}
