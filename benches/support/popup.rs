//! Popup scenarios: closed triggers, one popup, chains opened from inside each other up to
//! the depth limit, open/close, Escape, wheel, a moving anchor, model changes and many
//! distinct popups over time. Assertions run outside the timed pass and use the public API.
use std::time::Duration;
use zaxis::testing::Inspect;
use zaxis::winit::{
    dpi::PhysicalPosition,
    event::{DeviceId, ElementState, MouseScrollDelta, TouchPhase, WindowEvent},
    keyboard::KeyCode,
};
use zaxis::Instant;
use zaxis::{vec2, Context, Popup, Rect, Root, ScrollArea, Ui, MAX_POPUP_DEPTH};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PopupCase {
    /// Many closed triggers in a live scene: no layers, no frames.
    Closed,
    /// One simple popup, open and still.
    Single,
    /// Chains opened from inside each other, open and still.
    Chain2,
    Chain4,
    ChainMax,
    /// A chain of four opens and closes whole on alternate steps.
    Toggle,
    /// Escape closes the leaf of a chain of four; the next step opens it again.
    Keys,
    /// The wheel over the leaf of a chain of four scrolls its list.
    Wheel,
    /// The root trigger moves; every level follows its anchor and stays on screen.
    Moving,
    /// The application changes a value shown in the leaf every step.
    Model,
    /// One of many roots opens per step, replacing the one before.
    Lifecycle,
}

impl PopupCase {
    pub fn name(self) -> &'static str {
        match self {
            Self::Closed => "popup_closed",
            Self::Single => "popup_single",
            Self::Chain2 => "popup_chain_2",
            Self::Chain4 => "popup_chain_4",
            Self::ChainMax => "popup_chain_max",
            Self::Toggle => "popup_chain_open_close",
            Self::Keys => "popup_chain_escape",
            Self::Wheel => "popup_chain_wheel",
            Self::Moving => "popup_chain_moving_anchor",
            Self::Model => "popup_chain_model",
            Self::Lifecycle => "popup_lifecycle",
        }
    }

    pub fn interactive(self) -> bool {
        !matches!(
            self,
            Self::Closed | Self::Single | Self::Chain2 | Self::Chain4 | Self::ChainMax
        )
    }

    /// How many popups are expected open at rest.
    fn depth(self) -> usize {
        match self {
            Self::Closed => 0,
            Self::Single | Self::Lifecycle => 1,
            Self::Chain2 => 2,
            Self::ChainMax => MAX_POPUP_DEPTH,
            _ => 4,
        }
    }
}

pub struct Probe {
    kind: PopupCase,
    rows: usize,
    flags: Vec<bool>,
    triggers: Vec<Rect>,
    model: bool,
    shown: bool,
    scroll: f32,
    previous_scroll: f32,
    root_at: f32,
    step: usize,
    now: Instant,
    revision: u64,
    tessellations: u64,
    consumed: bool,
}

impl Probe {
    pub fn new(context: &mut Context, kind: PopupCase, count: usize) -> Self {
        let mut style = context.style().clone();
        style.motion.reduced_motion = true;
        context.set_style(style);
        let levels = match kind {
            PopupCase::Closed | PopupCase::Lifecycle => count.max(1),
            other => other.depth().max(1),
        };
        let mut probe = Self {
            kind,
            rows: count.max(8),
            flags: vec![false; levels],
            triggers: vec![Rect::default(); levels],
            model: false,
            shown: false,
            scroll: 0.0,
            previous_scroll: 0.0,
            root_at: 20.0,
            step: 0,
            now: Instant::now(),
            revision: 0,
            tessellations: 0,
            consumed: false,
        };
        for _ in 0..3 {
            probe.build(context);
        }
        if !matches!(
            kind,
            PopupCase::Closed | PopupCase::Toggle | PopupCase::Lifecycle
        ) {
            probe.flags.iter_mut().for_each(|flag| *flag = true);
        }
        for _ in 0..4 {
            probe.build(context);
        }
        probe
    }

    pub fn input(&mut self, context: &mut Context, step: usize) {
        self.step = step;
        self.now += Duration::from_millis(20);
        self.revision = context.draw_data().revision;
        self.tessellations = context.cache_stats().tessellated_elements;
        self.previous_scroll = self.scroll;
        let all = |flags: &mut Vec<bool>, on: bool| flags.iter_mut().for_each(|f| *f = on);
        match self.kind {
            PopupCase::Toggle => all(&mut self.flags, step.is_multiple_of(2)),
            PopupCase::Keys if step.is_multiple_of(2) => all(&mut self.flags, true),
            PopupCase::Keys => {
                context.on_key_event(KeyCode::Escape, ElementState::Pressed, false);
                context.on_key_event(KeyCode::Escape, ElementState::Released, false);
            }
            PopupCase::Wheel => self.wheel(context, step),
            PopupCase::Moving => self.root_at = 20.0 + (step % 40) as f32 * 5.0,
            PopupCase::Model => self.model = !self.model,
            PopupCase::Lifecycle => {
                // Only the triggers on screen can open; the rest are live, closed controls.
                let open = step % self.flags.len().min(16);
                self.flags
                    .iter_mut()
                    .enumerate()
                    .for_each(|(i, f)| *f = i == open);
            }
            _ => {}
        }
    }

