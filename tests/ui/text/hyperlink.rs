use super::support::*;
use winit::{keyboard::ModifiersState, window::CursorIcon};
use zaxis::{Hyperlink, LinkActivation, Underline, UrlError, UrlPolicy};

fn link(c: &mut Context) -> Response {
    frame(c, |ui| ui.hyperlink_to("Docs", "https://example.com"))
}

#[test]
fn click_activates_once_on_release_over_the_same_link() {
    let mut c = setup();
    let r = link(&mut c);
    click(&mut c, r.rect.center());
    let r = link(&mut c);
    assert!(r.clicked());
    assert_eq!(r.link_activation(), Some(LinkActivation::Primary));
    let r = link(&mut c);
    assert!(!r.clicked() && r.link_activation().is_none());
}

#[test]
fn pressing_and_leaving_does_not_activate() {
    let mut c = setup();
    let r = link(&mut c);
    down(&mut c, r.rect.center());
    c.move_pointer(r.rect.max + Vec2::splat(80.0));
    up(&mut c);
    let r = link(&mut c);
    assert!(r.link_activation().is_none());
    // A press and release over empty space never activates it either.
    click(&mut c, r.rect.max + Vec2::splat(80.0));
    assert!(link(&mut c).link_activation().is_none());
}

#[test]
fn enter_and_space_activate_the_focused_link() {
    let mut c = setup();
    link(&mut c);
    key(&mut c, KeyCode::Tab);
    let r = link(&mut c);
    assert!(r.has_focus && r.focus_visible);
    key(&mut c, KeyCode::Enter);
    assert_eq!(
        link(&mut c).link_activation(),
        Some(LinkActivation::Primary)
    );
    key(&mut c, KeyCode::Space);
    assert_eq!(
        link(&mut c).link_activation(),
        Some(LinkActivation::Primary)
    );
    assert!(link(&mut c).link_activation().is_none());
}

#[test]
fn middle_and_ctrl_click_are_the_secondary_activation() {
    let mut c = setup();
    let at = link(&mut c).rect.center();
    middle_click(&mut c, at);
    let r = link(&mut c);
    assert_eq!(r.link_activation(), Some(LinkActivation::Secondary));
    assert!(!r.clicked());
    c.set_modifiers(ModifiersState::CONTROL);
    click(&mut c, at);
    let r = link(&mut c);
    assert_eq!(r.link_activation(), Some(LinkActivation::Secondary));
    assert!(!r.clicked());
}

#[test]
fn a_disabled_link_ignores_input_and_keeps_the_arrow() {
    let mut c = setup();
    let show = |c: &mut Context| {
        frame(c, |ui| {
            ui.add(Hyperlink::new("Docs").url("https://x.org").enabled(false))
        })
    };
    let r = show(&mut c);
    assert!(!r.enabled);
    c.move_pointer(r.rect.center());
    assert_eq!(c.cursor_icon(), CursorIcon::Default);
    click(&mut c, r.rect.center());
    middle_click(&mut c, r.rect.center());
    key(&mut c, KeyCode::Tab);
    key(&mut c, KeyCode::Enter);
    let r = show(&mut c);
    assert!(r.link_activation().is_none() && !r.has_focus);
}

#[test]
fn the_pointer_is_a_hand_over_an_enabled_link_only() {
    let mut c = setup();
    let r = link(&mut c);
    c.move_pointer(r.rect.center());
    assert_eq!(c.cursor_icon(), CursorIcon::Pointer);
    c.move_pointer(r.rect.max + Vec2::splat(60.0));
    assert_eq!(c.cursor_icon(), CursorIcon::Default);
}

fn wrapped(c: &mut Context, width: f32) -> zaxis::HyperlinkOutput {
    frame(c, |ui| {
        ui.with_width(width, |ui| {
            Hyperlink::new("a hyperlink that is long enough to wrap onto several lines")
                .url("https://example.com")
                .show(ui)
        })
    })
}

#[test]
fn a_wrapped_link_has_a_hit_area_per_line_and_none_between() {
    let mut c = setup();
    let out = wrapped(&mut c, 120.0);
    wrapped(&mut c, 120.0);
    let id = out.response.id;
    let hits: Vec<_> = c
        .probe()
        .previous_hits
        .iter()
        .filter(|h| h.id == id)
        .map(|h| h.rect)
        .collect();
    assert!(hits.len() >= 2, "fragments: {hits:?}");
    for pair in hits.windows(2) {
        assert!(
            pair[1].min.y >= pair[0].max.y - 0.01,
            "stacked, not overlapping"
        );
    }
    let first = hits[0];
    c.move_pointer(first.center());
    assert_eq!(c.cursor_icon(), CursorIcon::Pointer);
    let last = *hits.last().unwrap();
    c.move_pointer(Vec2::new(last.max.x + 20.0, last.center().y));
    assert_eq!(c.cursor_icon(), CursorIcon::Default);
    // A press on one line and a release on the next is a click on the same link.
    down(&mut c, first.center());
    c.move_pointer(last.center());
    up(&mut c);
    assert_eq!(
        wrapped(&mut c, 120.0).activated,
        Some(LinkActivation::Primary)
    );
}

