//! Public Dock API, deterministic frame clock, assertions outside timed builds.
use std::time::Duration;
use zaxis::winit::event::{ElementState, MouseButton};
use zaxis::{
    testing::Inspect, vec2, Button, Context, Dock, DockChild, DockEvent, DockNode, DockOutput,
    DockState, DockStyle, DockViewer, Id, InputEvent, Layout, Padding, PanelId, Rect, Root, Theme,
    Ui,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockCase {
    Idle,
    Drag,
    Split,
    Ring,
    Theme,
    Lifecycle,
}
impl DockCase {
    pub fn name(self) -> &'static str {
        match self {
            Self::Idle => "dock_idle",
            Self::Drag => "dock_drag_zones",
            Self::Split => "dock_split_animation",
            Self::Ring => "dock_focus_ring",
            Self::Theme => "dock_theme",
            Self::Lifecycle => "dock_lifecycle",
        }
    }
    pub fn interactive(self) -> bool {
        self != Self::Idle
    }
}
fn panel(n: usize) -> PanelId {
    Id::new(("bench-panel", n))
}
fn tree(first: usize, count: usize, depth: usize) -> DockNode {
    if count == 1 {
        return DockNode::tabs(("group", first), [panel(first)]);
    }
    let half = count / 2;
    DockNode::split(
        (first, count),
        if depth.is_multiple_of(2) {
            Layout::Horizontal
        } else {
            Layout::Vertical
        },
        [
            DockChild::new(tree(first, half, depth + 1), half as f32 / count as f32),
            DockChild::new(
                tree(first + half, count - half, depth + 1),
                (count - half) as f32 / count as f32,
            ),
        ],
    )
}
struct Viewer<'a> {
    built: &'a mut usize,
    buttons: &'a mut Vec<(PanelId, Rect)>,
}
impl DockViewer for Viewer<'_> {
    fn title(&self, p: &PanelId) -> String {
        format!("View {:x}", p.value() & 255)
    }
    fn ui(&mut self, ui: &mut Ui<'_>, p: &PanelId) {
        *self.built += 1;
        let response = ui.add(Button::new("Edit"));
        self.buttons.push((*p, response.rect));
        ui.label("Panel content");
    }
}
pub struct Probe {
    kind: DockCase,
    state: DockState,
    out: DockOutput,
    count: usize,
    now: zaxis::Instant,
    built: usize,
    buttons: Vec<(PanelId, Rect)>,
    before_tessellations: u64,
    step: usize,
    drag_moves: usize,
}
impl Probe {
    pub fn new(context: &mut Context, kind: DockCase, count: usize) -> Self {
        let count = count.clamp(4, 64);
        context.on_input(InputEvent::Focus(true));
        let mut state = DockState::new(tree(0, count, 0));
        if kind == DockCase::Split {
            state.move_panel(panel(1), panel(0), None).unwrap();
        }
        let mut probe = Self {
            kind,
            state,
            out: DockOutput::default(),
            count,
            now: zaxis::Instant::now(),
            built: 0,
            buttons: Vec::new(),
            before_tessellations: 0,
            step: 0,
            drag_moves: 0,
        };
        for _ in 0..3 {
            probe.build(context);
        }
        probe.now += Duration::from_secs(3);
        probe.build(context);
        probe
    }
    pub fn input(&mut self, context: &mut Context, step: usize) {
        self.step = step;
        self.now += Duration::from_millis(16);
        self.before_tessellations = context.cache_stats().tessellated_elements;
        match self.kind {
            DockCase::Idle => {}
            DockCase::Theme => context.set_theme(if step.is_multiple_of(2) {
                Theme::light()
            } else {
                Theme::dark()
            }),
            DockCase::Ring => {
                let p = panel(step % 2);
                if let Some((_, rect)) = self.buttons.iter().find(|(id, _)| *id == p) {
                    pointer(context, rect.center());
                    button(context, ElementState::Pressed);
                    button(context, ElementState::Released);
                }
            }
            DockCase::Split => {
                if step.is_multiple_of(24) {
                    self.state
                        .split(panel(1), panel(0), zaxis::DockSide::Right, 0.5)
                        .unwrap();
                } else if step % 24 == 12 {
                    self.state.move_panel(panel(1), panel(0), None).unwrap();
                }
            }
            DockCase::Lifecycle => {
                if step.is_multiple_of(2) {
                    self.state.close(panel(1)).unwrap();
                } else {
                    self.state.open(panel(1), Some(panel(0))).unwrap();
                }
            }
            DockCase::Drag => self.drag(context, step),
        }
    }
    fn drag(&mut self, c: &mut Context, step: usize) {
        if step.is_multiple_of(8) {
            let key = Id::new(panel(0));
            let source = c
                .probe()
                .drag
                .last_sources
                .iter()
                .find(|s| s.key == key)
                .map(|s| s.id)
                .expect("Dock tab source");
            let rect = c
                .probe()
                .previous_hits
                .iter()
                .find(|h| h.id == source)
                .expect("source hit")
                .rect;
            pointer(c, rect.min + vec2(12.0, 16.0));
            button(c, ElementState::Pressed);
            pointer(c, rect.min + vec2(12.0, 34.0));
        } else if step % 8 == 7 {
            button(c, ElementState::Released);
        } else if let Some(target) = self.out.panels.iter().find(|p| p.panel != panel(0)) {
            let point = if step % 8 < 4 {
                target.content_bounds.min + vec2(2.0, 20.0)
            } else {
                target.content_bounds.center()
            };
            pointer(c, point);
        }
    }
    pub fn build(&mut self, c: &mut Context) {
        self.built = 0;
        self.buttons.clear();
        c.run_at(self.now, |c| {
            Root::new().padding(Padding::all(8.0)).show(c, |ui| {
                self.out = Dock::new("bench-dock", &mut self.state)
                    .style(DockStyle {
                        minimum: Some(vec2(40.0, 40.0)),
                        ..Default::default()
                    })
                    .show(
                        ui,
                        Viewer {
                            built: &mut self.built,
                            buttons: &mut self.buttons,
                        },
                    );
            })
        });
        self.drag_moves += self
            .out
            .events
            .iter()
            .filter(|e| matches!(e, DockEvent::Moved { .. } | DockEvent::Split { .. }))
            .count();
    }
    pub fn verify(&self, c: &Context) {
        assert!(self.state.validate((0..self.count).map(panel)).is_empty());
        assert_eq!(self.built, self.out.panels.len());
        assert!(self.out.panels.iter().all(|p| p.bounds.is_finite()
            && p.target_bounds.is_finite()
            && p.target_bounds.max.x <= c.viewport().max.x
            && p.target_bounds.max.y <= c.viewport().max.y));
        assert!(self.out.focus_ring.is_some());
        assert!(c.probe().counts.docks <= 1);
        assert!(c.draw_data().textures.len() <= 4);
        if self.kind == DockCase::Idle {
            assert_eq!(self.state.panels().len(), self.count);
            assert_eq!(self.out.panels.len(), self.count);
            assert_eq!(
                c.cache_stats().tessellated_elements,
                self.before_tessellations
            );
            assert!(!c.wants_animation_frame());
            assert!(c.next_repaint().is_none());
        }
        if self.kind == DockCase::Drag && self.step >= 8 {
            assert!(self.drag_moves > 0, "drag must change the model");
        }
    }
}

fn pointer(c: &mut Context, p: zaxis::Vec2) {
    let p = p * c.scale_factor();
    c.on_input(InputEvent::PointerMoved {
        x: p.x as f64,
        y: p.y as f64,
    });
}
fn button(c: &mut Context, state: ElementState) {
    c.on_input(InputEvent::Button {
        button: MouseButton::Left,
        state,
    });
}
