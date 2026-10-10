use super::{Response, Ui};
use crate::{context::FocusAxis, AccessOrientation, AccessRole, Id};
use std::hash::Hash;

/// A set of controls that is one Tab stop. Tab and Shift+Tab enter and leave it; inside,
/// the arrow keys along its axis move focus between the controls and Home and End jump to
/// the first and last, skipping controls that are disabled, hidden, clipped away or gone.
/// Focus moves, the value does not: navigation selects nothing and calls nothing.
///
/// Every control built in the closure that takes focus joins: [`Button`](crate::Button)s,
/// regions made with [`Ui::interact`] and `Sense::FOCUS`, other widgets. Clicking one
/// focuses it and makes it the stop, so Tab and Shift+Tab come back to it. A group built
/// inside another is one member of the outer group. Content that opens in another layer
/// (a popup, a window) does not join.
///
/// A control that keeps arrow keys for itself, such as a [`TextEdit`](crate::TextEdit) or a
/// [`Slider`](crate::Slider), is a member too: neighbours move focus onto it, it keeps its
/// own keys while focused, and Tab leaves the group. A custom widget does the same by
/// claiming keys with [`Ui::keys`]. A complex part with several focusable regions and its
/// own arrow-key navigation (a [`TabBar`](crate::TabBar), a [`ListBox`](crate::ListBox)) is
/// a nested [`FocusGroup::slot`]: one member for the group around it, whose arrows then
/// belong to the part alone.
///
/// ```no_run
/// # use zaxis::{FocusGroup, AccessRole, Ui};
/// # fn view(ui: &mut Ui<'_>) {
/// FocusGroup::new("tools")
///     .horizontal()
///     .label("Tools")
///     .role(AccessRole::Toolbar)
///     .show(ui, |ui| {
///         ui.horizontal(|ui| {
///             ui.button("Cut");
///             ui.button("Copy");
///             ui.button("Paste");
///         });
///     });
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct FocusGroup {
    source: Id,
    axis: FocusAxis,
    wrap: bool,
    label: Option<String>,
    role: AccessRole,
}

/// What [`FocusGroup::show`] returns.
pub struct FocusGroupOutput<R> {
    /// The value of the closure.
    pub inner: R,
    /// The group as a whole. `has_focus` is true while focus is on any member,
    /// `gained_focus` and `lost_focus` report focus entering and leaving the group, and
    /// `rect` is the union of its members as laid out.
    pub response: Response,
    stop: Option<Id>,
    moved: Option<Id>,
}

impl<R> FocusGroupOutput<R> {
    /// The region Tab lands on from outside, as the last pass left it. Pass it to
    /// [`Context::request_focus`](crate::Context::request_focus) to move focus into the
    /// group from code.
    pub fn stop(&self) -> Option<Id> {
        self.stop
    }

    /// The region arrow keys, Home or End moved focus to since the last pass, if they did:
    /// the final one when several keys arrived between passes. Tab, clicks and
    /// `request_focus` are not reported. A control that selects as focus moves uses this.
    pub fn navigated(&self) -> Option<Id> {
        self.moved
    }
}

impl FocusGroup {
    /// A group with a stable identity among its siblings. Arrows are horizontal and do
    /// not wrap; the accessibility role is `Group`.
    pub fn new(id_source: impl Hash) -> Self {
        Self {
            source: Id::new(id_source),
            axis: FocusAxis::Horizontal,
            wrap: false,
            label: None,
            role: AccessRole::Group,
        }
    }

    /// A part with several focusable regions that navigates itself. No group moves focus
    /// inside it with the arrow keys or acts on them while focus is inside; as a member of
    /// a group it is one member and one stop.
    pub fn slot(id_source: impl Hash) -> Self {
        Self::new(id_source).axis(FocusAxis::None)
    }

    pub fn axis(mut self, axis: FocusAxis) -> Self {
        self.axis = axis;
        self
    }
    /// Left and Right move focus.
    pub fn horizontal(self) -> Self {
        self.axis(FocusAxis::Horizontal)
    }
    /// Up and Down move focus.
    pub fn vertical(self) -> Self {
        self.axis(FocusAxis::Vertical)
    }
    /// Whether moving past the last member continues at the first, and the other way
    /// round. Off by default: at an end the key does nothing and stays with the group.
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }
    /// The name assistive technology speaks for the group.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
    /// What the group is called to assistive technology: `Toolbar`, `RadioGroup`, `Menu`,
    /// `TabList`, ... The members describe themselves.
    pub fn role(mut self, role: AccessRole) -> Self {
        self.role = role;
        self
    }

    /// Build the members. The closure runs once, in a scope of its own for ids.
    pub fn show<R>(
        self,
        ui: &mut Ui<'_>,
        build: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> FocusGroupOutput<R> {
        let id = ui.scope.with(("focus-group", self.source));
        let scope = ui.a11y_begin(id.with("node"), self.role, |node| {
            if let Some(label) = &self.label {
                node.label(label.as_str());
            }
            match self.axis {
                FocusAxis::Horizontal => {
                    node.orientation(AccessOrientation::Horizontal);
                }
                FocusAxis::Vertical => {
                    node.orientation(AccessOrientation::Vertical);
                }
                FocusAxis::Both | FocusAxis::None => {}
            }
        });
        let stop = ui.context.focus_groups.stop(id, ui.context.focused());
        // The accessibility node closes even when `build` unwinds, like the scopes it nests in.
        let built = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            ui.focus_scope(id, self.axis, self.wrap, |ui| ui.push_id(id, build))
        }));
        ui.a11y_end(scope, None);
        let (inner, end) = match built {
            Ok(built) => built,
            Err(payload) => std::panic::resume_unwind(payload),
        };
        let moved = ui.context.focus_groups.take_moved(id);
        let focused = ui.context.focused();
        let events = ui.context.gestures.events(id);
        let mut response = ui.response(id, end.bounds.unwrap_or_default(), false);
        response.enabled = ui.is_enabled();
        response.has_focus = end.has_focus;
        response.focus_visible =
            response.has_focus && focused.is_some_and(|focus| ui.context.focus_visible(focus));
        response.gained_focus = events.gained_focus;
        response.lost_focus = events.lost_focus;
        FocusGroupOutput {
            inner,
            response,
            stop,
            moved,
        }
    }
}
