use crate::{Id, Rect, Vec2};

/// Visual interaction state; pressing takes precedence over hovering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WidgetState {
    Idle,
    Hovered,
    Pressed,
    Disabled,
}

/// Widget geometry and interaction for the current UI pass.
///
/// Event methods (`clicked`, `double_clicked`, `drag_started`, ...) report one
/// user input exactly once: the pass that follows the input sees it, the next
/// pass does not. A response is plain data and stays `Copy`; it borrows nothing.
#[derive(Clone, Copy, Debug)]
pub struct Response {
    pub id: Id,
    pub rect: Rect,
    pub hovered: bool,
    pub pressed: bool,
    pub has_focus: bool,
    /// Keyboard focus indication. Pointer focus alone does not show a ring.
    pub focus_visible: bool,
    pub enabled: bool,
    pub(super) clicked: bool,
    pub(super) changed: bool,
    pub(super) submitted: bool,
    pub(super) lost_focus: bool,
    pub(super) gained_focus: bool,
    pub(super) double_clicked: bool,
    pub(super) secondary_clicked: bool,
    pub(super) drag_started: bool,
    pub(super) dragging: bool,
    pub(super) drag_stopped: bool,
    pub(super) drag_delta: Vec2,
    pub(super) menu_selected: Option<Id>,
}

impl Response {
    /// Enter confirmed the text during this pass; independent of value changes.
    pub fn submitted(self) -> bool {
        self.submitted
    }

    /// The widget lost focus during this pass, including focus changes between redraws.
    pub fn lost_focus(self) -> bool {
        self.lost_focus
    }

    /// The widget received keyboard focus (Tab, press or `request_focus`) since the last pass.
    pub fn gained_focus(self) -> bool {
        self.gained_focus
    }

    pub fn state(self) -> WidgetState {
        if !self.enabled {
            WidgetState::Disabled
        } else if self.pressed {
            WidgetState::Pressed
        } else if self.hovered {
            WidgetState::Hovered
        } else {
            WidgetState::Idle
        }
    }
    /// True on release inside the control, or on Enter/Space release when focused.
    /// A press that turned into a drag is not a click.
    pub fn clicked(self) -> bool {
        self.clicked
    }

    /// The second click of a double click: same widget, within 500 ms and a few
    /// pixels of the first. `clicked` is also true on both clicks.
    pub fn double_clicked(self) -> bool {
        self.double_clicked
    }

    /// Secondary button pressed and released over this widget. Context menus
    /// attached to the widget open on this same signal.
    pub fn secondary_clicked(self) -> bool {
        self.secondary_clicked
    }

    /// The pointer moved past the drag threshold while pressed on this widget.
    pub fn drag_started(self) -> bool {
        self.drag_started
    }

    /// A drag is in progress, including the passes where it starts and stops.
    pub fn dragged(self) -> bool {
        self.dragging
    }

    /// The button was released (or the gesture was cancelled) after a drag.
    pub fn drag_stopped(self) -> bool {
        self.drag_stopped
    }

    /// Pointer movement in logical pixels since the previous pass. The first
    /// delta of a drag starts at the press position, so the deltas of one drag
    /// add up to its total displacement.
    pub fn drag_delta(self) -> Vec2 {
        self.drag_delta
    }

    /// For custom widgets: report that this widget changed its value during this pass,
    /// like the built-in controls do. `Ui::add` then schedules the follow-up redraw.
    pub fn mark_changed(&mut self) {
        self.changed = true;
    }

    /// The context menu attached with [`super::Widget::context_menu`] chose this item.
    pub fn menu_selected(self) -> Option<Id> {
        self.menu_selected
    }

    /// True when this pass carries a one-shot event of any kind.
    pub(super) fn has_event(self) -> bool {
        self.clicked
            || self.changed
            || self.submitted
            || self.lost_focus
            || self.gained_focus
            || self.double_clicked
            || self.secondary_clicked
            || self.drag_started
            || self.drag_stopped
            || self.drag_delta != Vec2::ZERO
            || self.menu_selected.is_some()
    }

    /// True when this widget changed its bound value during the current UI pass.
    pub fn changed(self) -> bool {
        self.changed
    }
}
