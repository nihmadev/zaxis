//! One pass of a combo box: the stages in order, each with its inputs and results.

use super::{
    choice::{self, Choice},
    list::List,
    open::{Input, Open},
    paint, ComboBox, ComboBoxState, Response, Ui, Widget,
};

impl<T: Clone + PartialEq> Widget for ComboBox<'_, T> {
    fn ui(self, ui: &mut Ui<'_>) -> Response {
        let label = crate::components::visible_label(&self.label);
        // The trigger: style, place, hit region and its closed accessibility node.
        let (trigger, mut response) = self.trigger(ui, label);
        // A dismissal, the click, assistive technology and keys open or close the popup.
        let (mut state, was_open) = ComboBoxState::load(ui, trigger.id, self.default_open);
        let input = Input::take(ui, &trigger, response.clicked());
        let requests = state.apply(was_open, input, &trigger, &self.options);
        let selected = self.selected.as_ref();
        state.start(requests.open, &self.options, selected, self.filterable);
        // The popup, animated open or closed: the filter, the highlight and the rows.
        let progress = trigger.animate(ui, state.open);
        let list = List {
            trigger: &trigger,
            label,
            options: &self.options,
            selected,
            filterable: self.filterable,
        };
        let opening = requests.open == Open::Opening;
        let picked = list.show(ui, &mut state, progress, opening, &requests.keys);
        // Every choice takes one path; whatever closed the popup returns focus once.
        let choice = Choice::of_pass(picked, state.open, requests.value);
        response.changed |= choice::apply(
            ui,
            &trigger,
            choice,
            &self.options,
            self.selected,
            &mut state,
        );
        // The trigger after the pass's input.
        trigger.respond(ui, &mut response, state.open);
        let chosen = self
            .options
            .iter()
            .find(|o| Some(&o.value) == self.selected.as_ref())
            .map(|o| o.label.as_str());
        let status = ui.field_status(self.status);
        let invalid = status == crate::SemanticStatus::Error;
        trigger.describe(ui, chosen, &self.placeholder, &state, invalid);
        paint::trigger(
            ui,
            response,
            label,
            trigger.allocation,
            chosen.unwrap_or(&self.placeholder),
            progress,
            &trigger.style,
            trigger.motion(),
            status,
            self.hover_style.or(ui.hover_style),
        );
        ui.context.menus.combo_boxes.insert(trigger.id, state);
        response
    }
}
