//! One pass of a color picker: retained state brought up to date with the bound color,
//! the row and its open toggle (click or assistive technology), the editor inline or in a
//! floating window, the row's content painted with the final color, and the response.

use super::{
    access,
    editor::Editor,
    panel::{self, Panel},
    row::{self, Content, Row},
    ColorPicker, ColorPickerState, ColorPickerType, HitAction,
};
use crate::{AccessAction, Id, Response, Ui, Vec2, Widget};

impl Widget for ColorPicker<'_> {
    fn ui(mut self, ui: &mut Ui<'_>) -> Response {
        self.enabled &= ui.is_enabled();
        let id = match self.id {
            Some(id) => ui.scope.with(("color-picker", id)),
            None => ui.auto_id(("color-picker", self.source)),
        };
        let original = *self.color;
        let mut state = ui
            .context
            .values
            .color_pickers
            .remove(&id)
            .unwrap_or_else(|| ColorPickerState::new(original, self.default_open));
        state.observe(original);
        let mut style = ui.style().clone();
        style.color_picker.merge(self.style);
        let row_height = style.color_picker.row_height.unwrap_or(24.0);
        let width = self
            .width
            .or(style.color_picker.width)
            .unwrap_or(300.0)
            .min(ui.available_width());
        let row = Row::new(
            crate::Rect::from_min_size(ui.layout.cursor, Vec2::new(width, row_height)),
            width,
        );
        let mut response = ui.response(id, row.rect, self.enabled);
        self.toggle(ui, id, response, &mut state);
        let angle = ui.transition_property(
            id.with("chevron-angle"),
            row.rect,
            if state.open {
                std::f32::consts::FRAC_PI_2
            } else {
                0.0
            },
            style.motion.expand.clone(),
        );
        let mut rect = ui.allocate_space(Vec2::new(width, row_height));
        response.rect = rect;
        super::hit(ui, id, row.rect, self.enabled, HitAction::Activate);
        // The row is the color well; its value is filled in once the editor has run.
        let well = access::well(ui, id, row.rect, &self.text, self.enabled);
        let surface = row::surface(ui, id, row, response, (&style, self.hover_style));
        let panel = Panel {
            editor: Editor {
                id,
                enabled: self.enabled,
                hover: self.hover_style,
                style: &style,
            },
            label: &self.text,
            width,
            requested_width: self.width,
            height: self.editor_height(&style),
        };
        // Floating panels are windows below a modal; inside one the editor expands inline.
        let kind = if ui.context.building_modal() {
            ColorPickerType::Internal
        } else {
            self.picker_type
        };
        let open = state.open;
        let parts = (&mut state, &mut *self.color);
        match kind {
            ColorPickerType::Internal => {
                let height = panel::inline(ui, &panel, row.rect, parts, &mut response);
                rect.max.y = row.rect.max.y + height;
                response.rect = rect;
            }
            ColorPickerType::Floating if self.enabled && open => {
                panel::floating(ui, &panel, row.rect, parts, &mut response)
            }
            ColorPickerType::Floating => {}
        }
        // Paint after editing so the preview reflects this pass's final color.
        let content = Content {
            label: &self.text,
            color: *self.color,
            angle,
            text: surface.text_color,
        };
        row::paint_content(ui, id, row, content, &style);
        access::complete_well(ui, well, id, *self.color, state.open);
        response.changed = original != *self.color;
        state.last_color = *self.color;
        ui.context.values.color_pickers.insert(id, state);
        response
    }
}

impl ColorPicker<'_> {
    /// A click toggles the editor; assistive technology names the state it wants. Closing
    /// commits the fields' drafts; a disabled picker drops them.
    fn toggle(
        &mut self,
        ui: &mut Ui<'_>,
        id: Id,
        response: Response,
        state: &mut ColorPickerState,
    ) {
        let mut open = state.open ^ response.clicked();
        for action in ui.context.take_access_actions(id) {
            match action {
                AccessAction::Expand if self.enabled => open = true,
                AccessAction::Collapse if self.enabled => open = false,
                _ => {}
            }
        }
        if open != state.open {
            state.open = open;
            if !state.open {
                state.commit(self.color);
            }
        }
        if !self.enabled {
            state.discard_drafts();
        }
    }
}
