//! NumberInput and DragValue: a form of numeric controls left alone, typed into and
//! committed, stepped with the arrows, and a DragValue dragged. Input goes through the
//! public event path; assertions run outside the timed pass.
use std::time::Duration;
use zaxis::winit::{
    dpi::PhysicalPosition,
    event::{DeviceId, ElementState, MouseButton, WindowEvent},
    keyboard::{KeyCode, ModifiersState},
};
use zaxis::Instant;
use zaxis::{Context, DragValue, NumberInput, Rect, Root, ScrollArea, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumberCase {
    /// Nothing changed: no repaint, no new geometry.
    Idle,
    /// Select all, type a number and press Enter in a focused NumberInput.
    Typing,
    /// ArrowUp and ArrowDown step a focused NumberInput.
    Keys,
    /// A DragValue dragged right, then left.
    Drag,
}

impl NumberCase {
    pub fn name(self) -> &'static str {
        match self {
            Self::Idle => "number_idle",
            Self::Typing => "number_typing_commit",
            Self::Keys => "number_arrow_steps",
            Self::Drag => "drag_value_drag",
        }
    }
    pub fn interactive(self) -> bool {
        self != Self::Idle
    }
}

/// Rows of controls in the form; the rest of the objects scroll out of view.
const ROWS: usize = 64;

pub struct Probe {
    kind: NumberCase,
    inputs: Vec<f64>,
    drags: Vec<i64>,
    /// The first row's NumberInput and DragValue.
    input: Rect,
    drag: Rect,
    changed: bool,
    submitted: bool,
    now: Instant,
    step: usize,
    before: (f64, i64),
    revision: u64,
    tessellations: u64,
}

impl Probe {
    pub fn new(context: &mut Context, kind: NumberCase, count: usize) -> Self {
        let rows = count.clamp(1, ROWS);
        let mut probe = Self {
            kind,
            inputs: (0..rows).map(|i| i as f64 * 0.5).collect(),
            drags: (0..rows).map(|i| i as i64).collect(),
            input: Rect::default(),
            drag: Rect::default(),
            changed: false,
            submitted: false,
            now: Instant::now(),
            step: 0,
            before: (0.0, 0),
            revision: 0,
            tessellations: 0,
        };
        for _ in 0..3 {
            probe.build(context);
        }
        if matches!(kind, NumberCase::Typing | NumberCase::Keys) {
            pointer(context, probe.input.center());
            click(context);
            for _ in 0..3 {
                probe.build(context);
            }
        }
        probe
    }

    pub fn input(&mut self, context: &mut Context, step: usize) {
        self.step = step;
        self.now += Duration::from_millis(20);
        self.before = (self.inputs[0], self.drags[0]);
        self.revision = context.draw_data().revision;
        self.tessellations = context.cache_stats().tessellated_elements;
        match self.kind {
            NumberCase::Idle => {}
            NumberCase::Typing => {
                modifiers(context, ModifiersState::CONTROL);
                key(context, KeyCode::KeyA);
                modifiers(context, ModifiersState::empty());
                let typed = if step.is_multiple_of(2) { "12.5" } else { "7" };
                assert!(
                    context.on_text_event(typed).consumed,
                    "the field takes text"
                );
                key(context, KeyCode::Enter);
            }
            NumberCase::Keys => key(
                context,
                if step % 64 < 32 {
                    KeyCode::ArrowUp
                } else {
                    KeyCode::ArrowDown
                },
            ),
            NumberCase::Drag => {
                let from = self.drag.center();
                let dx = if step.is_multiple_of(2) { 40.0 } else { -40.0 };
                pointer(context, from);
                mouse(context, ElementState::Pressed);
                pointer(context, from + Vec2::new(dx, 0.0));
                mouse(context, ElementState::Released);
            }
        }
    }

    pub fn build(&mut self, context: &mut Context) {
        self.now += Duration::from_millis(1);
        let (mut changed, mut submitted) = (false, false);
        let (mut input, mut drag) = (Rect::default(), Rect::default());
        context.run_at(self.now, |context| {
            Root::new().show(context, |ui| {
                ScrollArea::vertical()
                    .id_source("numbers")
                    .max_height(560.0)
                    .show(ui, |ui| {
                        for (row, (value, count)) in
                            self.inputs.iter_mut().zip(&mut self.drags).enumerate()
                        {
                            ui.horizontal(|ui| {
                                let a = ui.add(
                                    NumberInput::new(value)
                                        .id_source(("input", row))
                                        .range(-1000.0..=1000.0)
                                        .step(0.5)
                                        .precision(2)
                                        .width(140.0),
                                );
                                let b = ui.add(
                                    DragValue::new(count)
                                        .id_source(("drag", row))
                                        .range(-100_000..=100_000)
                                        .suffix(" px")
                                        .width(120.0),
                                );
                                if row == 0 {
                                    (input, drag) = (a.rect, b.rect);
                                    changed = a.changed() || b.changed();
                                    submitted = a.submitted();
                                }
                            });
                        }
                    });
            });
        });
        (self.input, self.drag) = (input, drag);
        (self.changed, self.submitted) = (changed, submitted);
    }

    pub fn verify(&self, context: &Context) {
        let (value, count) = (self.inputs[0], self.drags[0]);
        match self.kind {
            NumberCase::Idle => {
                assert!(!context.needs_repaint(), "an unchanged form needs no frame");
                assert_eq!(self.revision, context.draw_data().revision);
                assert_eq!(
                    self.tessellations,
                    context.cache_stats().tessellated_elements
                );
            }
            NumberCase::Typing => {
                let expected = if self.step.is_multiple_of(2) {
                    12.5
                } else {
                    7.0
                };
                assert_eq!(value, expected, "Enter commits the typed number");
                assert!(self.submitted && self.changed);
            }
            NumberCase::Keys => {
                let delta = if self.step % 64 < 32 { 0.5 } else { -0.5 };
                assert_eq!(value, self.before.0 + delta, "an arrow steps once");
                assert!(self.changed);
            }
            NumberCase::Drag => {
                let moved = count - self.before.1;
                assert!(
                    if self.step.is_multiple_of(2) {
                        moved > 0
                    } else {
                        moved < 0
                    },
                    "the drag moved the value by {moved}"
                );
                assert!(self.changed);
                assert_eq!(value, self.before.0, "the drag leaves the input alone");
            }
        }
    }
}

fn pointer(context: &mut Context, at: Vec2) {
    let scale = f64::from(context.scale_factor());
    context.on_window_event(&WindowEvent::CursorMoved {
        device_id: DeviceId::dummy(),
        position: PhysicalPosition::new(f64::from(at.x) * scale, f64::from(at.y) * scale),
    });
}

fn mouse(context: &mut Context, state: ElementState) {
    context.on_window_event(&WindowEvent::MouseInput {
        device_id: DeviceId::dummy(),
        state,
        button: MouseButton::Left,
    });
}

fn click(context: &mut Context) {
    mouse(context, ElementState::Pressed);
    mouse(context, ElementState::Released);
}

fn modifiers(context: &mut Context, state: ModifiersState) {
    context.on_window_event(&WindowEvent::ModifiersChanged(state.into()));
}

fn key(context: &mut Context, code: KeyCode) {
    context.on_key_event(code, ElementState::Pressed, false);
    context.on_key_event(code, ElementState::Released, false);
}
