//! The vertical axis of both variants: swipe, keys, wheel and indicator.
use super::Rig;
use crate::prelude::*;

fn rig(images: bool) -> Rig {
    let mut rig = Rig::new(4);
    rig.images = images;
    rig.vertical = true;
    rig.settle();
    rig
}

#[test]
fn a_vertical_swipe_turns_the_page_in_both_variants() {
    for images in [false, true] {
        let mut rig = rig(images);
        let start = rig.empty();
        rig.drag(start, Vec2::new(0.0, -90.0), 6, 16);
        assert!(rig.position() > 0.2, "follows the pointer vertically");
        rig.tick(200);
        rig.release();
        assert!(rig.tick(16).changed);
        rig.settle();
        assert_eq!(rig.page, 1, "images {images}");
        // A horizontal drag does nothing on a vertical carousel.
        let start = rig.empty();
        rig.drag(start, Vec2::new(-120.0, 0.0), 6, 16);
        rig.tick(200);
        rig.release();
        rig.settle();
        assert_eq!(rig.page, 1);
    }
}

#[test]
fn vertical_keys_and_wheel_follow_the_axis() {
    for images in [false, true] {
        let mut rig = rig(images);
        rig.focus();
        assert_eq!(
            rig.page, 0,
            "focusing must not turn the page (images {images})"
        );
        assert!(rig.key(KeyCode::ArrowDown).changed && rig.page == 1);
        assert!(!rig.key(KeyCode::ArrowRight).changed);
        assert!(rig.key(KeyCode::ArrowUp).changed && rig.page == 0);
        rig.settle();
        rig.c.move_pointer(rig.empty());
        rig.tick(16);
        assert!(rig.c.scroll_wheel(Vec2::new(0.0, 14.0)));
        assert!(rig.tick(16).changed && rig.page == 1, "images {images}");
    }
}

#[test]
fn vertical_slides_line_up_along_y() {
    let rig = rig(true);
    let layers = rig.c.probe().carousels[&rig.id()].layers.clone();
    let front = layers.iter().find(|(k, _)| *k == 0).unwrap().1;
    let next = layers.iter().find(|(k, _)| *k == 1).unwrap().1;
    assert!(next.center().y > front.center().y, "neighbours are below");
    assert!((next.center().x - front.center().x).abs() < 20.0);
}
