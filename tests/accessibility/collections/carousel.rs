use super::*;

struct Deck {
    harness: Harness,
    page: usize,
    pages: usize,
    images: bool,
    enabled: bool,
    indicator: CarouselIndicator,
    /// Passes in which the carousel reported that it changed the page.
    changes: usize,
    opened: Vec<usize>,
}

impl Deck {
    fn new(pages: usize) -> Self {
        let mut harness = Harness::new();
        // Transitions finish at once: the tree is asserted at rest.
        let mut style = harness.context.style().clone();
        style.motion.reduced_motion = true;
        harness.context.set_style(style);
        let mut deck = Self {
            harness,
            page: 0,
            pages,
            images: false,
            enabled: true,
            indicator: CarouselIndicator::Dots,
            changes: 0,
            opened: Vec::new(),
        };
        deck.settle();
        deck
    }
    fn pass(&mut self) -> Option<usize> {
        let (page, changes, opened) = (&mut self.page, &mut self.changes, &mut self.opened);
        let mut carousel = Carousel::new("deck")
            .accessible_label("Steps")
            .pages(self.pages)
            .size(Vec2::new(420.0, 260.0))
            .arrows(true)
            .indicator(self.indicator)
            .enabled(self.enabled);
        if self.images {
            carousel = carousel.images();
        }
        self.harness.pass(|ctx| {
            Root::new().show(ctx, |ui| {
                let out = carousel.show(ui, page, |ui, index| {
                    ui.label(format!("Step {index}"));
                    if ui.button(format!("Open {index}")).clicked() {
                        opened.push(index);
                    }
                });
                *changes += usize::from(out.changed);
            });
        })
    }
    fn settle(&mut self) {
        for _ in 0..6 {
            self.pass();
        }
    }
    fn region(&self) -> NodeId {
        self.harness.tree.expect(Role::Region, "Steps")
    }
    fn act(&mut self, role: Role, name: &str, action: Action) -> bool {
        let node = self.harness.tree.expect(role, name);
        let accepted = self.harness.act(node, action);
        self.settle();
        accepted
    }
    /// Names of the page groups that assistive technology can reach.
    fn exposed(&self) -> Vec<String> {
        let pages = children(&self.harness, self.region(), Role::Group);
        let shown = pages
            .into_iter()
            .filter(|page| !hidden(&self.harness, *page));
        shown.map(|page| self.harness.tree.name(page)).collect()
    }
}

#[test]
fn a_carousel_is_a_region_whose_current_page_is_the_one_exposed() {
    let mut deck = Deck::new(4);
    let region = deck.region();
    let node = deck.harness.tree.node(region);
    let range = (node.min_numeric_value(), node.max_numeric_value());
    assert_eq!(
        (node.numeric_value(), range),
        (Some(1.0), (Some(1.0), Some(4.0)))
    );
    assert!(node.supports_action(Action::Increment) && node.supports_action(Action::Focus));
    assert_eq!(deck.exposed(), ["Page 1"]);
    let tree = &deck.harness.tree;
    let page = tree.expect(Role::Group, "Page 1");
    assert_eq!(
        (
            tree.node(page).position_in_set(),
            tree.node(page).size_of_set()
        ),
        (Some(0), Some(4))
    );
    // The content of the page is inside its group, and the group covers it.
    let (label, button) = (
        tree.expect(Role::Label, "Step 0"),
        tree.expect(Role::Button, "Open 0"),
    );
    assert_eq!(
        (tree.parent(label), tree.parent(button)),
        (Some(page), Some(page))
    );
    let (card, inside) = (logical(&deck.harness, page), logical(&deck.harness, button));
    assert!(
        card.min.x <= inside.min.x && card.max.y >= inside.max.y,
        "{card:?} {inside:?}"
    );
    // Sheets behind the front card are drawn but not offered.
    for other in children(&deck.harness, region, Role::Group) {
        assert_eq!(hidden(&deck.harness, other), other != page);
    }
    assert!(tree
        .find(Role::Button, "Open 1")
        .is_none_or(|id| hidden(&deck.harness, id)));

    // Arrows and indicator items are named buttons.
    let previous = tree.node(tree.expect(Role::Button, "Previous"));
    assert!(previous.is_disabled(), "nothing before the first page");
    assert!(!tree.node(tree.expect(Role::Button, "Next")).is_disabled());
    for n in 1..=4 {
        let item = tree.expect(Role::Button, &format!("Page {n}"));
        assert!(tree.node(item).supports_action(Action::Click));
    }
    assert!(
        deck.harness.context.diagnostics().is_empty(),
        "{:?}",
        deck.harness.context.diagnostics()
    );
    assert_eq!(deck.pass(), None, "an idle pass publishes nothing");
}

