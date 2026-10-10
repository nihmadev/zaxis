use crate::prelude::*;
use std::time::Duration;
use winit::{dpi::PhysicalSize, event::ElementState, keyboard::ModifiersState};

mod drag;
mod floating;
mod input;
mod layout;
mod model;
mod motion;

fn p(n: u32) -> PanelId {
    Id::new(("panel", n))
}
fn state() -> DockState {
    DockState::new(DockNode::split(
        "root",
        Layout::Horizontal,
        [
            DockChild::new(DockNode::tabs("left", [p(0), p(1)]), 0.5),
            DockChild::new(DockNode::tabs("right", [p(2), p(3)]), 0.5),
        ],
    ))
}
struct Viewer<'a> {
    built: &'a mut Vec<PanelId>,
    controls: &'a mut HashMap<PanelId, Response>,
    text: &'a mut Option<String>,
}
impl DockViewer for Viewer<'_> {
    fn title(&self, panel: &PanelId) -> String {
        format!(
            "Panel {}",
            (0..100).find(|n| p(*n) == *panel).unwrap_or(100)
        )
    }
    fn ui(&mut self, ui: &mut Ui<'_>, panel: &PanelId) {
        self.built.push(*panel);
        let response = if let Some(text) = self.text {
            ui.add(TextEdit::new(text).width(ui.available_width()))
        } else {
            ui.button("Child")
        };
        self.controls.insert(*panel, response);
    }
}
struct Scene {
    c: Context,
    state: DockState,
    out: DockOutput,
    built: Vec<PanelId>,
    controls: HashMap<PanelId, Response>,
    now: Instant,
    style: DockStyle,
    events: Vec<DockEvent>,
    text: Option<String>,
    actions: DockActions,
}
impl Scene {
    fn new(reduced: bool, scale: f64) -> Self {
        let mut c = Context::new();
        c.set_viewport(
            PhysicalSize::new((800.0 * scale) as u32, (600.0 * scale) as u32),
            scale,
        );
        c.on_input(InputEvent::Focus(true));
        let mut style = c.style().clone();
        style.motion.reduced_motion = reduced;
        c.set_style(style);
        let mut s = Self {
            c,
            state: state(),
            out: DockOutput::default(),
            built: Vec::new(),
            controls: HashMap::new(),
            now: Instant::now(),
            style: DockStyle::default(),
            events: Vec::new(),
            text: None,
            actions: DockActions::default(),
        };
        s.frame();
        s.frame();
        s
    }
    fn frame(&mut self) {
        self.advance(16);
    }
    fn advance(&mut self, millis: u64) {
        self.now += Duration::from_millis(millis);
        self.built.clear();
        self.controls.clear();
        self.c.run_at(self.now, |c| {
            Root::new().padding(Padding::all(8.0)).show(c, |ui| {
                self.out = Dock::new("test-dock", &mut self.state)
                    .style(self.style)
                    .actions(self.actions)
                    .show(
                        ui,
                        Viewer {
                            built: &mut self.built,
                            controls: &mut self.controls,
                            text: &mut self.text,
                        },
                    );
            });
        });
        self.events.extend(self.out.events.clone());
    }
    fn settle(&mut self) {
        self.frame();
        self.advance(3000);
        self.frame();
    }
    fn tab(&self, panel: PanelId) -> Rect {
        fn group(node: &DockNode, panel: PanelId) -> Option<Id> {
            match node {
                DockNode::Tabs { id, panels, .. } => panels.contains(&panel).then_some(*id),
                DockNode::Split { children, .. } => {
                    children.iter().find_map(|c| group(&c.node, panel))
                }
            }
        }
        let gid = self
            .state
            .root
            .as_ref()
            .and_then(|n| group(n, panel))
            .or_else(|| self.state.floats.iter().find_map(|f| group(&f.node, panel)))
            .unwrap();
        let id = Id::new("zaxis-root")
            .with("content")
            .with(("dock", Id::new("test-dock")))
            .with(("tabs", gid))
            .with(("tab-bar", Id::new(gid)))
            .with(("interact", Id::new(("tab", Id::new(panel)))));
        self.c
            .probe()
            .previous_hits
            .iter()
            .find(|h| h.id == id)
            .or_else(|| {
                let source = self
                    .c
                    .probe()
                    .drag
                    .last_sources
                    .iter()
                    .find(|s| s.key == Id::new(panel))?
                    .id;
                self.c.probe().previous_hits.iter().find(|h| h.id == source)
            })
            .expect("tab hit")
            .rect
    }
    fn click(&mut self, point: Vec2) {
        self.c.move_pointer(point);
        self.c.primary_button(ElementState::Pressed);
        self.c.primary_button(ElementState::Released);
        self.frame();
    }
    fn begin(&mut self, panel: PanelId) {
        let at = self.tab(panel).min + vec2(20.0, 16.0);
        self.c.move_pointer(at);
        self.c.primary_button(ElementState::Pressed);
        self.c.move_pointer(at + vec2(0.0, 15.0));
        self.frame();
        self.frame();
        assert!(self.c.dragging().is_some());
    }
}
