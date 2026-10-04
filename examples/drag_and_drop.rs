#[path = "drag_and_drop/model.rs"]
mod model;
#[path = "drag_and_drop/panels.rs"]
mod panels;
use model::{id, Data};
use std::collections::HashSet;
use zaxis::{
    winit::{
        dpi::PhysicalPosition,
        event::{DeviceId, ElementState, MouseButton, WindowEvent},
    },
    App, Context, Frame, Id, Rect, Root, ScrollArea, SplitPanel, SplitSize, TreeEvent, Vec2,
};

struct Demo {
    data: Data,
    open: HashSet<Id>,
    selected: Option<Id>,
    reduced: bool,
    smoke: bool,
    /// `--hold=list|tree` replays a drag and stops mid-way, for visual inspection.
    hold: Option<String>,
    pass: u32,
    tree: Rect,
    first: Rect,
    done: Rect,
}

impl App for Demo {
    fn update(&mut self, c: &mut Context, frame: &mut Frame<'_>) {
        // Scripted input is delivered before the UI is built, like a real event
        // arriving between two redraws.
        if self.smoke || self.hold.is_some() {
            self.script(c, frame);
        }
        let mut style = c.style().clone();
        style.motion.reduced_motion = self.reduced;
        c.set_style(style);
        Root::new().show(c, |ui| {
            ui.checkbox(&mut self.reduced, "Reduced motion");
            let panels = [
                SplitPanel::new("lists")
                    .default_size(SplitSize::Weight(1.0))
                    .min_size(200.0),
                SplitPanel::new("scene")
                    .default_size(SplitSize::Weight(1.0))
                    .min_size(200.0),
                SplitPanel::new("table")
                    .default_size(SplitSize::Weight(1.0))
                    .min_size(220.0),
            ];
            ui.split_horizontal("panels", panels, |split| {
                split.panel("lists", |ui| {
                    ScrollArea::vertical()
                        .id_source("lists-panel")
                        .max_height(ui.available_height())
                        .show(ui, |ui| {
                            ui.label("To do");
                            let todo = panels::task_list(ui, "todo", &self.data.todo);
                            ui.label("Done");
                            let done = panels::task_list(ui, "done", &self.data.done);
                            self.first = todo.first.unwrap_or(self.first);
                            self.done = done.rect;
                            for m in todo.moves.into_iter().chain(done.moves) {
                                self.data.apply(m);
                            }
                            panels::archive(ui, &mut self.data.archive);
                        });
                });
                split.panel("scene", |ui| {
                    let (events, viewport) =
                        panels::tree(ui, &self.data.objects, &mut self.open, &mut self.selected);
                    self.tree = viewport;
                    for event in events {
                        if let TreeEvent::Moved {
                            node,
                            target,
                            position,
                        } = event
                        {
                            self.data.objects.move_node(node, target, position);
                        }
                    }
                });
                split.panel("table", |ui| panels::table(ui, &mut self.data));
            });
        });
    }
}

impl Demo {
    /// Drive one real drag through the event path: press on the first task,
    /// travel past the threshold, release over the "Done" list.
    fn script(&mut self, c: &mut Context, frame: &mut Frame<'_>) {
        self.pass += 1;
        let scale = c.scale_factor();
        let send = |c: &mut Context, p: Vec2, state: Option<ElementState>| {
            let device_id = DeviceId::dummy();
            let position = PhysicalPosition::new(f64::from(p.x * scale), f64::from(p.y * scale));
            c.on_window_event(&WindowEvent::CursorMoved {
                device_id,
                position,
            });
            if let Some(state) = state {
                c.on_window_event(&WindowEvent::MouseInput {
                    device_id,
                    state,
                    button: MouseButton::Left,
                });
            }
        };
        let (start, end) = match self.hold.as_deref() {
            Some("tree") => {
                let x = self.tree.min.x + 70.0;
                (
                    Vec2::new(x, self.tree.min.y + 5.0 * 24.0 + 12.0),
                    Vec2::new(x, self.tree.min.y + 10.0 * 24.0 + 21.0),
                )
            }
            Some(_) => (
                self.first.center(),
                self.first.center() + Vec2::new(0.0, 2.0 * 57.5 + 12.0),
            ),
            None => (self.first.center(), self.done.center()),
        };
        if self.hold.is_some() && self.pass > 8 {
            return;
        }
        match self.pass {
            3 => send(c, start, Some(ElementState::Pressed)),
            4 => send(c, start + Vec2::new(0.0, 14.0), None),
            7 => send(c, end, None),
            9 if self.hold.is_none() => send(c, end, Some(ElementState::Released)),
            14 => {
                assert_eq!(self.data.done.len(), 1, "task moved to the Done list");
                assert_eq!(self.data.todo.len(), 23);
                assert!(c.dragging().is_none());
                println!("drag_and_drop smoke: one task moved by a real drag");
                frame.close();
            }
            _ => {}
        }
        c.request_repaint();
    }
}

fn main() -> Result<(), zaxis::RunError> {
    zaxis::run_with_options(
        Demo {
            data: Data::new(),
            open: [id(1), id(12), id(13), id(2)].into_iter().collect(),
            selected: None,
            reduced: false,
            smoke: std::env::args().any(|a| a == "--smoke-test"),
            hold: std::env::args().find_map(|a| a.strip_prefix("--hold=").map(str::to_owned)),
            pass: 0,
            tree: Rect::default(),
            first: Rect::default(),
            done: Rect::default(),
        },
        zaxis::RunOptions {
            window_attributes: zaxis::winit::window::Window::default_attributes()
                .with_title("zaxis — Drag and drop")
                .with_inner_size(zaxis::winit::dpi::LogicalSize::new(1100.0, 720.0)),
            ..Default::default()
        },
    )
}
