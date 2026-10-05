//! Links: a `Hyperlink` on its own and the links inside rich text.

use super::*;

#[derive(Default)]
struct Clicks {
    clicked: u32,
    primary: u32,
    secondary: u32,
}

impl Clicks {
    fn count(&mut self, response: Response) {
        self.clicked += u32::from(response.clicked());
        match response.link_activation() {
            Some(LinkActivation::Primary) => self.primary += 1,
            Some(LinkActivation::Secondary) => self.secondary += 1,
            None => {}
        }
    }
}

fn docs(context: &mut Context, clicks: &mut Clicks, link: impl FnOnce(Hyperlink) -> Hyperlink) {
    Window::new("Links").show(context, |ui| {
        ui.label("See also");
        let link = link(Hyperlink::new("Documentation").url("https://example.com/docs"));
        clicks.count(ui.add(link));
    });
}

#[test]
fn a_hyperlink_is_one_link_node_with_its_text_and_address() {
    let mut harness = Harness::new();
    let mut clicks = Clicks::default();
    harness.pass(|ctx| docs(ctx, &mut clicks, |link| link));
    let link = harness.tree.expect(Role::Link, "Documentation");
    let node = harness.tree.node(link);
    assert_eq!(node.url(), Some("https://example.com/docs"));
    assert!(!node.is_visited() && !node.is_disabled());
    assert!(node.supports_action(Action::Click) && node.supports_action(Action::Focus));
    assert!(
        node.children().is_empty(),
        "the link is the whole widget: no label around it"
    );
    assert_eq!(
        harness.tree.all(Role::Label).len(),
        1,
        "only the plain label beside it"
    );
    assert_eq!(
        harness.tree.parent(link),
        Some(harness.tree.expect(Role::Window, "Links"))
    );
    assert_eq!(missing_names(&harness.context), 0);
    assert_eq!(
        harness.pass(|ctx| docs(ctx, &mut clicks, |link| link)),
        None
    );

    harness.pass(|ctx| docs(ctx, &mut clicks, |link| link.visited(true)));
    assert!(harness.tree.node(link).is_visited());
}

#[test]
fn a_click_request_activates_a_hyperlink_once_like_enter_does() {
    let mut harness = Harness::new();
    let mut clicks = Clicks::default();
    harness.pass(|ctx| docs(ctx, &mut clicks, |link| link));
    let link = harness.tree.expect(Role::Link, "Documentation");
    assert!(harness.act(link, Action::Click));
    harness.settle(|ctx| docs(ctx, &mut clicks, |link| link));
    assert_eq!(
        (clicks.clicked, clicks.primary, clicks.secondary),
        (1, 1, 0)
    );

    // The keyboard path: Tab reaches the link, Enter activates it the same way.
    harness
        .context
        .on_key_event(KeyCode::Tab, ElementState::Pressed, false);
    harness
        .context
        .on_key_event(KeyCode::Tab, ElementState::Released, false);
    harness.pass(|ctx| docs(ctx, &mut clicks, |link| link));
    assert_eq!(harness.tree.focus(), link, "a link is a focus stop");
    harness
        .context
        .on_key_event(KeyCode::Enter, ElementState::Pressed, false);
    harness
        .context
        .on_key_event(KeyCode::Enter, ElementState::Released, false);
    harness.settle(|ctx| docs(ctx, &mut clicks, |link| link));
    assert_eq!(
        (clicks.clicked, clicks.primary, clicks.secondary),
        (2, 2, 0)
    );

    // A focus request moves the keyboard focus to the link.
    harness.context.set_focus(None);
    harness.pass(|ctx| docs(ctx, &mut clicks, |link| link));
    assert!(harness.act(link, Action::Focus));
    harness.pass(|ctx| docs(ctx, &mut clicks, |link| link));
    assert_eq!(harness.tree.focus(), link);
    assert_eq!(clicks.clicked, 2, "focus is not activation");
}

