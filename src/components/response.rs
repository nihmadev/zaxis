use crate::{Id, Rect};

/// Visual interaction state; pressing takes precedence over hovering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WidgetState {
    Idle,
    Hovered,
    Pressed,
    Disabled,
}

/// Widget geometry and interaction for the current UI pass.
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
    pub fn clicked(self) -> bool {
        self.clicked
    }

    /// True when this widget changed its bound value during the current UI pass.
    pub fn changed(self) -> bool {
        self.changed
    }
}
