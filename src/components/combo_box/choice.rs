//! Choosing an option by pointer, keyboard or assistive technology, applied one way.

use super::{trigger::Trigger, ComboBoxOption, ComboBoxState, Ui};

/// How an option was chosen; [`apply`] applies every kind the same way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Choice {
    /// A click on a row of the open list.
    Pointer(usize),
    /// Enter or Space on the highlighted row.
    Keyboard(usize),
    /// `SetValue` from assistive technology, by the option's label.
    Access(usize),
}

impl Choice {
    /// The pass's choice: the list's counts only if the popup is still open after the
    /// list was built; a request of assistive technology counts either way and yields to
    /// the list's.
    pub(super) fn of_pass(list: Option<Self>, open: bool, access: Option<usize>) -> Option<Self> {
        list.filter(|_| open).or(access.map(Self::Access))
    }

    fn index(self) -> usize {
        match self {
            Self::Pointer(index) | Self::Keyboard(index) | Self::Access(index) => index,
        }
    }
}

/// The one path for a choice of any kind, and for whatever else closed the popup this
/// pass. A chosen value is stored and reported (returns `true`) only when it differs from
/// the selection, and the popup closes. A popup still registered as open is dismissed
/// here, which returns focus to the trigger once, and its own dismissal is forgotten so
/// the next pass does not take it for an outside one; the open animation turns back.
pub(super) fn apply<T: Clone + PartialEq>(
    ui: &mut Ui<'_>,
    trigger: &Trigger,
    choice: Option<Choice>,
    options: &[ComboBoxOption<T>],
    selected: &mut Option<T>,
    state: &mut ComboBoxState,
) -> bool {
    let mut changed = false;
    if let Some(choice) = choice {
        let value = &options[choice.index()].value;
        if selected.as_ref() != Some(value) {
            *selected = Some(value.clone());
            changed = true;
        }
        state.open = false;
    }
    if !state.open && ui.context.popup_open(trigger.popup) {
        ui.context.close_popup_of_builder(trigger.popup);
        trigger.animate_closed(ui);
    }
    changed
}
