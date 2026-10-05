use crate::reveal::fade_up;
use std::time::Duration;
use zaxis::{icons, Align, Button, FontWeight, Text, Ui};

/// Height of the centered title, subtitle and button together, for vertical centering.
const CONTENT_HEIGHT: f32 = 230.0;

/// The first screen: a title, a subtitle and the button that scrolls to the features. Each
/// arrives a little after the one above. Returns whether Explore was pressed.
pub fn show(ui: &mut Ui<'_>, height: f32) -> bool {
    let mut explore = false;
    ui.with_height(height, |ui| {
        ui.add_space(((height - CONTENT_HEIGHT) * 0.5).max(24.0));
        ui.vertical_aligned(Align::Center, |ui| {
            fade_up(ui, "title", Duration::ZERO, |ui| {
                ui.add(Text::new("zaxis").size(72.0).weight(FontWeight::BOLD));
            });
            ui.add_space(8.0);
            fade_up(ui, "subtitle", Duration::from_millis(250), |ui| {
                ui.with_max_width(520.0, |ui| {
                    ui.muted("An immediate mode GUI on wgpu, in a window and in the browser.");
                });
            });
            ui.add_space(24.0);
            fade_up(ui, "explore", Duration::from_millis(500), |ui| {
                let button = Button::new("Explore")
                    .icon(&icons::CHEVRON_DOWN)
                    .min_size(zaxis::vec2(140.0, 40.0));
                explore = ui.add(button).clicked();
            });
        });
    });
    explore
}
