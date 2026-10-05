//! Modal scenarios: closed overhead, open/close, steady state and large content.
//! Assertions run outside the timed pass and use only the public API.
use std::time::Duration;
use zaxis::winit::{
    dpi::PhysicalPosition,
    event::{DeviceId, MouseScrollDelta, TouchPhase, WindowEvent},
};
use zaxis::Instant;
use zaxis::{Context, Dialog, DialogAction, Rect, Root, ScrollArea, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModalCase {
    /// Closed modal in a live scene: it must cost nothing and leave nothing behind.
    Closed,
    /// Open on even steps, close on odd steps.
    Toggle,
    /// Open and still: no repaint, no new geometry, hover blocked.
    Steady,
    /// A long body that scrolls under the wheel while the scene below stays put.
    Large,
}
impl ModalCase {
    pub fn name(self) -> &'static str {
        match self {
            Self::Closed => "modal_closed",
            Self::Toggle => "modal_open_close",
            Self::Steady => "modal_steady",
            Self::Large => "modal_large_content",
        }
    }
    pub fn interactive(self) -> bool {
        matches!(self, Self::Toggle | Self::Large)
    }
}

pub struct Probe {
    kind: ModalCase,
    count: usize,
    open: bool,
    step: usize,
    now: Instant,
    mounted: bool,
    under: Rect,
    under_hovered: bool,
    under_offset: f32,
    first_row: f32,
    previous_first_row: f32,
    closed_vertices: usize,
    top_row: f32,
    revision: u64,
    tessellations: u64,
}

impl Probe {
    pub fn new(context: &mut Context, kind: ModalCase, count: usize) -> Self {
        let mut style = context.style().clone();
        style.motion.reduced_motion = true;
        context.set_style(style);
        let mut probe = Self {
            kind,
            count,
            open: false,
            step: 0,
            now: Instant::now(),
            mounted: false,
            under: Rect::default(),
            under_hovered: false,
            under_offset: 0.0,
            first_row: 0.0,
            previous_first_row: 0.0,
            closed_vertices: 0,
            top_row: 0.0,
            revision: 0,
            tessellations: 0,
        };
        // Settle the scene while closed and remember what "nothing open" draws.
        for _ in 0..3 {
            probe.build(context);
        }
        probe.closed_vertices = context.draw_data().vertices.len();
        probe.open = kind != ModalCase::Closed && kind != ModalCase::Toggle;
        // Open kinds settle their first measurement pass before timing starts.
        for _ in 0..4 {
            probe.build(context);
        }
        probe.top_row = probe.first_row;
        probe
    }

    pub fn input(&mut self, context: &mut Context, step: usize) {
        self.step = step;
        self.now += Duration::from_millis(20);
        self.revision = context.draw_data().revision;
        self.tessellations = context.cache_stats().tessellated_elements;
        self.previous_first_row = self.first_row;
        if self.kind == ModalCase::Toggle {
            self.open = step.is_multiple_of(2);
        }
        let viewport = context.viewport().center();
        let scale = f64::from(context.scale_factor());
        let target = match self.kind {
            ModalCase::Large => viewport,
            ModalCase::Steady => self.under.center(),
            _ => Vec2::splat(2.0),
        };
        context.on_window_event(&WindowEvent::CursorMoved {
            device_id: DeviceId::dummy(),
            position: PhysicalPosition::new(
                f64::from(target.x) * scale,
                f64::from(target.y) * scale,
            ),
        });
        if self.kind == ModalCase::Large {
            let direction = if step.is_multiple_of(2) { -1.0 } else { 1.0 };
            let consumed = context
                .on_window_event(&WindowEvent::MouseWheel {
                    device_id: DeviceId::dummy(),
                    delta: MouseScrollDelta::PixelDelta(PhysicalPosition::new(
                        0.0,
                        direction * 40.0 * scale,
                    )),
                    phase: TouchPhase::Moved,
                })
                .consumed;
            assert!(consumed, "the wheel belongs to the modal");
        }
    }

    pub fn build(&mut self, context: &mut Context) {
        let (count, large) = (self.count, self.kind == ModalCase::Large);
        let mut first_row = None;
        let mut mounted = false;
        self.now += Duration::from_millis(1);
        context.run_at(self.now, |context| {
            Root::new().show(context, |ui| {
                let under = ui.button("Under the overlay");
                self.under = under.rect;
                self.under_hovered = under.hovered;
                let rows = ScrollArea::vertical()
                    .id_source("under")
                    .max_height(220.0)
                    .show_rows(ui, 24.0, count, |ui, n| {
                        ui.label(format!("Item {n}"));
                    });
                self.under_offset = rows.offset.y;
                let output = Dialog::new("bench", "Modal benchmark")
                    .description("Content under the overlay keeps running.")
                    .action(DialogAction::new("Cancel"))
                    .action(DialogAction::new("OK").primary())
                    .show(ui, &mut self.open, |ui| {
                        let lines = if large { count } else { 3 };
                        for n in 0..lines {
                            let row = ui.label(format!("Line {n}"));
                            if n == 0 {
                                first_row = Some(row.rect.min.y);
                            }
                        }
                    });
                mounted = output.is_some();
            });
        });
        self.mounted = mounted;
        if let Some(y) = first_row {
            self.first_row = y;
        }
    }

    pub fn verify(&self, context: &Context) {
        match self.kind {
            ModalCase::Closed => {
                assert!(!self.mounted, "a closed modal builds nothing");
                assert!(!context.needs_repaint());
                assert!(!context.wants_animation_frame());
                assert_eq!(self.revision, context.draw_data().revision);
                assert_eq!(
                    self.tessellations,
                    context.cache_stats().tessellated_elements
                );
            }
            ModalCase::Toggle => {
                if self.step.is_multiple_of(2) {
                    assert!(self.mounted && self.open);
                } else {
                    assert!(!self.mounted, "closed in a single pass with reduced motion");
                    assert_eq!(
                        context.draw_data().vertices.len(),
                        self.closed_vertices,
                        "closing leaves no geometry behind"
                    );
                }
            }
            ModalCase::Steady => {
                assert!(self.mounted);
                assert!(!self.under_hovered, "the overlay blocks hover below");
                assert!(
                    !context.needs_repaint(),
                    "an open, still modal needs no frames"
                );
                assert!(!context.wants_animation_frame());
                assert_eq!(self.revision, context.draw_data().revision);
                assert_eq!(
                    self.tessellations,
                    context.cache_stats().tessellated_elements
                );
            }
            ModalCase::Large => {
                assert!(self.mounted);
                assert_eq!(
                    self.under_offset, 0.0,
                    "wheel input never scrolls the scene below"
                );
                if self.step.is_multiple_of(2) {
                    assert!(
                        self.first_row < self.previous_first_row,
                        "the body scrolled down"
                    );
                } else if self.previous_first_row < self.top_row {
                    assert!(
                        self.first_row > self.previous_first_row,
                        "the body scrolled up"
                    );
                }
            }
        }
    }
}