#[test]
fn a_disabled_hyperlink_refuses_clicks() {
    let mut harness = Harness::new();
    let mut clicks = Clicks::default();
    harness.pass(|ctx| docs(ctx, &mut clicks, |link| link.enabled(false)));
    let link = harness.tree.expect(Role::Link, "Documentation");
    let node = harness.tree.node(link);
    assert!(node.is_disabled());
    assert!(!node.supports_action(Action::Click) && !node.supports_action(Action::Focus));
    assert!(!harness.act(link, Action::Click));
    harness.settle(|ctx| docs(ctx, &mut clicks, |link| link.enabled(false)));
    assert_eq!((clicks.clicked, clicks.primary), (0, 0));
}

#[test]
fn a_shortened_or_selectable_hyperlink_keeps_its_whole_name() {
    let text = "A very long link caption that does not fit the column";
    let mut harness = Harness::new();
    let build = |ctx: &mut Context| {
        Window::new("Links").show(ctx, |ui| {
            ui.with_width(120.0, |ui| ui.add(Hyperlink::new(text).truncate(true)));
        });
    };
    harness.pass(build);
    let link = harness.tree.expect(Role::Link, text);
    assert!(harness.tree.node(link).supports_action(Action::Click));
    assert_eq!(harness.pass(build), None);

    // Selectable: the text is a label that can be selected, the link its child.
    let mut harness = Harness::new();
    let build = |ctx: &mut Context| {
        Window::new("Links").show(ctx, |ui| {
            ui.add(
                Hyperlink::new("Terms")
                    .url("https://example.com/terms")
                    .selectable(true),
            );
        });
    };
    harness.pass(build);
    let label = harness.tree.expect(Role::Label, "Terms");
    let link = harness.tree.expect(Role::Link, "Terms");
    assert_eq!(harness.tree.parent(link), Some(label));
    check_text(&harness, label, "Terms", true);
    assert!(harness
        .tree
        .node(label)
        .supports_action(Action::SetTextSelection));
}

fn rich() -> RichText {
    RichText::new()
        .text("Read the ")
        .link("guide", "https://example.com/guide")
        .text(" or the ")
        .link_with(
            "API reference",
            LinkTarget::new()
                .url("https://example.com/api")
                .visited(true),
        )
        .text(", not the ")
        .link_with(
            "old manual",
            LinkTarget::new().enabled(false).tooltip("Retired"),
        )
        .bold(" at all.")
}

fn rich_label(context: &mut Context, width: f32, activations: &mut Vec<(usize, LinkActivation)>) {
    Window::new("Links").show(context, |ui| {
        ui.with_width(width, |ui| {
            let output = ui.rich_label(rich());
            for link in &output.links {
                activations.extend(link.activation.map(|kind| (link.index, kind)));
            }
            if let Some(link) = output.activated() {
                assert!(link.activation.is_some() && link.response.clicked());
            }
        });
    });
}

#[test]
fn links_of_rich_text_are_children_of_its_label() {
    let mut harness = Harness::new();
    let mut seen = Vec::new();
    harness.pass(|ctx| rich_label(ctx, 600.0, &mut seen));
    let text = rich();
    let label = harness.tree.expect(Role::Label, text.as_str());
    check_text(&harness, label, text.as_str(), true);
    let links = harness.tree.all(Role::Link);
    let names: Vec<_> = links.iter().map(|id| harness.tree.name(*id)).collect();
    assert_eq!(names, ["guide", "API reference", "old manual"]);
    let children = harness.tree.node(label).children();
    assert_eq!(
        children[children.len() - 3..],
        links[..],
        "after the text runs, in order"
    );
    let node = |i: usize| harness.tree.node(links[i]);
    assert_eq!(node(0).url(), Some("https://example.com/guide"));
    assert!(!node(0).is_visited() && node(1).is_visited());
    assert!(node(2).is_disabled() && node(2).url().is_none());
    assert_eq!(node(2).description(), Some("Retired"));
    assert!(!node(2).supports_action(Action::Click));
    // Each link lies inside the label, on the line of its text.
    let area = bounds(harness.tree.node(label));
    for link in &links {
        let inner = bounds(harness.tree.node(*link));
        assert!(inner.x0 >= area.x0 - 1.0 && inner.x1 <= area.x1 + 1.0 && inner.x1 > inner.x0);
        assert!(inner.y0 >= area.y0 - 1.0 && inner.y1 <= area.y1 + 1.0);
    }
    assert_eq!(missing_names(&harness.context), 0);
    assert_eq!(harness.pass(|ctx| rich_label(ctx, 600.0, &mut seen)), None);
}

