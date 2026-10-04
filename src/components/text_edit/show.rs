use super::*;

impl Widget for TextEdit<'_> {
    fn ui(mut self, ui: &mut Ui<'_>) -> Response {
        self.enabled &= ui.is_enabled();
        let id = self.exact_id.unwrap_or_else(|| match self.id {
            Some(id) => ui.scope.with(("text-edit", id)),
            None => ui.auto_id(("text-edit", self.source)),
        });
        let look = self.look(ui);
        match self.area {
            Some(_) => self.show_area(ui, id, look),
            None => self.show_single(ui, id, look),
        }
    }
}
