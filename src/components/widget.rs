use super::{Response, Ui};

/// An immediate mode component that allocates and paints itself in a UI.
pub trait Widget {
    fn ui(self, ui: &mut Ui<'_>) -> Response;
}
