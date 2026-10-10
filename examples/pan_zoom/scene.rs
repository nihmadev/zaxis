use zaxis::{
    vec2, Border, Color, Image, ImageSource, Rect, ScrollArea, Sense, Shape, Slider, TextEdit, Ui,
    Vec2, Widget,
};

const LANDSCAPE: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="720" viewBox="0 0 1200 720">
<defs><linearGradient id="sky" x2="0" y2="1"><stop stop-color="#0e2459"/><stop offset="1" stop-color="#edab81"/></linearGradient>
<linearGradient id="lake" x2="0" y2="1"><stop stop-color="#5993a5"/><stop offset="1" stop-color="#172747"/></linearGradient></defs>
<rect width="1200" height="720" fill="url(#sky)"/><circle cx="850" cy="230" r="100" fill="#ffdc9c"/>
<path d="M0 420L210 90L390 335L580 125L860 460L1100 150L1200 280V720H0Z" fill="#496381"/>
<path d="M110 245L210 90L310 250L228 206L205 235L180 202Z M495 234L580 125L677 242L608 218L581 248L554 209Z" fill="#d8dbe9"/>
<path d="M0 510L320 300L535 485L735 360L930 530L1200 350V720H0Z" fill="#253e5b"/>
<path d="M0 530Q350 500 660 550T1200 520V720H0Z" fill="url(#lake)"/>
<path d="M750 580H970M660 610H1070M720 650H990" stroke="#a8c9cd" stroke-width="3" opacity=".6"/>
</svg>"##;

pub fn bounds() -> Rect {
    Rect::from_min_size(vec2(-140.0, -100.0), vec2(1040.0, 620.0))
}
pub struct Scene {
    title: String,
    radius: f32,
    offset: Vec2,
    bright: bool,
    popup: bool,
    objects: [Vec2; 5],
}
impl Default for Scene {
    fn default() -> Self {
        Self {
            title: "Mountain study".into(),
            radius: 42.0,
            offset: Vec2::ZERO,
            bright: true,
            popup: false,
            objects: [
                vec2(-70.0, 0.0),
                vec2(160.0, -40.0),
                vec2(390.0, 160.0),
                vec2(30.0, 270.0),
                vec2(250.0, 350.0),
            ],
        }
    }
}
impl Scene {
    pub fn show(&mut self, ui: &mut Ui<'_>, visible: Rect, image: bool) {
        if image {
            let rect = bounds();
            ui.at("landscape", rect, |ui| {
                ui.add(
                    Image::new(ImageSource::bytes(LANDSCAPE))
                        .size(rect.size())
                        .alt("Mountains reflected in a lake at sunset"),
                );
            });
        } else {
            for (i, center) in self.objects.iter_mut().enumerate() {
                let rect = Rect::from_min_size(
                    *center + self.offset - Vec2::splat(self.radius),
                    Vec2::splat(self.radius * 2.0),
                );
                if rect.intersect(visible).is_empty() {
                    continue;
                }
                let response = ui.interact(rect, ("object", i), Sense::CLICK | Sense::DRAG);
                if response.clicked() {
                    self.bright = !self.bright;
                }
                if response.dragged() {
                    let delta = ui
                        .context()
                        .input_transform(response.id)
                        .inverse()
                        .vector(response.drag_delta());
                    *center += delta;
                }
                let fill = if self.bright {
                    Color::rgb(90 + i as u8 * 23, 145, 230 - i as u8 * 17)
                } else {
                    Color::rgb(114, 98, 156)
                };
                ui.paint(Shape::Circle {
                    center: *center + self.offset,
                    radius: self.radius,
                    fill,
                    border: Border::NONE,
                });
            }
        }
        ui.at(
            "panel",
            Rect::from_min_size(vec2(520.0, 20.0), vec2(280.0, 390.0)),
            |ui| {
                ui.add(
                    TextEdit::new(&mut self.title)
                        .width(260.0)
                        .accessible_label("Title"),
                );
                ui.add(
                    Slider::new(&mut self.radius, 20.0..=80.0)
                        .width(260.0)
                        .accessible_label("Radius"),
                );
                if ui.button("Move shapes").clicked() {
                    self.offset.x = if self.offset.x == 0.0 { 50.0 } else { 0.0 };
                }
                let trigger = ui.button("Palette");
                if trigger.clicked() {
                    self.popup = !self.popup;
                }
                let mut chosen = false;
                zaxis::Popup::new("palette", trigger.rect)
                    .size(vec2(180.0, 110.0))
                    .show(ui, &mut self.popup, |ui| {
                        if ui.button("Blue").clicked() {
                            self.bright = true;
                            chosen = true;
                        }
                        if ui.button("Violet").clicked() {
                            self.bright = false;
                            chosen = true;
                        }
                    });
                if chosen {
                    self.popup = false;
                }
                ScrollArea::vertical()
                    .id_source("objects")
                    .max_height(160.0)
                    .show(ui, |ui| {
                        for (i, object) in self.objects.iter_mut().enumerate() {
                            if ui.button(format!("Move object {}", i + 1)).clicked() {
                                object.y -= 30.0;
                            }
                        }
                    });
            },
        );
    }
}
