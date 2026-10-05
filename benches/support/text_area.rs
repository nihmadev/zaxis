//! Multi-line TextEdit scenarios over documents from small to stress size: an unchanged
//! frame, typing, a large paste with undo, wheel scrolling and width changes. Input goes
//! through the public event path; assertions run outside the timed pass.
use std::time::Duration;
use zaxis::winit::{
    dpi::PhysicalPosition,
    event::{DeviceId, ElementState, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent},
    keyboard::{KeyCode, ModifiersState},
};
use zaxis::Instant;
use zaxis::{Context, Rect, Root, TextEdit, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextAreaCase {
    /// Nothing changed: no repaint, no shaping, no geometry.
    Idle,
    /// One typed character per step, with a line break now and then.
    Typing,
    /// Even steps paste a large block, odd steps undo it.
    Paste,
    /// Wheel scrolling through the document.
    Scroll,
    /// The field width alternates, re-wrapping the visible paragraphs.
    Resize,
}

impl TextAreaCase {
    pub fn name(self) -> &'static str {
        match self {
            Self::Idle => "text_area_idle",
            Self::Typing => "text_area_typing",
            Self::Paste => "text_area_paste_block",
            Self::Scroll => "text_area_scroll",
            Self::Resize => "text_area_resize",
        }
    }
    pub fn interactive(self) -> bool {
        self != Self::Idle
    }
}

/// Paragraphs shaped per step may not grow with the document: only what is on screen.
const VISIBLE_BUDGET: u64 = 64;

pub struct Probe {
    kind: TextAreaCase,
    text: String,
    base_len: usize,
    block: String,
    rect: Rect,
    width: f32,
    changed: bool,
    now: Instant,
    step: usize,
    len_before: usize,
    built: u64,
    revision: u64,
    tessellations: u64,
}

fn line(n: usize) -> String {
    format!(
        "Line {n}: Привет, мир — the quick brown fox jumps over the lazy dog, again {n} and again"
    )
}