#[test]
fn next_previous_and_indicator_clicks_turn_the_page_once() {
    let mut deck = Deck::new(4);
    assert!(deck.act(Role::Button, "Next", Action::Click));
    assert_eq!((deck.page, deck.changes), (1, 1), "one request, one change");
    assert_eq!(deck.exposed(), ["Page 2"]);
    assert_eq!(
        deck.harness.tree.node(deck.region()).numeric_value(),
        Some(2.0)
    );
    assert!(deck.act(Role::Button, "Page 4", Action::Click));
    assert_eq!((deck.page, deck.changes), (3, 2));
    assert!(deck
        .harness
        .tree
        .node(deck.harness.tree.expect(Role::Button, "Next"))
        .is_disabled());
    assert!(
        !deck.act(Role::Button, "Next", Action::Click),
        "nothing after the last page"
    );
    assert!(deck.act(Role::Button, "Previous", Action::Click));
    assert_eq!((deck.page, deck.changes), (2, 3));
    // The pointer on the same button does the same.
    let at = logical(
        &deck.harness,
        deck.harness.tree.expect(Role::Button, "Previous"),
    )
    .center();
    click(&mut deck.harness.context, at);
    deck.settle();
    assert_eq!((deck.page, deck.changes), (1, 4));
    // The page that came to the front is the one whose controls answer.
    assert!(deck.act(Role::Button, "Open 1", Action::Click));
    assert_eq!(deck.opened, [1]);
    assert_eq!(deck.changes, 4);
    assert_eq!(deck.pass(), None);
}

#[test]
fn increment_decrement_and_a_page_number_act_like_the_arrow_keys() {
    let mut by_request = Deck::new(5);
    let region = by_request.region();
    assert!(by_request.harness.act(region, Action::Increment));
    by_request.settle();
    assert_eq!((by_request.page, by_request.changes), (1, 1));

    let mut by_key = Deck::new(5);
    let region_by_key = by_key.region();
    assert!(by_key.harness.act(region_by_key, Action::Focus));
    by_key.pass();
    assert_eq!(by_key.harness.tree.focus(), region_by_key);
    press(&mut by_key.harness.context, KeyCode::ArrowRight);
    by_key.settle();
    assert_eq!(
        (by_key.page, by_key.changes),
        (by_request.page, by_request.changes)
    );

    assert!(by_request.harness.act(region, Action::Decrement));
    by_request.settle();
    assert_eq!((by_request.page, by_request.changes), (0, 2));
    assert!(
        by_request.harness.act(region, Action::Decrement),
        "accepted; already at the start"
    );
    by_request.settle();
    assert_eq!((by_request.page, by_request.changes), (0, 2));

    let set = |deck: &mut Deck, value: f64| {
        deck.harness
            .act_with(region, Action::SetValue, ActionData::NumericValue(value));
        deck.settle();
        deck.page
    };
    assert_eq!(set(&mut by_request, 4.0), 3, "page numbers start at one");
    assert_eq!(set(&mut by_request, 99.0), 4);
    assert_eq!(set(&mut by_request, -3.0), 0);
    let changes = by_request.changes;
    assert_eq!(changes, 5);
    for bad in [f64::NAN, f64::INFINITY] {
        assert_eq!(set(&mut by_request, bad), 0);
    }
    assert_eq!(by_request.changes, changes);
    assert_eq!(by_request.exposed(), ["Page 1"]);
    assert_eq!(by_request.pass(), None);
}

#[test]
fn a_slider_of_images_and_a_page_count_are_described_the_same_way() {
    let mut deck = Deck::new(30);
    deck.images = true;
    deck.settle();
    // Too many pages for dots: the indicator is the text "1 / 30".
    assert!(deck.harness.tree.find(Role::Label, "1 / 30").is_some());
    assert!(deck.harness.tree.find(Role::Button, "Page 2").is_none());
    assert_eq!(deck.exposed(), ["Page 1"]);
    assert!(deck.act(Role::Button, "Next", Action::Click));
    assert_eq!(deck.page, 1);
    assert!(deck.harness.tree.find(Role::Label, "2 / 30").is_some());
    assert_eq!(deck.exposed(), ["Page 2"]);
    let neighbours = children(&deck.harness, deck.region(), Role::Group);
    assert!(
        neighbours.len() > 1,
        "neighbouring slides are built: {}",
        neighbours.len()
    );
    assert_eq!(deck.pass(), None);
}

#[test]
fn a_disabled_carousel_refuses_requests() {
    let mut deck = Deck::new(4);
    deck.enabled = false;
    deck.settle();
    let region = deck.region();
    assert!(deck.harness.tree.node(region).is_disabled());
    assert!(!deck.harness.act(region, Action::Increment));
    for name in ["Next", "Page 3"] {
        let button = deck.harness.tree.expect(Role::Button, name);
        assert!(deck.harness.tree.node(button).is_disabled(), "{name}");
        assert!(!deck.harness.act(button, Action::Click));
    }
    deck.settle();
    assert_eq!((deck.page, deck.changes), (0, 0));
}
