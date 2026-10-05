use super::{children, nameless, set};
use crate::support::*;
use std::time::Duration;

const ICON: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M6 9l6 6 6-6" fill="none" stroke="#fff" stroke-width="2"/></svg>"##;
const NAMES: [&str; 3] = ["General", "Audio", "Video"];

fn tab_bar(ctx: &mut Context, page: &mut usize, changes: &mut u32) {
    Window::new("Test").show(ctx, |ui| {
        let tabs = NAMES.into_iter().enumerate();
        for response in ui.tab_bar(page, tabs) {
            *changes += u32::from(response.changed());
        }
        ui.tab_pages("pages", *page, Vec2::new(300.0, 80.0), |ui, index| {
            ui.button(format!("On page {index}"));
        });
    });
}

fn selected(harness: &Harness) -> Vec<Option<bool>> {
    harness
        .tree
        .all(Role::Tab)
        .iter()
        .map(|id| harness.tree.node(*id).is_selected())
        .collect()
}

#[test]
fn a_tab_bar_is_a_tab_list_whose_tabs_select_on_click() {
    let mut harness = Harness::new();
    let (mut page, mut changes) = (0, 0);
    harness.pass(|ctx| tab_bar(ctx, &mut page, &mut changes));
    let list = harness.tree.all(Role::TabList)[0];
    assert_eq!(
        children(&harness, list),
        NAMES.map(|name| (Role::Tab, name.to_owned()))
    );
    assert_eq!(selected(&harness), [Some(true), Some(false), Some(false)]);
    let audio = harness.tree.expect(Role::Tab, "Audio");
    assert_eq!(set(&harness, audio), (Some(1), Some(3)));
    assert_eq!(
        harness.tree.node(audio).toggled(),
        None,
        "a tab is selected, not pressed"
    );
    assert_eq!(
        harness.pass(|ctx| tab_bar(ctx, &mut page, &mut changes)),
        None
    );

    assert!(harness.act(audio, Action::Click));
    harness.pass(|ctx| tab_bar(ctx, &mut page, &mut changes));
    assert_eq!((page, changes), (1, 1));
    assert_eq!(
        selected(&harness),
        [Some(false), Some(true), Some(false)],
        "the pass that selects a tab deselects the other"
    );
    harness.settle(|ctx| tab_bar(ctx, &mut page, &mut changes));
    assert!(harness.act(audio, Action::Click));
    harness.settle(|ctx| tab_bar(ctx, &mut page, &mut changes));
    assert_eq!(
        (page, changes),
        (1, 1),
        "selecting the selected tab changes nothing"
    );
}

#[test]
fn tab_pages_are_a_panel_named_by_the_selected_tab() {
    let mut harness = Harness::new();
    let (mut page, mut changes) = (0, 0);
    harness.pass(|ctx| tab_bar(ctx, &mut page, &mut changes));
    let general = harness.tree.expect(Role::Tab, "General");
    let panel = harness.tree.expect(Role::TabPanel, "General");
    assert_eq!(harness.tree.node(general).controls(), [panel]);
    assert_eq!(
        children(&harness, panel),
        [(Role::Button, "On page 0".to_owned())]
    );

    let video = harness.tree.expect(Role::Tab, "Video");
    assert!(harness.act(video, Action::Click));
    harness.settle(|ctx| tab_bar(ctx, &mut page, &mut changes));
    // While the pages slide, the one leaving is hidden: one panel is presented.
    let panels = harness.tree.all(Role::TabPanel);
    let shown: Vec<_> = panels
        .iter()
        .filter(|id| !harness.tree.node(**id).is_hidden())
        .collect();
    assert_eq!(shown.len(), 1);
    assert_eq!(harness.tree.name(*shown[0]), "Video");
    assert_eq!(harness.tree.node(video).controls(), [*shown[0]]);
    assert!(harness.tree.node(general).controls().is_empty());

    std::thread::sleep(Duration::from_millis(700));
    harness.settle(|ctx| tab_bar(ctx, &mut page, &mut changes));
    std::thread::sleep(Duration::from_millis(300));
    harness.settle(|ctx| tab_bar(ctx, &mut page, &mut changes));
    assert_eq!(harness.tree.all(Role::TabPanel).len(), 1);
    assert!(harness.tree.find(Role::Button, "On page 2").is_some());
    assert!(harness.tree.find(Role::Button, "On page 0").is_none());
    assert_eq!(
        harness.pass(|ctx| tab_bar(ctx, &mut page, &mut changes)),
        None
    );
}

