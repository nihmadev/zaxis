use zaxis::{vec2, Presence, Ui};

pub struct Layout {
    pub visible: bool,
    pub expanded: bool,
    pub page: usize,
}
impl Default for Layout {
    fn default() -> Self {
        Self {
            visible: true,
            expanded: true,
            page: 0,
        }
    }
}

pub fn show(ui: &mut Ui<'_>, state: &mut Layout) {
    ui.checkbox(&mut state.visible, "Presence visible");
    ui.horizontal(|ui| {
        Presence::fade().show(ui, "fade", state.visible, |ui| ui.label("Fade"));
        Presence::slide(vec2(-24.0, 0.0)).show(ui, "slide", state.visible, |ui| ui.label("Slide"));
        Presence::scale(0.5).show(ui, "scale", state.visible, |ui| ui.label("Scale"));
        Presence::fade().offset(vec2(0.0, 16.0)).scaling(0.8).show(
            ui,
            "combined",
            state.visible,
            |ui| ui.label("Fade + slide + scale"),
        );
    });
    ui.separator();

    ui.checkbox(&mut state.expanded, "Reveal expanded");
    ui.reveal("reveal", state.expanded, |ui| {
        ui.label("Height follows the measured content.");
        ui.label("Closing content loses input immediately.");
        ui.button("Inside reveal");
    });
    ui.separator();

    ui.collapsing("collapsing", "Collapsing header", |ui| {
        ui.label("Animated disclosure and body");
    });
    ui.separator();

    ui.horizontal(|ui| {
        if ui.button("Previous").clicked() && state.page > 0 {
            state.page -= 1;
        }
        if ui.button("Next").clicked() && state.page < 3 {
            state.page += 1;
        }
    });
    let width = ui.available_width();
    ui.tab_pages("pages", state.page, vec2(width, 60.0), |ui, index| {
        ui.label(format!("Page {}", index + 1));
        ui.label("Directional slide between pages");
    });
}