fn decorations(c: &Context, id: Id) -> usize {
    c.probe()
        .cache
        .get(&id.with("deco"))
        .map_or(0, |e| e.paint.len())
}

#[test]
fn hover_underlines_and_reduced_motion_is_instant_and_settled() {
    let mut c = setup();
    let show = |c: &mut Context, underline| {
        frame(c, |ui| {
            // No address, so no tooltip timer: the pass is settled once the color is.
            Hyperlink::new("Docs").underline(underline).show(ui)
        })
    };
    let rest = show(&mut c, Underline::Hover);
    let id = rest.response.id;
    assert_eq!(decorations(&c, id), 0, "no underline at rest");
    c.move_pointer(rest.response.rect.center());
    show(&mut c, Underline::Hover);
    assert_eq!(decorations(&c, id), 1, "underline on hover, instantly");
    show(&mut c, Underline::Hover);
    show(&mut c, Underline::Hover);
    assert!(
        c.probe().next_repaint.is_none() && !c.needs_repaint(),
        "a settled link requests no redraw"
    );
    c.move_pointer(Vec2::splat(400.0));
    show(&mut c, Underline::Always);
    assert_eq!(decorations(&c, id), 1);
    show(&mut c, Underline::Never);
    assert_eq!(decorations(&c, id), 0);
}

#[test]
fn urls_are_checked_against_an_allowlist() {
    let policy = UrlPolicy::default();
    assert_eq!(
        policy.check("https://example.com/a?b=1"),
        Ok("https://example.com/a?b=1")
    );
    assert!(policy.check("HTTP://EXAMPLE.COM").is_ok());
    assert!(policy.check("mailto:me@example.com").is_ok());
    for bad in [
        "javascript:alert(1)",
        "file:///etc/passwd",
        "ms-settings:privacy",
        "data:text/html,x",
    ] {
        assert!(
            matches!(policy.check(bad), Err(UrlError::Scheme(_))),
            "{bad}"
        );
    }
    for bad in [
        "",
        "example.com",
        " https://x.org",
        "https://x.org\n",
        "java\tscript:x",
        "1http://x",
        "https:",
        "://x",
    ] {
        assert_eq!(policy.check(bad), Err(UrlError::Malformed), "{bad:?}");
    }
    assert!(policy.clone().allow("tel").check("tel:+123").is_ok());
    assert!(UrlPolicy::only(["ftp"]).check("https://x.org").is_err());
}

#[test]
fn a_refused_scheme_is_reported_not_opened() {
    let mut c = setup();
    let show = |c: &mut Context| {
        frame(c, |ui| {
            Hyperlink::new("Run")
                .url("javascript:alert(1)")
                .open_in_browser()
                .show(ui)
        })
    };
    let at = show(&mut c).response.rect.center();
    click(&mut c, at);
    let out = show(&mut c);
    assert_eq!(out.activated, Some(LinkActivation::Primary));
    assert!(
        c.diagnostics()
            .iter()
            .any(|d| d.message.contains("not allowed")),
        "{:?}",
        c.diagnostics()
    );
}

#[test]
fn the_tooltip_defaults_to_the_full_address() {
    let mut c = setup();
    let now = Instant::now();
    let show = |c: &mut Context, at, text: &'static str, url: Option<&'static str>| {
        frame_at(c, at, |ui| {
            let link = Hyperlink::new(text);
            ui.add(match url {
                Some(url) => link.url(url),
                None => link,
            })
        })
    };
    let r = show(
        &mut c,
        now,
        "Docs",
        Some("https://example.com/full/address"),
    );
    c.move_pointer(r.rect.center());
    show(
        &mut c,
        now,
        "Docs",
        Some("https://example.com/full/address"),
    );
    let later = now + std::time::Duration::from_millis(500);
    show(
        &mut c,
        later,
        "Docs",
        Some("https://example.com/full/address"),
    );
    let shown = c
        .probe()
        .cache
        .get(&r.id.with("tooltip"))
        .expect("tooltip painted");
    assert!(shown.paint.iter().any(|p| matches!(p,
        Paint::Text { text, .. } | Paint::Paragraph { text, .. } if text == "https://example.com/full/address")));
    // Without an address there is nothing to show.
    let plain = show(&mut c, later, "No address", None);
    c.move_pointer(plain.rect.center());
    show(&mut c, later, "No address", None);
    show(
        &mut c,
        later + std::time::Duration::from_millis(500),
        "No address",
        None,
    );
    assert!(!c.probe().cache.contains_key(&plain.id.with("tooltip")));
}