fn icon_tabs(ctx: &mut Context, page: &mut usize, changes: &mut u32, names: [&str; 3]) {
    Window::new("Test").show(ctx, |ui| {
        let tabs = names
            .into_iter()
            .enumerate()
            .map(|(index, name)| (index, ImageSource::bytes(ICON), name));
        ui.add_enabled_ui(*changes < 100, |ui| {
            if ui.add(IconTabs::new(page, tabs)).changed() {
                *changes += 1;
            }
        });
    });
}

#[test]
fn icon_tabs_are_named_by_their_labels() {
    let mut harness = Harness::new();
    let (mut page, mut changes) = (2, 0);
    harness.pass(|ctx| icon_tabs(ctx, &mut page, &mut changes, NAMES));
    let list = harness.tree.all(Role::TabList)[0];
    assert_eq!(
        children(&harness, list),
        NAMES.map(|name| (Role::Tab, name.to_owned()))
    );
    assert_eq!(
        harness.tree.node(list).orientation(),
        Some(accesskit::Orientation::Vertical)
    );
    assert_eq!(selected(&harness), [Some(false), Some(false), Some(true)]);
    assert_eq!(nameless(&harness), 0);
    let general = harness.tree.expect(Role::Tab, "General");
    assert_eq!(set(&harness, general), (Some(0), Some(3)));
    harness.settle(|ctx| icon_tabs(ctx, &mut page, &mut changes, NAMES));
    assert_eq!(
        harness.pass(|ctx| icon_tabs(ctx, &mut page, &mut changes, NAMES)),
        None
    );

    assert!(harness.act(general, Action::Click));
    harness.pass(|ctx| icon_tabs(ctx, &mut page, &mut changes, NAMES));
    assert_eq!((page, changes), (0, 1));
    assert_eq!(selected(&harness), [Some(true), Some(false), Some(false)]);
    harness.settle(|ctx| icon_tabs(ctx, &mut page, &mut changes, NAMES));
    assert_eq!(changes, 1);

    // Disabled as a group: the request is refused.
    changes = 100;
    harness.settle(|ctx| icon_tabs(ctx, &mut page, &mut changes, NAMES));
    let video = harness.tree.expect(Role::Tab, "Video");
    assert!(harness.tree.node(video).is_disabled());
    assert!(!harness.act(video, Action::Click));
    harness.settle(|ctx| icon_tabs(ctx, &mut page, &mut changes, NAMES));
    assert_eq!((page, changes), (0, 100));
}

#[test]
fn an_icon_tab_without_a_label_is_reported() {
    let mut harness = Harness::new();
    let (mut page, mut changes) = (0, 0);
    harness.pass(|ctx| icon_tabs(ctx, &mut page, &mut changes, ["General", "", "Video"]));
    assert_eq!(nameless(&harness), 1);
}

fn section(ctx: &mut Context, open: &mut bool, changes: &mut u32, enabled: bool) {
    Window::new("Test").show(ctx, |ui| {
        let output = CollapsingHeader::new("advanced", "Advanced")
            .open(open)
            .enabled(enabled)
            .animate_height(false)
            .show_with_actions(
                ui,
                |ui| {
                    ui.button("Reset");
                },
                |ui| {
                    ui.button("Inside");
                },
            );
        *changes += u32::from(output.changed);
    });
}

