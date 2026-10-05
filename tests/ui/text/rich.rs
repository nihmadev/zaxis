use super::support::*;
use zaxis::{
    FontWeight, LabelOutput, LinkActivation, LinkTarget, RichText, SelectableLabel, Shape, Span,
    SpanStyle, Underline,
};

fn paragraph() -> RichText {
    RichText::new()
        .text("Read the ")
        .link("first guide", "https://example.com/one")
        .text(" and the ")
        .link(
            "second guide that is long enough to wrap",
            "https://example.com/two",
        )
        .text(" before ")
        .link_with(
            "the last",
            LinkTarget::new()
                .url("https://example.com/three")
                .id("last"),
        )
        .text(".")
}

fn show(c: &mut Context, width: f32) -> LabelOutput {
    frame(c, |ui| {
        ui.with_width(width, |ui| ui.rich_label(paragraph()))
    })
}

#[test]
fn links_inside_a_paragraph_are_independent_tab_stops_in_order() {
    let mut c = setup();
    let out = show(&mut c, 260.0);
    let ids: Vec<Id> = out.links.iter().map(|l| l.id).collect();
    assert_eq!(ids.len(), 3);
    let mut seen = Vec::new();
    for _ in 0..5 {
        key(&mut c, KeyCode::Tab);
        show(&mut c, 260.0);
        seen.push(c.probe().focused_widget.unwrap());
    }
    // The paragraph itself, then each link in text order, then around again.
    assert_eq!(seen[0], out.response.id);
    assert_eq!(&seen[1..4], &ids[..]);
    assert_eq!(seen[4], out.response.id);
}

#[test]
fn enter_activates_exactly_the_focused_link() {
    let mut c = setup();
    show(&mut c, 260.0);
    for _ in 0..3 {
        key(&mut c, KeyCode::Tab);
        show(&mut c, 260.0);
    }
    key(&mut c, KeyCode::Enter);
    let out = show(&mut c, 260.0);
    let hit = out.activated().expect("an activation");
    assert_eq!(
        (hit.index, hit.url.as_deref()),
        (1, Some("https://example.com/two"))
    );
    assert_eq!(
        out.links.iter().filter(|l| l.activation.is_some()).count(),
        1
    );
}

#[test]
fn a_link_wrapped_over_lines_is_clickable_on_every_fragment_and_only_there() {
    let mut c = setup();
    let out = show(&mut c, 200.0);
    show(&mut c, 200.0);
    let second = out.links[1].id;
    let frags: Vec<Rect> = c
        .probe()
        .previous_hits
        .iter()
        .filter(|h| h.id == second)
        .map(|h| h.rect)
        .collect();
    assert!(frags.len() >= 2, "{frags:?}");
    for frag in &frags {
        click(&mut c, frag.center());
        let out = show(&mut c, 200.0);
        assert_eq!(out.links[1].activation, Some(LinkActivation::Primary));
        assert!(out.links[0].activation.is_none() && out.links[2].activation.is_none());
    }
    // Plain text of the paragraph is not a link.
    click(&mut c, out.response.rect.min + Vec2::new(3.0, 6.0));
    assert!(show(&mut c, 200.0).activated().is_none());
}

#[test]
fn dragging_across_links_selects_the_visible_text_and_never_activates() {
    let mut c = setup();
    let out = show(&mut c, 400.0);
    let rect = out.response.rect;
    let from = rect.min + Vec2::new(2.0, 8.0);
    let to = Vec2::new(rect.max.x - 2.0, rect.min.y + 8.0);
    down(&mut c, from);
    show(&mut c, 400.0);
    c.move_pointer((from + to) * 0.5);
    show(&mut c, 400.0);
    c.move_pointer(to);
    show(&mut c, 400.0);
    up(&mut c);
    let out = show(&mut c, 400.0);
    assert!(out.activated().is_none());
    let copied = c.selected_text().unwrap();
    assert!(
        copied.starts_with("Read the first guide and the second"),
        "{copied:?}"
    );
    assert!(
        !copied.contains("https"),
        "addresses are not part of the copy"
    );
    // A drag that begins on a link selects too, and is not a click on it.
    c.clear_selection();
    show(&mut c, 400.0);
    let link = out.links[0].response.rect;
    down(&mut c, link.min + Vec2::new(3.0, 8.0));
    show(&mut c, 400.0);
    c.move_pointer(link.max + Vec2::new(40.0, -2.0));
    show(&mut c, 400.0);
    up(&mut c);
    let out = show(&mut c, 400.0);
    assert!(out.activated().is_none());
    assert!(c.selected_text().is_some_and(|t| t.len() > 3));
}

