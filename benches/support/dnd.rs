//! Drag-and-drop scenarios for the shared performance harness. Input goes
//! through the same public event dispatcher as a host; every assertion runs in
//! `verify`, outside the timed intervals.
use std::time::Duration;
use zaxis::winit::{
    dpi::PhysicalPosition,
    event::{DeviceId, ElementState, MouseButton, WindowEvent},
    keyboard::KeyCode,
};
use zaxis::Instant;
use zaxis::{
    Context, DragReason, Id, Padding, Rect, Root, ScrollArea, TreeEvent, TreeModel, TreeNode,
    TreeView, Ui, Vec2,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DndCase {
    Idle,
    BeginEnd,
    MoveTargets,
    Autoscroll,
    Tree,
    Preview,
    Lifecycle,
}
impl DndCase {
    pub fn name(self) -> &'static str {
        match self {
            Self::Idle => "dnd_idle",
            Self::BeginEnd => "dnd_begin_end",
            Self::MoveTargets => "dnd_move_targets",
            Self::Autoscroll => "dnd_autoscroll_virtual",
            Self::Tree => "dnd_tree",
            Self::Preview => "dnd_preview",
            Self::Lifecycle => "dnd_lifecycle",
        }
    }
    pub fn interactive(self) -> bool {
        self != Self::Idle
    }
}

struct Chain {
    ids: Vec<Id>,
}
impl TreeModel for Chain {
    fn revision(&self) -> u64 {
        0
    }
    fn roots(&self) -> impl Iterator<Item = Id> {
        self.ids.iter().take(1).copied()
    }
    fn children(&self, id: Id) -> impl Iterator<Item = Id> {
        let all = if id == self.ids[0] {
            &self.ids[1..]
        } else {
            &[][..]
        };
        all.iter().copied()
    }
    fn node(&self, id: Id) -> Option<TreeNode<'_>> {
        self.ids.iter().position(|x| *x == id).map(|i| {
            if i == 0 {
                TreeNode::branch("Root")
            } else {
                TreeNode::leaf("Node")
            }
        })
    }
    fn parent(&self, id: Id) -> Option<Id> {
        (id != self.ids[0]).then_some(self.ids[0])
    }
}

pub struct Probe {
    kind: DndCase,
    count: usize,
    step: usize,
    now: Instant,
    chain: Chain,
    open: std::collections::HashSet<Id>,
    rows: Vec<(usize, Rect)>,
    viewport: Rect,
    offset: f32,
    built: usize,
    hovered: Option<usize>,
    begins: usize,
    drops: usize,
    ends: Vec<DragReason>,
    moves: usize,
    show_source: bool,
    last_offset: f32,
    revision: u64,
    tessellations: u64,
    full_rebuilds: u64,
}

impl Probe {
    pub fn new(kind: DndCase, count: usize) -> Self {
        let count = count.max(12);
        let ids: Vec<Id> = (0..=count).map(Id::new).collect();
        Self {
            kind,
            count,
            step: 0,
            now: Instant::now(),
            open: [ids[0]].into_iter().collect(),
            chain: Chain { ids },
            rows: Vec::new(),
            viewport: Rect::default(),
            offset: 0.0,
            built: 0,
            hovered: None,
            begins: 0,
            drops: 0,
            ends: Vec::new(),
            moves: 0,
            show_source: true,
            last_offset: 0.0,
            revision: 0,
            tessellations: 0,
            full_rebuilds: 0,
        }
    }

    fn pointer(c: &mut Context, p: Vec2) {
        let s = c.scale_factor();
        c.on_window_event(&WindowEvent::CursorMoved {
            device_id: DeviceId::dummy(),
            position: PhysicalPosition::new(f64::from(p.x * s), f64::from(p.y * s)),
        });
    }
    fn button(c: &mut Context, state: ElementState) {
        c.on_window_event(&WindowEvent::MouseInput {
            device_id: DeviceId::dummy(),
            state,
            button: MouseButton::Left,
        });
    }
    fn row(&self, n: usize) -> Rect {
        if self.kind == DndCase::Tree {
            let h = 24.0;
            let y = self.viewport.min.y + n as f32 * h;
            return Rect::from_min_size(
                Vec2::new(self.viewport.min.x + 40.0, y),
                Vec2::new(self.viewport.size().x - 60.0, h),
            );
        }
        self.rows
            .iter()
            .find(|r| r.0 == n)
            .map_or_else(|| self.rows[0].1, |r| r.1)
    }
    /// Built rows well inside the scroll viewport.
    fn visible(&self) -> Vec<(usize, Rect)> {
        self.rows
            .iter()
            .copied()
            .filter(|(_, r)| {
                // Stay out of the autoscroll bands at both edges.
                r.min.y >= self.viewport.min.y + 40.0 && r.max.y <= self.viewport.max.y - 40.0
            })
            .collect()
    }
    fn begin(&mut self, c: &mut Context, n: usize) {
        let at = self.row(n).center();
        Self::pointer(c, at);
        Self::button(c, ElementState::Pressed);
        Self::pointer(c, at + Vec2::new(0.0, 12.0));
    }

