//! Passive widgets: images, separators, cards, badges, progress bars, spinners and
//! loading placeholders.

use super::*;

fn window(context: &mut Context, build: impl FnOnce(&mut Ui<'_>)) {
    Window::new("Panel").show(context, build);
}

fn pixels() -> ImageSource {
    ImageSource::rgba([2, 2], vec![255; 16])
}

fn children(harness: &Harness) -> Vec<(Role, String)> {
    let tree = &harness.tree;
    let window = tree.expect(Role::Window, "Panel");
    let named = |id: &NodeId| (tree.node(*id).role(), tree.name(*id));
    tree.node(window).children().iter().map(named).collect()
}

#[test]
fn an_image_is_named_by_its_alt_text_and_a_decorative_one_is_skipped() {
    let mut harness = Harness::new();
    let source = pixels();
    let build = |ctx: &mut Context| {
        window(ctx, |ui| {
            ui.add(
                Image::new(&source)
                    .alt("Harbour at dusk")
                    .id_source("photo"),
            );
            ui.add(Image::new(&source).decorative().id_source("ornament"));
            ui.add(Image::new(&source).alt("").id_source("empty alt"));
            ui.add(Button::new("Save").icon(&source));
        });
    };
    harness.pass(build);
    let expected = [
        (Role::Image, "Harbour at dusk".to_owned()),
        (Role::Button, "Save".to_owned()),
    ];
    assert_eq!(
        children(&harness),
        expected,
        "decoration and a button's icon add no node"
    );
    // Pixels are uploaded by the image worker: busy until they are ready.
    let deadline = Instant::now() + Duration::from_secs(10);
    while harness.node(Role::Image, "Harbour at dusk").is_busy() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
        harness.pass(build);
    }
    let image = harness.node(Role::Image, "Harbour at dusk");
    assert!(!image.is_busy() && !image.supports_action(Action::Click));
    assert_eq!(
        harness.context.diagnostics().len(),
        0,
        "{:?}",
        harness.context.diagnostics()
    );
    assert_eq!(harness.pass(build), None);
}

#[test]
fn an_image_without_a_name_is_reported() {
    let mut harness = Harness::new();
    let source = pixels();
    harness.pass(|ctx| window(ctx, |ui| _ = ui.image(&source)));
    assert_eq!(harness.tree.all(Role::Image).len(), 1);
    assert_eq!(missing_names(&harness.context), 1);
    harness.pass(|ctx| {
        window(ctx, |ui| {
            ui.add(Image::new(&source).accessible_label("Logo"));
        })
    });
    assert!(harness.tree.find(Role::Image, "Logo").is_some());
    assert_eq!(missing_names(&harness.context), 0);
}

#[test]
fn a_clickable_image_is_clicked_once_and_cannot_hide_as_decoration() {
    let mut harness = Harness::new();
    let source = pixels();
    let clicks = Cell::new(0);
    let build = |ctx: &mut Context, enabled: bool| {
        window(ctx, |ui| {
            ui.add_enabled_ui(enabled, |ui| {
                let image = Image::new(&source).alt("Open preview").interactive(true);
                count(&clicks, ui.add(image).clicked());
            });
        });
    };
    harness.pass(|ctx| build(ctx, true));
    let image = harness.tree.expect(Role::Image, "Open preview");
    assert!(harness.tree.node(image).supports_action(Action::Click));
    assert!(harness.act(image, Action::Click));
    harness.settle(|ctx| build(ctx, true));
    assert_eq!(clicks.get(), 1);
    harness.pass(|ctx| build(ctx, false));
    assert!(harness.tree.node(image).is_disabled());
    assert!(!harness.act(image, Action::Click));
    harness.settle(|ctx| build(ctx, false));
    assert_eq!(clicks.get(), 1);

    let mut harness = Harness::new();
    harness.pass(|ctx| {
        window(ctx, |ui| {
            ui.add(Image::new(&source).decorative().interactive(true));
        });
    });
    assert_eq!(
        harness.tree.all(Role::Image).len(),
        1,
        "a control stays in the tree"
    );
    assert_eq!(missing_names(&harness.context), 1);
}

