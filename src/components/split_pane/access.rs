//! Split panes for assistive technology: every boundary is a `Splitter` whose value is the
//! size of the panel before it, in logical pixels, within the range a resize may reach.
//! Its requests become the input the arrow keys produce on a focused boundary.
use super::{allocation, SplitBoundaryOutput, SplitInput, SplitPanel, SplitStyle, Ui};
use crate::{
    accessibility::Scope, AccessAction, AccessActionKind as Kind, AccessOrientation, AccessRole,
    Id, Layout, Rect,
};
use winit::keyboard::{KeyCode, ModifiersState};

/// Requests for boundary `id`, appended to the input of this pass.
pub(super) fn requests(ui: &mut Ui<'_>, id: Id, axis: Layout, events: &mut Vec<SplitInput>) {
    let (back, forward) = match axis {
        Layout::Horizontal => (KeyCode::ArrowLeft, KeyCode::ArrowRight),
        Layout::Vertical => (KeyCode::ArrowUp, KeyCode::ArrowDown),
    };
    for request in ui.context.take_access_actions(id) {
        events.push(match request {
            AccessAction::Increment => SplitInput::Key(forward, ModifiersState::empty()),
            AccessAction::Decrement => SplitInput::Key(back, ModifiersState::empty()),
            AccessAction::SetNumericValue(size) => SplitInput::Set(size as f32),
            _ => continue,
        });
    }
}

struct Handle {
    id: Id,
    enabled: bool,
    size: f32,
    range: (f64, f64),
    node: Option<u32>,
}

/// The boundaries of one pass. A boundary is described right before the content of the
/// panel that follows it, so the tree reads panel, boundary, panel; its bounds arrive
/// later, with its hit region. Empty while nothing is collected.
pub(super) struct Handles {
    list: Vec<Handle>,
    axis: Layout,
    steps: (f32, f32),
}

impl Handles {
    pub(super) fn new(
        ui: &Ui<'_>,
        axis: Layout,
        panels: &[SplitPanel],
        sizes: &[f32],
        boundaries: &[SplitBoundaryOutput],
        style: SplitStyle,
    ) -> Self {
        let mut list = Vec::new();
        if ui.context.a11y_on() {
            list.extend(boundaries.iter().enumerate().map(|(i, out)| Handle {
                id: out.id,
                enabled: out.enabled,
                size: sizes[i],
                range: allocation::range(panels, sizes, i),
                node: None,
            }));
        }
        Self {
            list,
            axis,
            steps: (style.keyboard_step, style.keyboard_large_step),
        }
    }

    /// Add the node of boundary `index` unless it is there already.
    pub(super) fn describe(&mut self, ui: &mut Ui<'_>, index: usize) {
        let (axis, steps) = (self.axis, self.steps);
        let Some(handle) = self.list.get_mut(index).filter(|h| h.node.is_none()) else {
            return;
        };
        let scope = ui.a11y_begin(handle.id, AccessRole::Splitter, |node| {
            // Panels side by side are separated by a vertical bar.
            let orientation = match axis {
                Layout::Horizontal => AccessOrientation::Vertical,
                Layout::Vertical => AccessOrientation::Horizontal,
            };
            node.label("Resize")
                .orientation(orientation)
                .numeric(f64::from(handle.size), handle.range.0, handle.range.1)
                .step(f64::from(steps.0))
                .jump(f64::from(steps.1))
                .disabled(!handle.enabled)
                .action(Kind::Increment)
                .action(Kind::Decrement)
                .action(Kind::SetValue);
        });
        handle.node = scope.0;
        ui.a11y_end(scope, None);
    }

    /// Give boundary `index` its bounds, describing it first when the panel after it had
    /// no content.
    pub(super) fn place(&mut self, ui: &mut Ui<'_>, index: usize, bounds: Rect, clip: Rect) {
        self.describe(ui, index);
        if let Some(node) = self.list.get(index).and_then(|handle| handle.node) {
            ui.context.a11y_end(Scope(Some(node)), Some((bounds, clip)));
        }
    }
}