    pub fn input(&mut self, c: &mut Context, step: usize) {
        self.step = step;
        self.now += Duration::from_millis(16);
        self.revision = c.draw_data().revision;
        self.tessellations = c.cache_stats().tessellated_elements;
        self.full_rebuilds = c.cache_stats().geometry_full_rebuilds;
        let phase = step % 4;
        match self.kind {
            DndCase::Idle => {}
            DndCase::BeginEnd => match phase {
                0 => self.begin(c, 0),
                2 => Self::pointer(c, self.row(2).center()),
                3 => Self::button(c, ElementState::Released),
                _ => {}
            },
            DndCase::Tree => match phase {
                0 => self.begin(c, 3),
                2 => {
                    let r = self.row(6);
                    Self::pointer(c, Vec2::new(r.center().x, r.min.y + r.size().y * 0.9));
                }
                3 => Self::button(c, ElementState::Released),
                _ => {}
            },
            DndCase::MoveTargets | DndCase::Preview => {
                if step == 0 {
                    if self.kind == DndCase::Preview {
                        // Measure preview movement alone, without the indicator fade.
                        let mut style = c.style().clone();
                        style.motion.reduced_motion = true;
                        c.set_style(style);
                    }
                    self.begin(c, 0);
                } else if self.visible().len() > 2 {
                    let visible = self.visible();
                    let pick = if self.kind == DndCase::Preview {
                        2
                    } else {
                        (step * 7) % visible.len()
                    };
                    let r = visible[pick].1;
                    let jitter = Vec2::new((step % 5) as f32, 0.0);
                    Self::pointer(c, r.center() + jitter);
                }
            }
            DndCase::Autoscroll => {
                if step == 0 {
                    self.begin(c, 0);
                } else if step == 1 {
                    let p = Vec2::new(self.viewport.min.x + 30.0, self.viewport.max.y - 3.0);
                    Self::pointer(c, p);
                }
            }
            DndCase::Lifecycle => match phase {
                0 => {
                    self.show_source = true;
                    self.begin(c, 0);
                }
                2 => match (step / 4) % 3 {
                    0 => {
                        c.on_key_event(KeyCode::Escape, ElementState::Pressed, false);
                    }
                    1 => {
                        c.on_window_event(&WindowEvent::Focused(false));
                        c.on_window_event(&WindowEvent::Focused(true));
                    }
                    _ => self.show_source = false,
                },
                3 => {
                    // The user lets go after the drag was already cancelled.
                    self.show_source = true;
                    Self::button(c, ElementState::Released);
                }
                _ => {}
            },
        }
    }