impl Probe {
    pub fn new(context: &mut Context, kind: TextAreaCase, lines: usize) -> Self {
        let text = (0..lines.max(1)).map(line).collect::<Vec<_>>().join("\n");
        let block = (0..lines.clamp(1, 1000))
            .map(|n| format!("pasted {n} é👩‍💻 العربية"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut probe = Self {
            kind,
            base_len: text.len(),
            text,
            block,
            rect: Rect::default(),
            width: 520.0,
            changed: false,
            now: Instant::now(),
            step: 0,
            len_before: 0,
            built: 0,
            revision: 0,
            tessellations: 0,
        };
        for _ in 0..3 {
            probe.build(context);
        }
        if matches!(kind, TextAreaCase::Typing | TextAreaCase::Paste) {
            let at = probe.rect.center();
            pointer(context, at);
            for state in [ElementState::Pressed, ElementState::Released] {
                context.on_window_event(&WindowEvent::MouseInput {
                    device_id: DeviceId::dummy(),
                    state,
                    button: MouseButton::Left,
                });
            }
            // Jump to the end so typing appends and the paste lands there.
            modifiers(context, ModifiersState::CONTROL);
            key(context, KeyCode::End);
            modifiers(context, ModifiersState::empty());
            // The first edit scrolls the caret into view; settle that before timing.
            context.on_text_event(" ");
        }
        for _ in 0..4 {
            probe.build(context);
        }
        probe.base_len = probe.text.len();
        probe
    }

    pub fn input(&mut self, context: &mut Context, step: usize) {
        self.step = step;
        self.now += Duration::from_millis(20);
        self.len_before = self.text.len();
        self.built = context.cache_stats().text_layouts_built;
        self.revision = context.draw_data().revision;
        self.tessellations = context.cache_stats().tessellated_elements;
        match self.kind {
            TextAreaCase::Idle => {}
            TextAreaCase::Typing => {
                let typed = if step % 25 == 24 { "\n" } else { "a" };
                assert!(context.on_text_event(typed).consumed);
            }
            TextAreaCase::Paste => {
                if step.is_multiple_of(2) {
                    assert!(context.on_text_event(&self.block).consumed);
                } else {
                    modifiers(context, ModifiersState::CONTROL);
                    key(context, KeyCode::KeyZ);
                    modifiers(context, ModifiersState::empty());
                }
            }
            TextAreaCase::Scroll => {
                pointer(context, self.rect.center());
                let direction = if step % 64 < 32 { -1.0 } else { 1.0 };
                let scale = f64::from(context.scale_factor());
                let consumed = context
                    .on_window_event(&WindowEvent::MouseWheel {
                        device_id: DeviceId::dummy(),
                        delta: MouseScrollDelta::PixelDelta(PhysicalPosition::new(
                            0.0,
                            direction * 17.375 * scale,
                        )),
                        phase: TouchPhase::Moved,
                    })
                    .consumed;
                assert!(consumed, "the wheel scrolls the field");
            }
            TextAreaCase::Resize => {
                self.width = if step.is_multiple_of(2) { 420.0 } else { 560.0 };
                context.request_repaint();
            }
        }
    }

    pub fn build(&mut self, context: &mut Context) {
        let width = self.width;
        self.now += Duration::from_millis(1);
        context.run_at(self.now, |context| {
            Root::new().show(context, |ui| {
                let response = ui.add(
                    TextEdit::new(&mut self.text)
                        .id_source("bench-area")
                        .multiline()
                        .width(width)
                        .rows(12.0),
                );
                self.rect = response.rect;
                self.changed = response.changed();
            });
        });
    }

    pub fn verify(&self, context: &Context) {
        let built = context.cache_stats().text_layouts_built - self.built;
        match self.kind {
            TextAreaCase::Idle => {
                assert!(
                    !context.needs_repaint(),
                    "an unchanged field needs no frame"
                );
                assert_eq!(self.revision, context.draw_data().revision);
                assert_eq!(built, 0, "nothing is shaped again");
                assert_eq!(
                    self.tessellations,
                    context.cache_stats().tessellated_elements
                );
            }
            TextAreaCase::Typing => {
                assert!(self.changed);
                assert_eq!(self.text.len(), self.len_before + 1);
                assert!(self
                    .text
                    .ends_with(if self.step % 25 == 24 { '\n' } else { 'a' }));
                // The first edit may scroll the caret into view and shape that page once.
                let allowed = if self.step == 0 { VISIBLE_BUDGET } else { 4 };
                assert!(
                    built <= allowed,
                    "typing shaped {built} paragraphs at step {}",
                    self.step
                );
            }
            TextAreaCase::Paste => {
                assert!(self.changed);
                if self.step.is_multiple_of(2) {
                    assert_eq!(self.text.len(), self.base_len + self.block.len());
                } else {
                    assert_eq!(self.text.len(), self.base_len, "undo removes the block");
                }
                assert!(built <= VISIBLE_BUDGET, "paste shaped {built} paragraphs");
            }
            TextAreaCase::Scroll => {
                assert!(!self.changed);
                assert_eq!(self.text.len(), self.base_len);
                assert!(
                    built <= VISIBLE_BUDGET,
                    "scrolling shaped {built} paragraphs"
                );
                assert_ne!(self.revision, context.draw_data().revision);
            }
            TextAreaCase::Resize => {
                let expected = if self.step.is_multiple_of(2) {
                    420.0
                } else {
                    560.0
                };
                assert_eq!(self.rect.size().x, expected);
                assert_eq!(self.text.len(), self.base_len);
                assert!(built <= VISIBLE_BUDGET, "re-wrap shaped {built} paragraphs");
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

fn modifiers(context: &mut Context, state: ModifiersState) {
    context.on_window_event(&WindowEvent::ModifiersChanged(state.into()));
}

fn key(context: &mut Context, code: KeyCode) {
    context.on_key_event(code, ElementState::Pressed, false);
    context.on_key_event(code, ElementState::Released, false);
}
