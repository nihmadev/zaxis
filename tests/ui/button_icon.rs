use crate::prelude::*;
use winit::dpi::PhysicalSize;
use zaxis::{Button, ImageSource, Root};

const ICON: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M6 9l6 6 6-6" fill="none" stroke="#fff" stroke-width="2"/></svg>"##;

fn button(c: &mut Context, icon: bool) -> Response {
    let mut response = None;
    c.run(|c| {
        Root::new().show(c, |ui| {
            let button = Button::new("Explore");
            response = Some(ui.add(if icon {
                button.icon(ImageSource::bytes(ICON))
            } else {
                button
            }));
        })
    });
    response.unwrap()
}

#[test]
fn a_leading_icon_widens_the_button_and_stays_inside_it() {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(600, 400), 1.0);
    let plain = button(&mut c, false);
    let with_icon = button(&mut c, true);
    assert!(with_icon.rect.size().x > plain.rect.size().x);
    assert_eq!(with_icon.rect.size().y, plain.rect.size().y);
    let icons: Vec<_> = c
        .probe()
        .cache
        .values()
        .flat_map(|e| e.paint.iter())
        .filter_map(|p| match p {
            Paint::Image { rect, .. } => Some(*rect),
            _ => None,
        })
        .collect();
    assert_eq!(icons.len(), 1, "one icon is painted");
    let inside = icons[0].intersect(with_icon.rect);
    assert_eq!(inside, icons[0], "the icon lies within the button");
    assert!(
        icons[0].max.x <= with_icon.rect.center().x,
        "before the caption's center"
    );
}

#[test]
fn an_icon_button_clicks_like_any_button() {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(600, 400), 1.0);
    let rect = button(&mut c, true).rect;
    c.move_pointer(rect.center());
    c.primary_button(winit::event::ElementState::Pressed);
    c.primary_button(winit::event::ElementState::Released);
    assert!(button(&mut c, true).clicked());
}