fn width_of(c: &mut Context, text: RichText) -> f32 {
    frame(c, |ui| ui.rich_label(text)).response.rect.size().x
}

#[test]
fn spans_change_weight_family_and_color_inside_one_line() {
    let mut c = setup();
    let plain = width_of(&mut c, RichText::new().text("illimitable Wide"));
    let bold = width_of(&mut c, RichText::new().bold("illimitable Wide"));
    let mono = width_of(&mut c, RichText::new().code("illimitable Wide"));
    assert!(bold > plain && mono > plain, "{plain} {bold} {mono}");
    let mixed = width_of(&mut c, RichText::new().text("illimitable ").bold("Wide"));
    assert!(mixed > plain && mixed < bold);
    // The color of a span reaches the painted text.
    let red = zaxis::Color::rgb(220, 30, 30);
    let out = frame(&mut c, |ui| {
        ui.rich_label(RichText::new().text("plain ").colored("red", red))
    });
    let paint = &c.probe().cache[&out.response.id.with("text")].paint;
    let Paint::Rich { runs, .. } = &paint[0] else {
        panic!("styled paragraph: {paint:?}")
    };
    assert!(runs.iter().any(|r| r.color == Some(red)));
}

#[allow(clippy::reversed_empty_ranges)]
#[test]
fn spans_are_validated_on_cluster_boundaries() {
    let text = "e\u{301}x👍🏽y";
    let rich = RichText::from_spans(
        text,
        [
            Span::new(1..2).style(SpanStyle::default().bold()),
            Span::new(3..4).style(SpanStyle::default().bold()),
            Span::new(0..99),
            Span::new(5..3),
        ],
    );
    for span in rich.spans() {
        for edge in [span.range.start, span.range.end] {
            assert!(text.is_char_boundary(edge));
        }
    }
    assert!(rich
        .spans()
        .windows(2)
        .all(|w| w[0].range.end <= w[1].range.start));
    assert_eq!(rich.as_str(), text);
}

#[test]
fn selection_copy_of_rich_text_is_the_visible_text_with_links_by_choice() {
    let mut c = setup();
    let go = |c: &mut Context, urls: bool| {
        frame(c, |ui| {
            ui.add(SelectableLabel::rich(paragraph()).copy_url_in_selection(urls))
        })
    };
    let r = go(&mut c, false);
    click(&mut c, r.rect.min + Vec2::new(2.0, 8.0));
    go(&mut c, false);
    ctrl(&mut c, KeyCode::KeyA);
    go(&mut c, false);
    assert_eq!(
        c.selected_text().as_deref(),
        Some("Read the first guide and the second guide that is long enough to wrap before the last.")
    );
    go(&mut c, true);
    assert!(c
        .selected_text()
        .unwrap()
        .contains("first guide (https://example.com/one)"));
}

fn selection_rects(c: &Context, id: Id) -> Vec<Rect> {
    c.probe().cache[&id.with("selection")]
        .paint
        .iter()
        .map(|p| match p {
            Paint::Shape(Shape::Rect { rect, .. }) => *rect,
            other => panic!("{other:?}"),
        })
        .collect()
}

