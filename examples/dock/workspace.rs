use super::content::Content;
use zaxis::{vec2, DockChild, DockFloat, DockNode, DockState, Id, Layout, Rect};
pub fn panel(n: u32) -> Id {
    Id::new(("demo-panel", n))
}
pub struct Workspace {
    pub layout: DockState,
    pub content: Content,
    pub error: Option<String>,
}
impl Workspace {
    pub fn new() -> Self {
        Self {
            layout: layout(),
            content: Content::new(),
            error: None,
        }
    }
    pub fn save(&mut self) {
        self.error = std::fs::create_dir_all("target")
            .and_then(|()| std::fs::write("target/dock-layout.txt", self.layout.save()))
            .err()
            .map(|e| e.to_string());
    }
    pub fn load(&mut self) {
        match std::fs::read_to_string("target/dock-layout.txt")
            .map_err(|e| e.to_string())
            .and_then(|s| DockState::load(&s, (0..64).map(panel)).map_err(|e| e.to_string()))
        {
            Ok(layout) => self.layout = layout,
            Err(e) => self.error = Some(e),
        }
    }
}
pub fn layout() -> DockState {
    let mut state = DockState::new(DockNode::split(
        "workspace",
        Layout::Horizontal,
        [
            DockChild::new(DockNode::tabs("files", [panel(0)]), 0.18),
            DockChild::new(
                DockNode::split(
                    "center",
                    Layout::Vertical,
                    [
                        DockChild::new(DockNode::tabs("editor", [panel(1)]), 0.73),
                        DockChild::new(DockNode::tabs("bottom", [panel(2)]), 0.27),
                    ],
                ),
                0.57,
            ),
            DockChild::new(
                DockNode::tabs("right", [panel(3), panel(4), panel(5)]),
                0.25,
            ),
        ],
    ));
    state.floats.push(DockFloat {
        node: DockNode::tabs("floating", [panel(6)]),
        bounds: Rect::from_min_size(vec2(690.0, 390.0), vec2(320.0, 250.0)),
    });
    state
}
pub fn stress() -> DockState {
    DockState::new(DockNode::split(
        "stress",
        Layout::Vertical,
        (0..8).map(|row| {
            DockChild::new(
                DockNode::split(
                    ("row", row),
                    Layout::Horizontal,
                    (0..8).map(|col| {
                        DockChild::new(DockNode::tabs((row, col), [panel(row * 8 + col)]), 1.0)
                    }),
                ),
                1.0,
            )
        }),
    ))
}