#[test]
fn an_image_is_busy_while_it_loads_and_describes_its_failure() {
    let mut harness = Harness::new();
    let broken = ImageSource::encoded(vec![1, 2, 3]);
    let build = |ctx: &mut Context| {
        window(ctx, |ui| _ = ui.add(Image::new(&broken).alt("Chart")));
    };
    harness.pass(build);
    let image = harness.tree.expect(Role::Image, "Chart");
    assert!(
        harness.tree.node(image).is_busy(),
        "decoding has not finished in the first pass"
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    while harness.tree.node(image).description().is_none() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
        harness.pass(build);
    }
    let node = harness.tree.node(image);
    assert!(
        node.description()
            .is_some_and(|text| text.starts_with("image failed")),
        "{node:?}"
    );
    assert!(!node.is_busy());
    assert_eq!(harness.pass(build), None);
}

#[test]
fn a_separator_and_a_skeleton_add_nothing_to_the_tree() {
    let mut harness = Harness::new();
    let build = |ctx: &mut Context| {
        window(ctx, |ui| {
            ui.label("Above");
            ui.separator();
            ui.add(Separator::vertical(20.0));
            ui.skeleton(24.0);
            ui.add(Skeleton::new(12.0).width(80.0));
            ui.label("Below");
        });
    };
    let start = Instant::now();
    pass_at(&mut harness, start, build);
    let expected = [
        (Role::Label, "Above".to_owned()),
        (Role::Label, "Below".to_owned()),
    ];
    assert_eq!(children(&harness), expected);
    assert!(
        harness.context.wants_animation_frame(),
        "the placeholder shimmers"
    );
    let (_, updates) = advance(&mut harness, start, Duration::from_millis(1500), build);
    assert_eq!(
        updates, 0,
        "a shimmering placeholder never touches the tree"
    );
}

#[test]
fn a_card_groups_its_content() {
    for in_row in [false, true] {
        let mut harness = Harness::new();
        let saves = Cell::new(0);
        let card_rect = Cell::new(Rect::default());
        let build = |ctx: &mut Context| {
            window(ctx, |ui| {
                ui.label("Before");
                let card = |ui: &mut Ui<'_>| {
                    let output = Card::new("account").width(220.0).show(ui, |ui| {
                        ui.label("Account");
                        count(&saves, ui.button("Save").clicked());
                    });
                    card_rect.set(output.rect);
                };
                if in_row {
                    ui.horizontal(|ui| {
                        ui.label("Left");
                        card(ui);
                    });
                } else {
                    card(ui);
                }
                ui.label("After");
            });
        };
        harness.settle(build);
        let tree = &harness.tree;
        let save = tree.expect(Role::Button, "Save");
        let card = tree.parent(save).expect("the button has a parent");
        let node = tree.node(card);
        assert_eq!(
            node.role(),
            Role::GenericContainer,
            "a nameless group only structures"
        );
        assert_eq!(tree.parent(card), Some(tree.expect(Role::Window, "Panel")));
        assert_eq!(tree.parent(tree.expect(Role::Label, "Account")), Some(card));
        assert_ne!(tree.parent(tree.expect(Role::Label, "After")), Some(card));
        let (outer, inner) = (bounds(node), bounds(tree.node(save)));
        assert!(
            outer.x0 < inner.x0 && outer.y0 < inner.y0,
            "row {in_row}: padding around"
        );
        assert!(outer.x1 > inner.x1 && outer.y1 > inner.y1, "row {in_row}");
        assert_eq!(
            outer.x1 - outer.x0,
            220.0,
            "row {in_row}: the frame, not the content"
        );
        assert!((outer.y1 - outer.y0 - f64::from(card_rect.get().size().y)).abs() <= 1.0);
        assert!(harness.act(save, Action::Click));
        harness.settle(build);
        assert_eq!(saves.get(), 1);
        assert_eq!(harness.pass(build), None);
    }
}

#[test]
fn a_badge_is_a_label_and_an_empty_one_is_decoration() {
    let mut harness = Harness::new();
    let build = |ctx: &mut Context| {
        window(ctx, |ui| {
            ui.add(Badge::new("12"));
            ui.add(Badge::new("New##inbox"));
            ui.add(Badge::new("").id_source("dot"));
        });
    };
    harness.pass(build);
    let expected = [
        (Role::Label, "12".to_owned()),
        (Role::Label, "New".to_owned()),
    ];
    assert_eq!(children(&harness), expected);
    for name in ["12", "New"] {
        let id = harness.tree.expect(Role::Label, name);
        let runs = check_text(&harness, id, name, true);
        // The text sits inside the chip, behind its padding.
        let (chip, text) = (bounds(harness.tree.node(id)), bounds(&runs[0]));
        assert!(
            text.x0 >= chip.x0 + 5.0 && text.x1 <= chip.x1 - 5.0,
            "{chip:?} {text:?}"
        );
        // The line box is centered on the glyphs' ink, so it may poke out by a pixel or two.
        assert!(
            text.y0 >= chip.y0 - 2.5 && text.y1 <= chip.y1 + 2.5,
            "{chip:?} {text:?}"
        );
        assert!(
            harness.tree.node(id).live().is_none(),
            "a badge is not a live region"
        );
    }
    assert_eq!(harness.pass(build), None);
}

