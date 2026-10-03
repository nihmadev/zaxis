use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};
use zaxis::winit::{event::ElementState, keyboard::KeyCode};
use zaxis::{
    CollapsingHeader, Context, Id, Root, ScrollArea, TreeEvent, TreeModel, TreeNode, TreeOutput,
    TreeView,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisclosureCase {
    TreeCached,
    TreeKeys,
    TreeReveal,
    TreeRevision,
    TreeToggle,
    TreeLifecycle,
    CollapsingCached,
    CollapsingToggle,
}
impl DisclosureCase {
    pub fn name(self) -> &'static str {
        match self {
            Self::TreeCached => "tree_cached",
            Self::TreeKeys => "tree_keyboard",
            Self::TreeReveal => "tree_reveal",
            Self::TreeRevision => "tree_revision",
            Self::TreeToggle => "tree_toggle",
            Self::TreeLifecycle => "tree_lifecycle",
            Self::CollapsingCached => "collapsing_cached",
            Self::CollapsingToggle => "collapsing_toggle",
        }
    }
    pub fn interactive(self) -> bool {
        matches!(
            self,
            Self::TreeKeys | Self::TreeReveal | Self::TreeToggle | Self::CollapsingToggle
        )
    }
}
struct Model {
    ids: Vec<Id>,
    lookup: HashMap<Id, usize>,
    revision: u64,
}
impl TreeModel for Model {
    fn revision(&self) -> u64 {
        self.revision
    }
    fn roots(&self) -> impl Iterator<Item = Id> {
        self.ids.iter().take(1).copied()
    }
    fn children(&self, id: Id) -> impl Iterator<Item = Id> {
        self.ids
            .iter()
            .skip(1)
            .take(if id == self.ids[0] { self.ids.len() } else { 0 })
            .copied()
    }
    fn node(&self, id: Id) -> Option<TreeNode<'_>> {
        self.lookup.get(&id).map(|i| {
            if *i == 0 {
                TreeNode::branch("Objects")
            } else {
                TreeNode::leaf("Object")
            }
        })
    }
    fn parent(&self, id: Id) -> Option<Id> {
        (id != self.ids[0] && self.lookup.contains_key(&id)).then_some(self.ids[0])
    }
}
pub struct Probe {
    kind: DisclosureCase,
    model: Model,
    open: HashSet<Id>,
    selected: Option<Id>,
    output: Option<TreeOutput>,
    step: usize,
    reveal: Option<Id>,
    header_open: bool,
    header_runs: usize,
    rows: usize,
    now: Instant,
    revision: u64,
    tessellations: u64,
}
impl Probe {
    pub fn new(kind: DisclosureCase, count: usize) -> Self {
        let ids: Vec<_> = (0..=count.max(1)).map(Id::new).collect();
        let lookup = ids.iter().enumerate().map(|(i, id)| (*id, i)).collect();
        Self {
            kind,
            open: [ids[0]].into_iter().collect(),
            model: Model {
                ids,
                lookup,
                revision: 0,
            },
            selected: None,
            output: None,
            step: 0,
            reveal: None,
            header_open: true,
            header_runs: 0,
            rows: 0,
            now: Instant::now(),
            revision: 0,
            tessellations: 0,
        }
    }
    pub fn input(&mut self, c: &mut Context, step: usize) {
        self.step = step;
        self.now += Duration::from_millis(20);
        self.revision = c.draw_data().revision;
        self.tessellations = c.cache_stats().tessellated_elements;
        let root = self.model.ids[0];
        match self.kind {
            DisclosureCase::TreeKeys => {
                c.request_focus(self.output.as_ref().unwrap().id);
                let key = if step.is_multiple_of(2) {
                    KeyCode::End
                } else {
                    KeyCode::Home
                };
                assert!(c.on_key_event(key, ElementState::Pressed, false).consumed);
                c.on_key_event(key, ElementState::Released, false);
            }
            DisclosureCase::TreeReveal => {
                self.reveal = Some(if step.is_multiple_of(2) {
                    *self.model.ids.last().unwrap()
                } else {
                    root
                })
            }
            DisclosureCase::TreeRevision => {
                self.model.ids[1..].rotate_left(1);
                self.model.revision += 1;
            }
            DisclosureCase::TreeToggle => {
                c.request_focus(self.output.as_ref().unwrap().id);
                c.on_key_event(KeyCode::Home, ElementState::Pressed, false);
                let key = if self.open.contains(&root) {
                    KeyCode::ArrowLeft
                } else {
                    KeyCode::ArrowRight
                };
                c.on_key_event(key, ElementState::Pressed, false);
                c.on_key_event(key, ElementState::Released, false);
            }
            DisclosureCase::CollapsingToggle => self.header_open = !self.header_open,
            _ => {}
        }
    }
    pub fn build(&mut self, c: &mut Context) {
        self.rows = 0;
        self.header_runs = 0;
        if self.kind == DisclosureCase::TreeLifecycle && self.step % 2 == 1 {
            c.run_at(self.now, |_| {});
            self.output = None;
            return;
        }
        c.run_at(self.now, |c| {
            Root::new().show(c, |ui| {
                if matches!(
                    self.kind,
                    DisclosureCase::CollapsingCached | DisclosureCase::CollapsingToggle
                ) {
                    CollapsingHeader::new("header", "Settings")
                        .open(&mut self.header_open)
                        .animate_height(false)
                        .show(ui, |ui| {
                            self.header_runs += 1;
                            ScrollArea::vertical()
                                .id_source("items")
                                .max_height(200.0)
                                .show_rows(ui, 24.0, self.model.ids.len(), |ui, n| {
                                    self.rows += 1;
                                    ui.label(format!("Item {n}"));
                                });
                        });
                } else {
                    let mut view = TreeView::new("tree")
                        .open(&mut self.open)
                        .selected(&mut self.selected)
                        .max_height(200.0);
                    if let Some(node) = self.reveal.take() {
                        view = view.reveal_node(node);
                    }
                    self.output = Some(view.show_with_actions(ui, &self.model, |_, _| {
                        self.rows += 1;
                    }));
                }
            });
        });
    }
    pub fn verify(&self, c: &Context) {
        if self.kind == DisclosureCase::TreeLifecycle && self.output.is_none() {
            assert!(!c.wants_animation_frame());
            return;
        }
        if let Some(out) = &self.output {
            assert!(out.issues.is_empty());
            assert_eq!(out.rows_built, self.rows);
            assert!(out.rows_built <= 10);
            assert_eq!(
                out.logical_rows,
                if self.open.contains(&self.model.ids[0]) {
                    self.model.ids.len()
                } else {
                    1
                }
            );
            match self.kind {
                DisclosureCase::TreeCached => {
                    assert!(!out.rebuilt);
                    assert!(!c.needs_repaint());
                    assert_eq!(self.tessellations, c.cache_stats().tessellated_elements);
                    assert_eq!(self.revision, c.draw_data().revision);
                }
                DisclosureCase::TreeRevision => assert!(out.rebuilt),
                DisclosureCase::TreeKeys | DisclosureCase::TreeReveal => {
                    assert_eq!(
                        out.focused,
                        Some(if self.step.is_multiple_of(2) {
                            *self.model.ids.last().unwrap()
                        } else {
                            self.model.ids[0]
                        })
                    );
                    if self.step.is_multiple_of(2) && self.model.ids.len() > 10 {
                        assert!(out.scroll_offset.y > 0.0);
                    }
                }
                DisclosureCase::TreeToggle => assert_eq!(
                    out.events
                        .iter()
                        .filter(|e| matches!(e, TreeEvent::OpenChanged { .. }))
                        .count(),
                    1
                ),
                _ => {}
            }
        } else {
            assert_eq!(self.header_runs, usize::from(self.header_open));
            assert!(self.rows <= 10);
            if self.kind == DisclosureCase::CollapsingCached {
                assert!(!c.needs_repaint());
            }
        }
    }
}