    pub fn build(&mut self, c: &mut Context) {
        let (n, kind) = (self.count, self.kind);
        let (show_source, now) = (self.show_source, self.now);
        let open = &mut self.open;
        let chain = &self.chain;
        let (mut rows, mut hovered, mut built) = (Vec::new(), None, 0);
        let (mut begins, mut drops, mut ends, mut moves) = (0, 0, Vec::new(), 0);
        let (mut viewport, mut offset) = (self.viewport, self.offset);
        c.run_at(now, |c| {
            Root::new().padding(Padding::all(0.0)).show(c, |ui| {
                if kind == DndCase::Tree {
                    let out = TreeView::new("t")
                        .open(open)
                        .drag_nodes(true)
                        .max_height(300.0)
                        .show(ui, chain);
                    built = out.rows_built;
                    viewport = out.viewport;
                    moves += out
                        .events
                        .iter()
                        .filter(|e| matches!(e, TreeEvent::Moved { .. }))
                        .count();
                    return;
                }
                if kind == DndCase::Autoscroll {
                    if let Some(dragged) = ui.dragging() {
                        ui.keep_drag_source(dragged);
                    }
                }
                let mut row = |ui: &mut Ui<'_>, i: usize| {
                    built += 1;
                    let id = Id::new(i);
                    let target = ui.drop_target(
                        id,
                        |_: &usize| true,
                        |ui| {
                            if i == 0 && !show_source {
                                ui.label("Row");
                                return None;
                            }
                            let source = ui.drag_source(id, i, |ui| {
                                ui.label("Row");
                            });
                            begins += usize::from(source.started);
                            ends.extend(source.finished.map(|e| e.reason));
                            Some(())
                        },
                    );
                    rows.push((i, target.response.rect));
                    if target.hovering {
                        hovered = Some(i);
                    }
                    drops += usize::from(target.dropped.is_some());
                };
                let area = ScrollArea::vertical().id_source("rows").max_height(300.0);
                if kind == DndCase::Autoscroll {
                    let out = area.show_rows(ui, 24.0, n, |ui, i| row(ui, i));
                    (viewport, offset) = (out.viewport, out.offset.y);
                } else {
                    let out = area.show(ui, |ui| {
                        for i in 0..n {
                            row(ui, i);
                        }
                    });
                    (viewport, offset) = (out.viewport, out.offset.y);
                }
            });
        });
        self.last_offset = self.offset;
        (self.rows, self.hovered, self.built) = (rows, hovered, built);
        (self.viewport, self.offset) = (viewport, offset);
        self.begins += begins;
        self.drops += drops;
        self.ends.extend(ends);
        self.moves += moves;
    }

    pub fn verify(&self, c: &Context) {
        let (cycle, phase) = (self.step / 4, self.step % 4);
        match self.kind {
            DndCase::Idle => {
                assert!(!c.needs_repaint() && !c.wants_animation_frame());
                assert_eq!(c.draw_data().revision, self.revision);
                assert_eq!(c.cache_stats().tessellated_elements, self.tessellations);
                assert!(c.dragging().is_none());
            }
            DndCase::BeginEnd => match phase {
                0 => {
                    assert_eq!(self.begins, cycle + 1);
                    assert_eq!(self.ends.len(), cycle, "previous drop reported once");
                    assert_eq!(c.dragging(), Some(Id::new(0usize)));
                }
                2 => assert_eq!(self.hovered, Some(2)),
                3 => {
                    assert_eq!(self.drops, cycle + 1, "exactly one drop per cycle");
                    assert!(c.dragging().is_none());
                }
                _ => {}
            },
            DndCase::MoveTargets => {
                assert_eq!(self.begins, 1);
                assert!(c.dragging().is_some());
                let visible = self.visible();
                if self.step >= 3 && !visible.is_empty() {
                    let expected = visible[(self.step * 7) % visible.len()].0;
                    assert_eq!(self.hovered, Some(expected));
                }
                assert_eq!(self.drops, 0);
            }
            DndCase::Autoscroll => {
                assert!(c.dragging().is_some(), "source alive while unbuilt");
                assert!(self.built <= 16, "virtualized: {} rows built", self.built);
                assert!(self.offset >= self.last_offset);
                if self.step >= 3 && self.offset < (self.count as f32 * 24.0 - 300.0) {
                    assert!(self.offset > 0.0, "autoscroll moves the list");
                }
            }
            DndCase::Tree => match phase {
                0 => assert!(c.dragging().is_some()),
                3 => {
                    assert_eq!(self.moves, cycle + 1);
                    assert!(self.built <= 16, "{} rows built", self.built);
                }
                _ => {}
            },
            DndCase::Preview => {
                assert!(c.dragging().is_some());
                if self.step >= 4 {
                    assert_eq!(
                        c.cache_stats().tessellated_elements,
                        self.tessellations,
                        "moving the preview never re-tessellates"
                    );
                    assert_eq!(c.cache_stats().geometry_full_rebuilds, self.full_rebuilds);
                    assert!(c.draw_data().revision > self.revision);
                }
            }
            DndCase::Lifecycle => {
                if phase == 3 {
                    assert!(c.dragging().is_none(), "no hung session");
                    let expected = match cycle % 3 {
                        0 => DragReason::Escape,
                        1 => DragReason::FocusLost,
                        _ => DragReason::SourceLost,
                    };
                    assert_eq!(self.ends.last(), Some(&expected));
                    assert_eq!(self.begins, cycle + 1);
                }
            }
        }
    }
}
