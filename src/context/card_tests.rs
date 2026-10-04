use super::*;
use crate::*;
use winit::dpi::PhysicalSize;

fn setup() -> Context {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(800, 600), 1.0);
    let mut style = c.style().clone();
    style.motion.reduced_motion = true;
    c.set_style(style);
    c
}

fn pass(c: &mut Context, rows: usize) -> (CardOutput<Vec<Rect>>, Rect) {
    let mut out = None;
    let mut after = Rect::default();
    c.run(|c| {
        Window::new("Card test").show(c, |ui| {
            out = Some(
                Card::new("card")
                    .padding(Padding::all(10.0))
                    .show(ui, |ui| {
                        (0..rows)
                            .map(|i| ui.button(format!("Row {i}")).rect)
                            .collect()
                    }),
            );
            after = ui.button("After").rect;
        });
    });
    (out.unwrap(), after)
}

#[test]
fn frame_encloses_content_with_padding_and_settles() {
    let mut c = setup();
    pass(&mut c, 2);
    let (card, after) = pass(&mut c, 2);
    let (first, last) = (card.inner[0], card.inner[1]);
    assert_eq!(first.min.x - card.rect.min.x, 10.0);
    assert_eq!(first.min.y - card.rect.min.y, 10.0);
    assert_eq!(card.rect.max.y - last.max.y, 10.0);
    assert!(card.rect.size().x > first.size().x + 20.0);
    // The next sibling follows the frame by exactly one layout gap.
    assert_eq!(after.min.y - card.rect.max.y, c.style().spacing);
}

#[test]
fn growing_content_resizes_the_frame_after_one_redraw_and_then_idles() {
    let mut c = setup();
    pass(&mut c, 1);
    let (small, _) = pass(&mut c, 1);
    pass(&mut c, 3);
    let (large, _) = pass(&mut c, 3);
    assert!(large.rect.size().y > small.rect.size().y + 2.0 * c.style().control_height);
    let stats = c.cache_stats();
    pass(&mut c, 3);
    assert_eq!(
        c.cache_stats().tessellated_elements,
        stats.tessellated_elements
    );
    assert!(!c.needs_repaint());
}

#[test]
fn removed_cards_drop_their_retained_height() {
    let mut c = setup();
    pass(&mut c, 1);
    assert_eq!(c.cards.len(), 1);
    c.run(|c| {
        Window::new("Card test").show(c, |ui| {
            ui.button("Only");
        });
    });
    assert!(c.cards.is_empty());
}