#[test]
fn a_collapsing_header_expands_and_collapses_on_request() {
    let mut harness = Harness::new();
    let (mut open, mut changes) = (false, 0);
    harness.pass(|ctx| section(ctx, &mut open, &mut changes, true));
    let header = harness.tree.expect(Role::Button, "Advanced");
    assert_eq!(harness.tree.node(header).is_expanded(), Some(false));
    let window = harness.tree.expect(Role::Window, "Test");
    assert_eq!(
        children(&harness, window),
        ["Advanced", "Reset"].map(|name| (Role::Button, name.to_owned())),
        "the header comes before the actions in its row and does not contain them"
    );
    assert_eq!(
        harness.pass(|ctx| section(ctx, &mut open, &mut changes, true)),
        None
    );

    assert!(harness.act(header, Action::Click));
    harness.settle(|ctx| section(ctx, &mut open, &mut changes, true));
    assert_eq!((open, changes), (true, 1));
    assert_eq!(harness.tree.node(header).is_expanded(), Some(true));
    assert!(harness.tree.find(Role::Button, "Inside").is_some());
    assert_eq!(
        harness.pass(|ctx| section(ctx, &mut open, &mut changes, true)),
        None
    );

    // Expanding what is expanded changes nothing.
    harness.act(header, Action::Expand);
    harness.settle(|ctx| section(ctx, &mut open, &mut changes, true));
    assert_eq!((open, changes), (true, 1));
    assert!(harness.act(header, Action::Collapse));
    harness.settle(|ctx| section(ctx, &mut open, &mut changes, true));
    assert_eq!((open, changes), (false, 2));
    assert_eq!(harness.tree.node(header).is_expanded(), Some(false));
    assert!(harness.tree.find(Role::Button, "Inside").is_none());
    harness.act(header, Action::Collapse);
    harness.settle(|ctx| section(ctx, &mut open, &mut changes, true));
    assert_eq!((open, changes), (false, 2));
    assert!(harness.act(header, Action::Expand));
    harness.settle(|ctx| section(ctx, &mut open, &mut changes, true));
    assert_eq!((open, changes), (true, 3));

    harness.settle(|ctx| section(ctx, &mut open, &mut changes, false));
    assert!(harness.tree.node(header).is_disabled());
    for action in [Action::Click, Action::Expand, Action::Collapse] {
        assert!(!harness.act(header, action));
    }
    harness.settle(|ctx| section(ctx, &mut open, &mut changes, false));
    assert_eq!((open, changes), (true, 3));
}

#[test]
fn a_collapsing_header_that_keeps_its_own_state_is_toggled_by_a_click() {
    let mut harness = Harness::new();
    let mut changes = 0;
    let mut build = |ctx: &mut Context| {
        Window::new("Test").show(ctx, |ui| {
            let output = ui.collapsing("more", "More", |ui| {
                ui.label("Details");
            });
            changes += u32::from(output.changed);
        });
    };
    harness.pass(&mut build);
    let header = harness.tree.expect(Role::Button, "More");
    assert!(harness.act(header, Action::Click));
    harness.settle(&mut build);
    assert_eq!(harness.tree.node(header).is_expanded(), Some(true));
    assert!(harness.tree.find(Role::Label, "Details").is_some());
    assert_eq!(changes, 1);
}

#[test]
fn tabs_that_scroll_are_still_one_list_and_a_locked_header_takes_no_click() {
    let mut harness = Harness::new();
    let (mut page, mut locked) = (0_usize, false);
    let names: Vec<String> = (0..12).map(|i| format!("A rather long tab {i}")).collect();
    let mut build = |ctx: &mut Context| {
        Window::new("Test").show(ctx, |ui| {
            ui.tab_bar(&mut page, names.iter().cloned().enumerate());
            CollapsingHeader::new("locked", "Locked")
                .open(&mut locked)
                .expandable(false)
                .show(ui, |_| ());
        });
    };
    harness.pass(&mut build);
    let list = harness.tree.all(Role::TabList)[0];
    assert_eq!(harness.tree.node(list).children().len(), 12);
    let last = harness.tree.expect(Role::Tab, "A rather long tab 11");
    assert_eq!(set(&harness, last), (Some(11), Some(12)));
    // The strip scrolls the tab into view on request; a tab in view takes the click.
    assert!(harness.act(last, Action::ScrollIntoView));
    for _ in 0..3 {
        harness.settle(&mut build);
        std::thread::sleep(Duration::from_millis(150));
    }
    assert!(harness.act(last, Action::Click));
    harness.settle(&mut build);
    assert_eq!(harness.tree.node(last).is_selected(), Some(true));

    let header = harness.tree.expect(Role::Button, "Locked");
    let bounds = harness
        .tree
        .node(header)
        .bounds()
        .expect("the header is placed");
    assert!(bounds.width() > 0.0 && bounds.height() > 0.0, "{bounds:?}");
    assert_eq!(harness.tree.node(header).is_expanded(), Some(false));
    for action in [Action::Click, Action::Expand] {
        assert!(
            !harness.act(header, action),
            "{action:?} on a header that cannot expand"
        );
    }
    harness.settle(&mut build);
    assert_eq!((page, locked), (11, false));
}
