use std::ops::BitOr;

/// Which input a custom widget reacts to; passed to [`super::Ui::interact`].
///
/// Combine with `|`: `Sense::CLICK | Sense::FOCUS`. A sense only decides which
/// events and states the [`super::Response`] reports; the region always follows
/// the same clip, scroll, disabled and layer rules as built-in controls.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Sense(u8);

impl Sense {
    /// Passive: no input state at all.
    pub const NONE: Self = Self(0);
    /// `hovered` follows the pointer; presses still reach widgets underneath.
    pub const HOVER: Self = Self(1);
    /// Press and release inside produce `clicked`, `double_clicked` and
    /// `secondary_clicked`. Enter and Space click while focused.
    pub const CLICK: Self = Self(2);
    /// A press captures the pointer; movement beyond a few pixels produces
    /// `drag_started`, `drag_delta` and `drag_stopped`, also outside the region.
    pub const DRAG: Self = Self(4);
    /// Tab and presses move keyboard focus here: `has_focus`, `gained_focus`,
    /// `lost_focus`, `focus_visible`.
    pub const FOCUS: Self = Self(8);
    /// A middle-button press and release inside produce `middle_clicked`, and the press is
    /// not the start of an autoscroll. It claims no other input: combine it with `CLICK`.
    pub const MIDDLE: Self = Self(16);

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0 && other.0 != 0
    }
    pub const fn is_none(self) -> bool {
        self.0 == 0
    }
    pub const fn click(self) -> bool {
        self.contains(Self::CLICK)
    }
    pub const fn drag(self) -> bool {
        self.contains(Self::DRAG)
    }
    pub const fn focus(self) -> bool {
        self.contains(Self::FOCUS)
    }
    pub const fn middle(self) -> bool {
        self.contains(Self::MIDDLE)
    }
    /// Whether a pointer press anywhere inside must be claimed by this region.
    pub(crate) const fn claims_pointer(self) -> bool {
        self.0 & (Self::CLICK.0 | Self::DRAG.0 | Self::FOCUS.0) != 0
    }
}

impl BitOr for Sense {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}
