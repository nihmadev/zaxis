use super::*;

#[test]
fn split_move_close_float_and_return_preserve_panel_ids() {
    let mut s = state();
    assert!(matches!(
        s.split(p(1), p(0), DockSide::Bottom, 0.3).unwrap(),
        DockEvent::Split { .. }
    ));
    assert!(matches!(
        s.move_panel(p(1), p(2), Some(p(3))).unwrap(),
        DockEvent::Moved { .. }
    ));
    assert!(matches!(
        s.float(
            p(1),
            Rect::from_min_size(vec2(20.0, 30.0), vec2(240.0, 180.0))
        )
        .unwrap(),
        DockEvent::Floated { .. }
    ));
    assert_eq!(s.floats.len(), 1);
    s.dock_float(p(1), p(0), Some(DockSide::Left), 0.4).unwrap();
    assert!(s.floats.is_empty());
    s.close(p(1)).unwrap();
    assert_eq!(s.panels().len(), 3);
    assert!(s.validate([p(0), p(2), p(3)]).is_empty());
}
#[test]
fn normalization_repairs_duplicates_empty_groups_nan_and_active_ids() {
    let mut s = DockState {
        root: Some(DockNode::split(
            "split",
            Layout::Horizontal,
            [
                DockChild::new(DockNode::tabs("a", [p(0), p(0)]), f32::NAN),
                DockChild::new(DockNode::tabs("empty", []), -1.0),
                DockChild::new(
                    DockNode::split(
                        "nested",
                        Layout::Horizontal,
                        [DockChild::new(DockNode::tabs("a", [p(1)]), 1.0)],
                    ),
                    2.0,
                ),
            ],
        )),
        focused: Some(p(99)),
        ..Default::default()
    };
    let issues = s.validate([p(0), p(1)]);
    assert!(issues.contains(&DockIssue::DuplicatePanel(p(0))));
    assert!(issues
        .iter()
        .any(|i| matches!(i, DockIssue::InvalidFraction(_))));
    s.normalize();
    assert!(s.validate([p(0), p(1)]).is_empty());
    let DockNode::Split { children, .. } = s.root.as_ref().unwrap() else {
        panic!("two groups")
    };
    assert_eq!(children.len(), 2);
    s.close(p(0)).unwrap();
    assert!(matches!(s.root, Some(DockNode::Tabs { .. })));
    s.close(p(1)).unwrap();
    assert!(s.root.is_none());
}
#[test]
fn invalid_operations_are_transactional_and_missing_panels_are_reported() {
    let mut s = state();
    let before = s.clone();
    assert!(s.split(p(1), p(90), DockSide::Right, 0.5).is_err());
    assert!(s.split(p(1), p(0), DockSide::Right, f32::NAN).is_err());
    assert!(s.move_panel(p(90), p(0), None).is_err());
    assert!(s.close(p(90)).is_err());
    assert!(s
        .float(
            p(1),
            Rect {
                min: vec2(f32::NAN, 0.0),
                max: Vec2::ONE
            }
        )
        .is_err());
    assert_eq!(s, before);
}
#[test]
fn saved_layout_round_trips_active_shares_windows_and_skips_missing_data() {
    let mut s = state();
    s.activate(p(1)).unwrap();
    s.float(
        p(3),
        Rect::from_min_size(vec2(150.0, 90.0), vec2(320.0, 210.0)),
    )
    .unwrap();
    let saved = s.save();
    assert_eq!(DockState::load(&saved, s.panels()).unwrap(), s);
    let loaded = DockState::load(&saved, [p(0), p(1)]).unwrap();
    assert!(loaded.floats.is_empty());
    assert_eq!(loaded.panels(), [p(0), p(1)]);
    assert_eq!(
        loaded.focused,
        Some(p(1)),
        "fallback focus follows the surviving selected tab"
    );
    assert!(loaded.validate([p(0), p(1)]).is_empty());
}
#[test]
fn arbitrary_saved_bytes_and_excessive_counts_do_not_panic() {
    for input in [
        "",
        "zaxis-dock 1 - H 0 99999999999",
        "zaxis-dock 1 - T 0 - 1 0 0 extra",
        "zaxis-dock 1 - H 0 2 NaN E 1 E 0",
        "💥\0\n",
    ] {
        assert!(DockState::load(input, [p(0)]).is_err());
    }
    for n in 0..512_u32 {
        let text: String = (0..64)
            .map(|i| char::from_u32(32 + (n * 17 + i * 31) % 96).unwrap())
            .collect();
        let _ = DockState::load(&text, []);
    }
}

#[test]
fn enormous_depth_and_fraction_ranges_normalize_without_stack_overflow() {
    let mut node = DockNode::tabs("deep", [p(0)]);
    for n in 0..4096 {
        node = DockNode::split(n, Layout::Horizontal, [DockChild::new(node, 1.0)]);
    }
    let mut state = DockState {
        root: Some(node),
        ..Default::default()
    };
    assert!(state.contains(p(0)));
    assert_eq!(state.panels(), [p(0)]);
    assert!(state.validate([p(0)]).contains(&DockIssue::TooDeep));
    assert!(DockState::load(&state.save(), [p(0)]).is_ok());
    state.normalize();
    assert!(state.root.is_none());
    let mut state = DockState::new(DockNode::split(
        "extreme",
        Layout::Horizontal,
        [
            DockChild::new(DockNode::tabs("small", [p(0)]), f32::MIN_POSITIVE),
            DockChild::new(DockNode::tabs("large", [p(1)]), f32::MAX),
        ],
    ));
    state.normalize();
    assert!(state.validate([p(0), p(1)]).is_empty());
}
