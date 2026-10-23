use super::*;

fn aligned_wrap(c: &mut Context, lines: Align, items: Align) -> (WrapOutput<Vec<Id>>, Vec<Rect>) {
    let out = root(c, |ui| {
        Wrap::new("w")
            .width(200.0)
            .spacing(vec2(10.0, 4.0))
            .align_lines(lines)
            .align_items(items)
            .show(ui, |ui| {
                [("a", 100.0, 10.0), ("b", 80.0, 30.0), ("c", 60.0, 20.0)]
                    .map(|(key, w, h)| block(ui, key, w, h).id)
                    .to_vec()
            })
    });
    let rects = out.inner.iter().map(|&id| placed(c, id)).collect();
    (out, rects)
}

#[test]
fn lines_start_center_or_end_in_the_width() {
    let mut c = setup();
    // Line one holds a (100) only: 100 + 10 + 80 > 200 is false -> a and b fit (190).
    let (_, start) = aligned_wrap(&mut c, Align::Start, Align::Start);
    near(start[0], rect(0.0, 0.0, 100.0, 10.0));
    near(start[1], rect(110.0, 0.0, 80.0, 30.0));
    near(start[2], rect(0.0, 34.0, 60.0, 20.0));
    let (out, center) = aligned_wrap(&mut c, Align::Center, Align::Start);
    near(center[0], rect(5.0, 0.0, 100.0, 10.0));
    near(center[2], rect(70.0, 34.0, 60.0, 20.0));
    near(out.lines[1], rect(70.0, 34.0, 60.0, 20.0));
    let (_, end) = aligned_wrap(&mut c, Align::End, Align::Start);
    near(end[1], rect(120.0, 0.0, 80.0, 30.0));
    near(end[2], rect(140.0, 34.0, 60.0, 20.0));
}

#[test]
fn items_start_center_or_end_in_the_height_of_their_line() {
    let mut c = setup();
    let (_, start) = aligned_wrap(&mut c, Align::Start, Align::Start);
    let (_, center) = aligned_wrap(&mut c, Align::Start, Align::Center);
    let (_, end) = aligned_wrap(&mut c, Align::Start, Align::End);
    assert_eq!(
        [start[0].min.y, center[0].min.y, end[0].min.y],
        [0.0, 10.0, 20.0]
    );
    // The tallest item defines the line and does not move.
    assert_eq!([start[1].min.y, center[1].min.y, end[1].min.y], [0.0; 3]);
    // A line of one item has no spare height.
    assert_eq!([start[2].min.y, center[2].min.y, end[2].min.y], [34.0; 3]);
}

#[test]
fn an_item_wider_than_the_wrap_stays_at_the_leading_edge_whatever_the_alignment() {
    let mut c = setup();
    for align in [Align::Start, Align::Center, Align::End] {
        let out = root(&mut c, |ui| {
            Wrap::new("w")
                .width(100.0)
                .align_lines(align)
                .show(ui, |ui| block(ui, "wide", 150.0, 10.0).id)
        });
        near(placed(&c, out.inner), rect(0.0, 0.0, 150.0, 10.0));
        assert_eq!(out.rect.size().x, 150.0, "the area grows to the item");
    }
}

#[test]
fn a_line_of_a_gap_and_trailing_space_ignores_space_at_its_ends() {
    let mut c = setup();
    let out = root(&mut c, |ui| {
        Wrap::new("w")
            .width(200.0)
            .gap(10.0)
            .align_lines(Align::End)
            .show(ui, |ui| {
                ui.add_space(50.0);
                let a = block(ui, "a", 40.0, 10.0).id;
                ui.add_space(20.0);
                let b = block(ui, "b", 40.0, 10.0).id;
                ui.add_space(50.0);
                [a, b]
            })
    });
    // 40 + (20 + 10) + 40 = 110, flush right; the outer spaces do not count.
    near(placed(&c, out.inner[0]), rect(90.0, 0.0, 40.0, 10.0));
    near(placed(&c, out.inner[1]), rect(160.0, 0.0, 40.0, 10.0));
    assert_eq!(out.items.len(), 2, "space is not an item");
}