#[test]
fn progress_publishes_the_state_not_the_animation() {
    let mut harness = Harness::new();
    let build = |ctx: &mut Context, state: ProgressState| {
        window(ctx, |ui| {
            ui.add(Progress::new(state).accessible_label("Download"));
        });
    };
    let start = Instant::now();
    pass_at(&mut harness, start, |ctx| {
        build(ctx, ProgressState::Determinate(0.42))
    });
    let id = harness.tree.expect(Role::ProgressIndicator, "Download");
    let range = |harness: &Harness| {
        let node = harness.tree.node(id);
        (
            node.numeric_value(),
            node.min_numeric_value(),
            node.max_numeric_value(),
        )
    };
    assert_eq!(range(&harness), (Some(42.0), Some(0.0), Some(100.0)));
    assert_eq!(harness.tree.node(id).value(), Some("42%"));
    assert!(!harness.tree.node(id).is_busy());
    let idle = |ctx: &mut Context| build(ctx, ProgressState::Determinate(0.42));
    let (now, updates) = advance(&mut harness, start, Duration::from_millis(300), idle);
    assert_eq!(updates, 0);

    let busy = |ctx: &mut Context| build(ctx, ProgressState::Indeterminate { active: true });
    let (now, updates) = advance(&mut harness, now, Duration::from_millis(1500), busy);
    assert_eq!(
        updates, 1,
        "the state changed once; the moving segment publishes nothing"
    );
    assert!(harness.context.wants_animation_frame());
    assert_eq!(
        range(&harness),
        (None, None, None),
        "no invented percentage"
    );
    assert!(harness.tree.node(id).is_busy() && harness.tree.node(id).value().is_none());

    let paused = |ctx: &mut Context| build(ctx, ProgressState::Indeterminate { active: false });
    let (now, updates) = advance(&mut harness, now, Duration::from_millis(100), paused);
    assert_eq!(updates, 1);
    assert!(!harness.tree.node(id).is_busy());
    pass_at(&mut harness, now, |ctx| build(ctx, ProgressState::Complete));
    assert_eq!(range(&harness).0, Some(100.0));
    assert_eq!(harness.tree.node(id).value(), Some("100%"));
    // Values that are not a fraction are clamped like the painted bar, never published raw.
    for (state, expected) in [(f32::NAN, 0.0), (7.0, 100.0), (-1.0, 0.0)] {
        harness.pass(|ctx| build(ctx, ProgressState::Determinate(state)));
        assert_eq!(range(&harness).0, Some(expected), "{state}");
    }
    assert!(!harness.tree.node(id).supports_action(Action::Click));
}

#[test]
fn a_spinner_is_busy_while_active_and_silent_while_it_spins() {
    let mut harness = Harness::new();
    let build = |ctx: &mut Context, active: bool| {
        window(ctx, |ui| {
            ui.add(Loader::new().active(active).label("Loading messages"));
            ui.loader(active);
        });
    };
    let start = Instant::now();
    pass_at(&mut harness, start, |ctx| build(ctx, true));
    let spinners = harness.tree.all(Role::ProgressIndicator);
    assert_eq!(spinners.len(), 2);
    assert_eq!(harness.tree.name(spinners[0]), "Loading messages");
    assert_eq!(
        harness.tree.name(spinners[1]),
        "",
        "a spinner beside its explanation"
    );
    assert!(spinners.iter().all(|id| harness.tree.node(*id).is_busy()));
    assert_eq!(missing_names(&harness.context), 0);
    assert!(harness.context.wants_animation_frame(), "it does spin");
    let spin = |ctx: &mut Context| build(ctx, true);
    let (now, updates) = advance(&mut harness, start, Duration::from_millis(2000), spin);
    assert_eq!(updates, 0, "a spinning loader never touches the tree");

    // Stopped, it is a still icon: nothing to announce.
    pass_at(&mut harness, now, |ctx| build(ctx, false));
    assert!(harness.tree.all(Role::ProgressIndicator).is_empty());
    assert_eq!(
        advance(&mut harness, now, Duration::from_millis(100), |ctx| build(
            ctx, false
        ))
        .1,
        0
    );
}
