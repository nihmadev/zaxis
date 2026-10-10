//! Keys addressed to a custom widget, and the entry of a focus group.

use super::*;
use crate::context::{DiagnosticKind, KeyEvent, KeyInterest};

impl Ui<'_> {
    /// Own these keys while the widget of `response` has focus. Declare them in every pass:
    /// a declaration lasts until the pass after it, and a widget that stops declaring a key
    /// gives it back at once.
    ///
    /// Owning a key means the dispatcher consumes each matching press, and its autorepeat
    /// and release, when the event arrives, before the next pass: Actions, containers and
    /// the host never see it, even if the widget ignores the event. Keys that no declaration
    /// names keep their usual path. Several calls add up. A disabled widget claims nothing,
    /// and so does one that cannot take focus (no [`Sense::FOCUS`]) or that keeps its own
    /// keys (a text field, a slider): both are reported as a usage
    /// [diagnostic](crate::Diagnostic).
    ///
    /// Collect the events with [`Self::take_keys`], or use [`Self::keys`] for both.
    pub fn claim_keys(&mut self, response: &Response, interest: KeyInterest<'_>) {
        if response.enabled {
            self.context.claim_keys(response.id, &interest);
        }
    }

    /// The keys the widget of `response` owns that arrived since its last pass, once, in
    /// arrival order: presses, and repeats and releases when the claim asked for them. Each
    /// event carries the modifiers held when it arrived. Events of a widget that is not
    /// enabled, or that was not built in the pass after they arrived, are dropped.
    pub fn take_keys(&mut self, response: &Response) -> Vec<KeyEvent> {
        let events = self.context.take_key_events(response.id);
        if response.enabled {
            events
        } else {
            Vec::new()
        }
    }

    /// [`Self::claim_keys`] and [`Self::take_keys`] in one call, the usual shape:
    ///
    /// ```
    /// # use zaxis::{KeyInterest, Response, Sense, Ui, Vec2};
    /// # use zaxis::winit::keyboard::KeyCode;
    /// # fn knob(ui: &mut Ui<'_>, value: &mut f32) {
    /// let rect = ui.allocate_space(Vec2::splat(48.0));
    /// let response = ui.interact(rect, "knob", Sense::CLICK | Sense::DRAG | Sense::FOCUS);
    /// for key in ui.keys(&response, KeyInterest::arrows().repeats()) {
    ///     match key.code {
    ///         KeyCode::ArrowUp | KeyCode::ArrowRight => *value += 0.05,
    ///         _ => *value -= 0.05,
    ///     }
    /// }
    /// # }
    /// ```
    pub fn keys(&mut self, response: &Response, interest: KeyInterest<'_>) -> Vec<KeyEvent> {
        self.claim_keys(response, interest);
        self.take_keys(response)
    }

    /// Inside a [`FocusGroup`](crate::FocusGroup): make the item of `response` the one Tab
    /// lands on while focus is outside the group. Without it Tab lands on the item focus
    /// was last on, or the first. A choice control names its selected item here. The last
    /// call of a pass wins.
    pub fn focus_entry(&mut self, response: &Response) {
        if !self.context.focus_groups.set_entry(response.id) {
            self.context.report(
                DiagnosticKind::InvalidUsage,
                Some(response.id),
                None,
                || "Ui::focus_entry outside a focus group; it has no effect".into(),
            );
        }
    }
}