    fn wheel(&mut self, context: &mut Context, step: usize) {
        let scale = f64::from(context.scale_factor());
        let Some(leaf) = context.probe().popups.last().map(|p| p.rect.center()) else {
            return;
        };
        context.on_window_event(&WindowEvent::CursorMoved {
            device_id: DeviceId::dummy(),
            position: PhysicalPosition::new(f64::from(leaf.x) * scale, f64::from(leaf.y) * scale),
        });
        let direction = if step.is_multiple_of(2) { -1.0 } else { 1.0 };
        self.consumed = context
            .on_window_event(&WindowEvent::MouseWheel {
                device_id: DeviceId::dummy(),
                delta: MouseScrollDelta::PixelDelta(PhysicalPosition::new(
                    0.0,
                    direction * 30.0 * scale,
                )),
                phase: TouchPhase::Moved,
            })
            .consumed;
    }

    pub fn build(&mut self, context: &mut Context) {
        self.now += Duration::from_millis(1);
        context.run_at(self.now, |context| {
            Root::new().show(context, |ui| {
                if matches!(self.kind, PopupCase::Closed | PopupCase::Lifecycle) {
                    self.roots(ui);
                } else {
                    let at = Rect::from_min_size(vec2(self.root_at, 20.0), vec2(90.0, 28.0));
                    ui.at("root", at, |ui| self.level(ui, 0));
                }
            });
        });
        self.shown = self.flags.iter().any(|f| *f);
    }

    /// Many independent roots, as a toolbar or a table of rows would have.
    fn roots(&mut self, ui: &mut Ui<'_>) {
        for i in 0..self.flags.len() {
            let trigger = ui.push_id(i, |ui| ui.button("Open"));
            self.triggers[i] = trigger.rect;
            let mut open = self.flags[i];
            Popup::new(("root", i), trigger.rect)
                .size(vec2(120.0, 60.0))
                .show(ui, &mut open, |ui| {
                    ui.label("Root");
                });
            self.flags[i] = open;
        }
    }

    /// Level `n` of the chain: its trigger, and the popup of the next level inside it.
    fn level(&mut self, ui: &mut Ui<'_>, n: usize) {
        if n >= self.flags.len() {
            return;
        }
        let trigger = ui.button(format!("Level {n}"));
        self.triggers[n] = trigger.rect;
        let mut open = self.flags[n];
        Popup::new(("level", n), trigger.rect)
            .size(vec2(180.0, 120.0))
            .show(ui, &mut open, |ui| {
                if n + 1 == self.flags.len() {
                    let area = ScrollArea::vertical()
                        .id_source("leaf")
                        .max_height(80.0)
                        .show_rows(ui, 20.0, self.rows, |ui, row| {
                            ui.label(format!("Row {row}"));
                        });
                    self.scroll = area.offset.y;
                    ui.label(if self.model { "On" } else { "Off" });
                } else {
                    self.level(ui, n + 1);
                }
            });
        self.flags[n] = open;
    }

    pub fn verify(&self, context: &Context) {
        let probe = context.probe();
        let popups = probe.popups;
        let viewport = context.viewport();
        let live = probe.popup_layers.len();
        assert!(live <= MAX_POPUP_DEPTH + 1, "layers stay bounded: {live}");
        assert!(popups.windows(2).all(|w| w[1].parent == Some(w[0].id)));
        assert!(
            popups.iter().all(|p| on_screen(viewport, p.rect)),
            "every level fits"
        );
        match self.kind {
            PopupCase::Closed => {
                assert!(popups.is_empty() && live == 0);
                assert!(!context.needs_repaint() && !context.wants_animation_frame());
            }
            PopupCase::Toggle => {
                let expected = if self.step.is_multiple_of(2) {
                    self.kind.depth()
                } else {
                    0
                };
                assert_eq!(popups.len(), expected);
            }
            PopupCase::Keys => {
                let expected = self.kind.depth() - usize::from(!self.step.is_multiple_of(2));
                assert_eq!(popups.len(), expected);
            }
            PopupCase::Wheel => {
                assert_eq!(popups.len(), self.kind.depth());
                assert!(self.consumed, "the wheel belongs to the branch");
                assert!(self.scroll != self.previous_scroll || self.step < 2);
            }
            PopupCase::Moving => {
                assert_eq!(popups.len(), self.kind.depth());
                assert!((popups[0].anchor.min.x - self.root_at).abs() < 1.0);
                assert!(popups
                    .windows(2)
                    .all(|w| !w[1].anchor.intersect(w[0].rect).is_empty()));
            }
            PopupCase::Model => {
                assert_eq!(popups.len(), self.kind.depth());
            }
            PopupCase::Lifecycle => {
                assert_eq!(popups.len(), 1);
                assert!(live <= 2);
            }
            PopupCase::Single | PopupCase::Chain2 | PopupCase::Chain4 | PopupCase::ChainMax => {
                assert_eq!(popups.len(), self.kind.depth());
                assert!(!context.needs_repaint() && !context.wants_animation_frame());
                assert_eq!(self.revision, context.draw_data().revision);
                assert_eq!(
                    self.tessellations,
                    context.cache_stats().tessellated_elements
                );
            }
        }
        let _ = self.shown;
    }
}

fn on_screen(viewport: Rect, rect: Rect) -> bool {
    rect.min.x >= viewport.min.x - 0.5
        && rect.min.y >= viewport.min.y - 0.5
        && rect.max.x <= viewport.max.x + 0.5
        && rect.max.y <= viewport.max.y + 0.5
}