#[test]
fn a_click_request_activates_exactly_the_link_it_names() {
    let mut harness = Harness::new();
    let mut seen = Vec::new();
    harness.pass(|ctx| rich_label(ctx, 600.0, &mut seen));
    let second = harness.tree.expect(Role::Link, "API reference");
    assert!(harness.act(second, Action::Click));
    harness.settle(|ctx| rich_label(ctx, 600.0, &mut seen));
    assert_eq!(
        seen,
        [(1, LinkActivation::Primary)],
        "one request, one activation"
    );
    let disabled = harness.tree.expect(Role::Link, "old manual");
    assert!(!harness.act(disabled, Action::Click));
    harness.settle(|ctx| rich_label(ctx, 600.0, &mut seen));
    assert_eq!(seen.len(), 1);

    // Tab walks the label, then its enabled links, in order.
    let mut stops = Vec::new();
    for _ in 0..3 {
        harness
            .context
            .on_key_event(KeyCode::Tab, ElementState::Pressed, false);
        harness
            .context
            .on_key_event(KeyCode::Tab, ElementState::Released, false);
        harness.pass(|ctx| rich_label(ctx, 600.0, &mut seen));
        let focus = harness.tree.focus();
        stops.push((harness.tree.node(focus).role(), harness.tree.name(focus)));
    }
    assert_eq!(stops[1], (Role::Link, "guide".to_owned()));
    assert_eq!(stops[2], (Role::Link, "API reference".to_owned()));
    assert_eq!(stops[0].0, Role::Label);
}

#[test]
fn a_wrapped_link_is_one_node_over_all_its_fragments() {
    let mut harness = Harness::new();
    let mut seen = Vec::new();
    harness.pass(|ctx| rich_label(ctx, 90.0, &mut seen));
    let text = rich();
    let label = harness.tree.expect(Role::Label, text.as_str());
    let runs = check_text(&harness, label, text.as_str(), true);
    assert!(runs.len() >= 4, "the text wraps: {} lines", runs.len());
    assert_eq!(
        harness.tree.all(Role::Link).len(),
        3,
        "one node per link, not per fragment"
    );
    let line = bounds(&runs[0]);
    let link = bounds(harness.node(Role::Link, "API reference"));
    assert!(
        link.y1 - link.y0 > (line.y1 - line.y0) * 1.5,
        "the bounds span its lines"
    );
    let second = harness.tree.expect(Role::Link, "API reference");
    assert!(harness.act(second, Action::Click));
    harness.settle(|ctx| rich_label(ctx, 90.0, &mut seen));
    assert_eq!(seen, [(1, LinkActivation::Primary)]);
    assert_eq!(harness.pass(|ctx| rich_label(ctx, 90.0, &mut seen)), None);
}

#[test]
fn a_link_hidden_by_the_ellipsis_is_not_offered() {
    let mut harness = Harness::new();
    let text = RichText::new()
        .text("A long introduction that fills the whole line before ")
        .link("the link", "https://example.com");
    let build = |ctx: &mut Context| {
        Window::new("Links").show(ctx, |ui| {
            ui.with_width(120.0, |ui| {
                ui.add(SelectableLabel::rich(text.clone()).truncate(true))
            });
        });
    };
    harness.pass(build);
    let label = harness.tree.expect(Role::Label, text.as_str());
    check_text(&harness, label, text.as_str(), true);
    assert!(
        harness.tree.all(Role::Link).is_empty(),
        "nothing on screen to activate"
    );
    assert_eq!(harness.pass(build), None);
}
