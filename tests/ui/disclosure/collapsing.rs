use super::*;

#[test]
fn collapsing_default_controlled_keyboard_actions_and_focus_return() {
    let mut c = setup();
    let mut open = false;
    let mut body_runs = 0;
    let mut action_runs = 0;
    let mut render = |c: &mut Context, controlled: Option<&mut bool>| {
        let mut out = None;
        c.run(|c| {
            Root::new().show(c, |ui| {
                let mut header = CollapsingHeader::new("settings", "Same").default_open(true);
                if let Some(open) = controlled {
                    header = header.open(open);
                }
                out = Some(header.show_with_actions(
                    ui,
                    |ui| {
                        let r = ui.button("Reset");
                        if r.clicked() {
                            action_runs += 1;
                        }
                        r
                    },
                    |ui| {
                        body_runs += 1;
                        ui.button("Child")
                    },
                ));
            });
        });
        out.unwrap()
    };
    let out = render(&mut c, None);
    let body_id = out.body.unwrap().id;
    c.request_focus(body_id);
    let out = render(&mut c, Some(&mut open));
    assert!(!out.open);
    assert!(out.body.is_none());
    assert!(!out.changed);
    assert_eq!(c.probe().focused_widget, Some(out.header_response.id));
    key(&mut c, KeyCode::Space);
    let out = render(&mut c, Some(&mut open));
    assert!(out.open && out.changed && open);
    let action_rect = c.visual_rect(out.actions.id, out.actions.rect);
    click(&mut c, action_rect);
    let out = render(&mut c, Some(&mut open));
    assert!(out.open && !out.changed);
    click(&mut c, out.header_response.rect);
    assert!(!render(&mut c, None).open);
    assert!(!render(&mut c, None).open); // Repeated default(true) does not reset.
    drop(render);
    assert_eq!(action_runs, 1);
    assert_eq!(body_runs, 3);
}

#[test]
fn collapsing_hidden_body_once_dynamic_height_animation_and_cleanup() {
    let mut c = setup();
    let mut style = c.style().clone();
    style.motion.reduced_motion = false;
    c.set_style(style);
    let start = c.frame_time();
    let mut open = false;
    let mut runs = 0;
    let mut render = |c: &mut Context, open: &mut bool, height: f32, time| {
        let mut body = None;
        c.run_at(time, |c| {
            Root::new().show(c, |ui| {
                CollapsingHeader::new("animated", "Header")
                    .open(open)
                    .show(ui, |ui| {
                        runs += 1;
                        body = Some(ui.allocate_space(Vec2::new(100.0, height)));
                    });
                ui.button("Neighbor");
            });
        });
        body
    };
    assert!(render(&mut c, &mut open, 80.0, start).is_none());
    open = true;
    render(&mut c, &mut open, 80.0, start);
    render(
        &mut c,
        &mut open,
        140.0,
        start + std::time::Duration::from_millis(80),
    );
    open = false;
    render(
        &mut c,
        &mut open,
        140.0,
        start + std::time::Duration::from_millis(100),
    );
    assert_eq!(
        c.probe()
            .previous_hits
            .iter()
            .filter(|h| h.action == HitAction::Activate)
            .count(),
        2
    );
    open = true;
    render(
        &mut c,
        &mut open,
        100.0,
        start + std::time::Duration::from_millis(130),
    );
    open = false;
    render(
        &mut c,
        &mut open,
        100.0,
        start + std::time::Duration::from_secs(1),
    );
    assert!(render(
        &mut c,
        &mut open,
        100.0,
        start + std::time::Duration::from_secs(2)
    )
    .is_none());
    drop(render);
    assert_eq!(runs, 5);
    assert!(!c.needs_repaint_at(start + std::time::Duration::from_secs(2)));
    c.run(|_| {});
    assert!(c.probe().counts.collapsing_headers == 0);
    assert!(!c.wants_animation_frame());
}

#[test]
fn collapsing_locked_and_nested_stable_captions() {
    let mut c = setup();
    let mut outer = true;
    let mut nested_open = None;
    let mut render = |c: &mut Context, outer: &mut bool| {
        c.run(|c| {
            Root::new().show(c, |ui| {
                CollapsingHeader::new("outer", "Same")
                    .open(outer)
                    .show(ui, |ui| {
                        let out = CollapsingHeader::new("inner", "Same")
                            .default_open(true)
                            .show(ui, |ui| ui.button("Child"));
                        nested_open = Some(out.open);
                    });
                let out = CollapsingHeader::new("locked", "Same")
                    .default_open(true)
                    .expandable(false)
                    .show(ui, |ui| ui.button("Available"));
                assert!(out.body.unwrap().enabled);
            });
        });
    };
    render(&mut c, &mut outer);
    outer = false;
    render(&mut c, &mut outer);
    outer = true;
    render(&mut c, &mut outer);
    drop(render);
    assert_eq!(nested_open, Some(true));
}