#[test]
fn selection_and_underlines_sit_on_the_pixel_grid_at_every_scale() {
    for scale in [1.0, 1.25, 1.5, 2.0] {
        let mut c = setup_at((640.0 * scale) as u32, (480.0 * scale) as u32, scale);
        let go = |c: &mut Context| {
            frame(c, |ui| {
                ui.with_width(150.0, |ui| {
                    ui.add(
                        SelectableLabel::new(
                            "a long paragraph of text that wraps over several visual lines in a narrow column",
                        )
                        .size(13.0),
                    )
                })
            })
        };
        let r = go(&mut c);
        click(&mut c, r.rect.center());
        go(&mut c);
        ctrl(&mut c, KeyCode::KeyA);
        go(&mut c);
        let mut rects = selection_rects(&c, r.id);
        assert!(rects.len() >= 4, "scale {scale}: {rects:?}");
        rects.sort_by(|a, b| a.min.y.total_cmp(&b.min.y));
        let on_grid = |v: f32| ((v * scale as f32) - (v * scale as f32).round()).abs() < 1e-3;
        for rect in &rects {
            assert!(
                [rect.min.x, rect.min.y, rect.max.x, rect.max.y]
                    .into_iter()
                    .all(on_grid),
                "scale {scale}: {rect:?}"
            );
        }
        for pair in rects.windows(2) {
            assert_eq!(
                pair[0].max.y, pair[1].min.y,
                "scale {scale}: no gap, no overlap"
            );
        }
        // The underline of a link is a whole number of device pixels thick.
        let link = frame(&mut c, |ui| {
            ui.add(zaxis::Hyperlink::new("underlined").underline(Underline::Always))
        });
        let deco = &c.probe().cache[&link.id.with("deco")].paint;
        let Paint::Shape(Shape::Line { width, .. }) = &deco[0] else {
            panic!("{deco:?}")
        };
        let device = width * scale as f32;
        assert!(
            device >= 1.0 && (device - device.round()).abs() < 1e-3,
            "scale {scale}: {width}"
        );
    }
}

#[test]
fn long_text_is_shaped_once_not_every_frame() {
    let mut c = setup();
    let text = "word ".repeat(6000);
    let go = |c: &mut Context| {
        frame(c, |ui| {
            ui.with_width(500.0, |ui| ui.selectable_label(text.as_str()))
        })
    };
    go(&mut c);
    go(&mut c);
    let built = c.text_layouts_built();
    for _ in 0..10 {
        go(&mut c);
    }
    assert_eq!(c.text_layouts_built(), built);
    // A new width shapes again; the same width afterwards does not.
    frame(&mut c, |ui| {
        ui.with_width(300.0, |ui| ui.selectable_label(text.as_str()))
    });
    assert!(c.text_layouts_built() > built);
}

#[test]
fn a_plain_label_has_no_hit_region_and_no_selection_state() {
    let mut c = setup();
    frame(&mut c, |ui| ui.label("not selectable"));
    frame(&mut c, |ui| ui.label("not selectable"));
    assert!(c
        .probe()
        .previous_hits
        .iter()
        .all(|h| h.action != HitAction::StaticText));
    assert_eq!(c.probe().counts.selection_items, 0);
}

#[test]
fn degenerate_input_never_panics() {
    let mut c = setup();
    for width in [0.0, 0.5, f32::NAN, f32::INFINITY, 1.0] {
        for text in ["", " ", "\n", "👍🏽", "e\u{301}", "wide wide wide"] {
            let r = frame(&mut c, |ui| {
                ui.with_width(width, |ui| {
                    ui.add(SelectableLabel::new(text).truncate(true));
                    ui.selectable_label(text)
                })
            });
            click(&mut c, r.rect.center());
            ctrl(&mut c, KeyCode::KeyA);
            frame(&mut c, |ui| ui.selectable_label(text));
        }
    }
    frame(&mut c, |_| {});
    assert_eq!(
        c.probe().counts.selection_items,
        0,
        "state follows the widgets"
    );
}

#[test]
fn hover_motion_runs_then_settles_without_requesting_redraws() {
    let mut c = Context::new();
    c.set_viewport(winit::dpi::PhysicalSize::new(640, 480), 1.0);
    let start = Instant::now();
    let go = |c: &mut Context, at: Instant| {
        frame_at(c, at, |ui| {
            ui.add(zaxis::Hyperlink::new("Docs").underline(Underline::Always))
        })
    };
    let r = go(&mut c, start);
    c.move_pointer(r.rect.center());
    let mut t = start;
    go(&mut c, t);
    assert!(c.next_repaint().is_some(), "the color is animating");
    for _ in 0..40 {
        t += std::time::Duration::from_millis(16);
        go(&mut c, t);
    }
    assert!(
        c.next_repaint().is_none() && !c.needs_repaint_at(t),
        "settled"
    );
    let _ = FontWeight::BOLD;
}
