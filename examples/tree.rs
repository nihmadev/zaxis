#[path = "tree/model.rs"]
mod model;
use model::{id, Objects};
use std::{
    collections::HashSet,
    time::{Duration, Instant},
};
use zaxis::{
    App, Button, Context, Frame, Id, Root, SplitPane, SplitPanel, SplitSize, Theme, TreeEvent,
    TreeView,
};

struct Browser {
    objects: Objects,
    open: HashSet<Id>,
    selected: Option<Id>,
    reveal: Option<Id>,
    name: String,
    editing: Option<Id>,
    activated: Option<Id>,
    theme: usize,
    follows: bool,
    reduced: bool,
    loading: Option<Instant>,
    smoke: bool,
    passes: usize,
}
impl App for Browser {
    fn update(&mut self, c: &mut Context, frame: &mut Frame<'_>) {
        if self.loading.is_some_and(|t| c.frame_time() >= t) {
            self.objects.finish_loading();
            self.loading = None;
        }
        if let Some(t) = self.loading {
            c.request_repaint_after(t.saturating_duration_since(c.frame_time()));
        }
        let mut theme = match self.theme {
            1 => Theme::light(),
            2 => Theme::high_contrast(),
            _ => Theme::dark(),
        };
        let mut motion = c.style().motion.clone();
        motion.reduced_motion = self.reduced;
        theme.overrides.motion = Some(motion);
        c.set_theme(theme);
        Root::new().show(c, |ui| {
            ui.horizontal(|ui| {
                for (n, label) in ["Dark", "Light", "High contrast"].into_iter().enumerate() {
                    if ui
                        .add(Button::new(label).selected(self.theme == n))
                        .clicked()
                    {
                        self.theme = n;
                    }
                }
                ui.checkbox(&mut self.follows, "Select on focus");
                ui.checkbox(&mut self.reduced, "Reduced motion");
            });
            let size = zaxis::vec2(ui.available_width(), ui.available_height());
            SplitPane::horizontal("objects-split")
                .size(size)
                .panels([
                    SplitPanel::new("tree")
                        .default_size(SplitSize::Weight(2.0))
                        .min_size(120.0),
                    SplitPanel::new("inspector")
                        .default_size(SplitSize::Weight(1.0))
                        .min_size(120.0),
                ])
                .show(ui, |split| {
                    split.panel("tree", |ui| {
                        ui.horizontal(|ui| {
                            if ui.button("Add").clicked() {
                                self.objects.add();
                            }
                            if ui.button("Reverse").clicked() {
                                self.objects
                                    .nodes
                                    .get_mut(&id(1))
                                    .unwrap()
                                    .children
                                    .reverse();
                                self.objects.revision += 1;
                            }
                            if ui.button("Reveal last").clicked() {
                                self.reveal = Some(id(10_999));
                            }
                        });
                        let mut tree = TreeView::new("objects")
                            .open(&mut self.open)
                            .selected(&mut self.selected)
                            .expand_on_double_click(true)
                            .selection_follows_focus(self.follows)
                            .max_height(ui.available_height());
                        if let Some(node) = self.reveal.take() {
                            tree = tree.reveal_node(node);
                        }
                        let mut edit = None;
                        let out = tree.show_with_actions(ui, &self.objects, |ui, node| {
                            if ui.button("Edit").clicked() {
                                edit = Some(node);
                            }
                        });
                        if let Some(n) = edit {
                            self.editing = Some(n);
                            self.name = self.objects.nodes[&n].label.clone();
                        }
                        for event in out.events {
                            match event {
                                TreeEvent::RequestChildren { node } => {
                                    self.objects.load(node);
                                    self.loading = Some(
                                        ui.context().frame_time() + Duration::from_millis(500),
                                    );
                                }
                                TreeEvent::Activated { node } => {
                                    self.activated = Some(node);
                                    self.reveal = Some(node);
                                }
                                TreeEvent::ContextAction { node } => {
                                    self.editing = Some(node);
                                    self.name = self.objects.nodes[&node].label.clone();
                                }
                                _ => {}
                            }
                        }
                        if self.smoke {
                            assert!(out.rows_built < 40);
                        }
                    });
                    split.panel("inspector", |ui| {
                        if let Some(node) =
                            self.editing.filter(|n| self.objects.nodes.contains_key(n))
                        {
                            ui.text_edit(&mut self.name);
                            if ui.button("Save name").clicked() {
                                self.objects
                                    .nodes
                                    .get_mut(&node)
                                    .unwrap()
                                    .label
                                    .clone_from(&self.name);
                                self.objects.revision += 1;
                            }
                        }
                        if let Some(node) =
                            self.selected.filter(|n| self.objects.nodes.contains_key(n))
                        {
                            ui.label(&self.objects.nodes[&node].label);
                            if ui.button("Remove").clicked() {
                                self.objects.remove(node);
                            }
                        }
                        if ui.button("Expand objects").clicked() {
                            self.open.extend([id(0), id(1), id(6)]);
                        }
                        if ui.button("Collapse all").clicked() {
                            self.open.clear();
                        }
                        if let Some(node) = self
                            .activated
                            .filter(|n| self.objects.nodes.contains_key(n))
                        {
                            ui.label(format!("Opened {}", self.objects.nodes[&node].label));
                        }
                    });
                });
        });
        if self.smoke {
            self.passes += 1;
            if self.passes >= 3 {
                frame.close();
            } else {
                c.request_repaint();
            }
        }
    }
}
fn main() -> Result<(), zaxis::RunError> {
    zaxis::run_with_options(
        Browser {
            objects: Objects::new(),
            open: [id(0), id(1), id(100)].into_iter().collect(),
            selected: None,
            reveal: None,
            name: String::new(),
            editing: None,
            activated: None,
            theme: 0,
            follows: false,
            reduced: false,
            loading: None,
            smoke: std::env::args().any(|a| a == "--smoke-test"),
            passes: 0,
        },
        zaxis::RunOptions {
            window_attributes: zaxis::winit::window::Window::default_attributes()
                .with_title("zaxis — Tree")
                .with_inner_size(zaxis::winit::dpi::LogicalSize::new(980.0, 700.0)),
            ..Default::default()
        },
    )
}
