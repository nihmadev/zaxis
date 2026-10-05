use crate::prelude::*;
use winit::{dpi::PhysicalSize, event::ElementState, keyboard::KeyCode};
use zaxis::{vec2, MenuBar, MenuBarOutput, MenuItem, Rect, Root};

fn setup() -> Context {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(600, 400), 1.0);
    let mut style = c.style().clone();
    style.motion.reduced_motion = true;
    c.set_style(style);
    c
}
fn items() -> Vec<MenuItem> {
    vec![
        MenuItem::submenu(
            "File",
            [
                MenuItem::new("new", "New").shortcut("Ctrl+N"),
                MenuItem::separator(),
                MenuItem::submenu(
                    "Recent",
                    [
                        MenuItem::new("r1", "one"),
                        MenuItem::new("r2", "two").enabled(false),
                        MenuItem::new("r3", "three"),
                    ],
                ),
                MenuItem::new("auto", "Autosave").checked(true),
            ],
        ),
        MenuItem::submenu("Edit", [MenuItem::new("undo", "Undo")]),
        MenuItem::submenu("Empty", []).enabled(false),
    ]
}
/// Two passes: hover and layout follow the pointer on the next pass.
fn draw(c: &mut Context, items: &[MenuItem], compact: bool) -> MenuBarOutput {
    let mut out = MenuBarOutput::default();
    for _ in 0..2 {
        c.run(|c| {
            Root::new().show(c, |ui| {
                let pass = MenuBar::new("menu", items).compact(compact).show(ui);
                out.open = pass.open;
                out.selected = out.selected.or(pass.selected);
            });
        });
    }
    out
}
fn click(c: &mut Context, p: Vec2) {
    c.move_pointer(p);
    c.primary_button(ElementState::Pressed);
    c.primary_button(ElementState::Released);
}
fn key(c: &mut Context, code: KeyCode) {
    assert!(c.on_key_event(code, ElementState::Pressed, false).consumed);
    c.on_key_event(code, ElementState::Released, false);
}
/// Clickable rows of one panel, top to bottom.
fn rows(c: &Context, window: Id) -> Vec<Rect> {
    let mut rows: Vec<Rect> = c
        .probe()
        .previous_hits
        .iter()
        .filter(|h| h.window == window && h.action == HitAction::Activate)
        .map(|h| h.rect)
        .collect();
    rows.sort_by(|a, b| a.min.y.total_cmp(&b.min.y));
    rows
}
fn root(c: &Context) -> Id {
    c.probe().popup.as_ref().expect("a menu is open").id
}
fn panel(c: &Context, level: usize) -> Id {
    c.probe().popup.as_ref().unwrap().extra[level].0
}
/// Triggers are the only Activate hits that are not inside a popup layer.
fn titles(c: &Context) -> Vec<Rect> {
    let mut hits: Vec<Rect> = c
        .probe()
        .previous_hits
        .iter()
        .filter(|h| {
            h.action == HitAction::Activate
                && c.probe().popup.as_ref().is_none_or(|p| {
                    h.window != p.id && !p.extra.iter().any(|(layer, _)| *layer == h.window)
                })
        })
        .map(|h| h.rect)
        .collect();
    hits.sort_by(|a, b| a.min.x.total_cmp(&b.min.x));
    hits
}
/// Opens the first menu and returns the title rectangles (File, Edit).
fn open_file(c: &mut Context, items: &[MenuItem]) -> Vec<Rect> {
    draw(c, items, false);
    let titles = titles(c);
    click(c, titles[0].center());
    assert!(draw(c, items, false).open);
    titles
}

#[test]
fn a_title_opens_its_panel_and_an_action_closes_the_menu_once() {
    let mut c = setup();
    let items = items();
    let titles = open_file(&mut c, &items);
    assert_eq!(titles.len(), 2); // the disabled title is not clickable
    let rows = rows(&c, root(&c));
    assert_eq!(rows.len(), 3); // New, Recent, Autosave; the separator is not a row.
    click(&mut c, rows[0].center());
    let out = draw(&mut c, &items, false);
    assert_eq!(out.selected, Some(Id::new("new")));
    assert!(!out.open && c.probe().popup.is_none());
    assert_eq!(draw(&mut c, &items, false).selected, None);
}

#[test]
fn moving_across_titles_switches_and_the_open_title_toggles() {
    let mut c = setup();
    let items = items();
    let t = open_file(&mut c, &items);
    c.move_pointer(t[1].center());
    assert!(draw(&mut c, &items, false).open);
    assert_eq!(rows(&c, root(&c)).len(), 1); // Edit: Undo
    click(&mut c, t[1].center());
    assert!(!draw(&mut c, &items, false).open);
    assert!(c.probe().popup.is_none());
}

#[test]
fn hovering_opens_a_submenu_and_pressing_inside_it_is_not_outside() {
    let mut c = setup();
    let items = items();
    open_file(&mut c, &items);
    let parent = rows(&c, root(&c));
    c.move_pointer(parent[1].center()); // Recent
    draw(&mut c, &items, false);
    assert_eq!(c.probe().popup.as_ref().unwrap().extra.len(), 1);
    let sub = rows(&c, panel(&c, 0));
    assert_eq!(sub.len(), 2); // the disabled row is a block, not an activation
    assert!(
        sub[0].min.x > parent[1].min.x,
        "the submenu opens beside its parent"
    );
    // Disabled rows do nothing and keep the menu open.
    let disabled = (sub[0].center() + sub[1].center()) * 0.5;
    click(&mut c, disabled);
    assert!(draw(&mut c, &items, false).open);
    click(&mut c, sub[1].center());
    let out = draw(&mut c, &items, false);
    assert_eq!(out.selected, Some(Id::new("r3")));
    assert!(c.probe().popup.is_none());
}

#[test]
fn leaving_a_submenu_row_for_a_sibling_closes_it() {
    let mut c = setup();
    let items = items();
    open_file(&mut c, &items);
    let parent = rows(&c, root(&c));
    c.move_pointer(parent[1].center());
    draw(&mut c, &items, false);
    assert_eq!(c.probe().popup.as_ref().unwrap().extra.len(), 1);
    c.move_pointer(parent[2].center());
    draw(&mut c, &items, false);
    assert!(c.probe().popup.as_ref().unwrap().extra.is_empty());
}

#[test]
fn compact_mode_lists_the_titles_as_submenus() {
    let mut c = setup();
    let items = items();
    draw(&mut c, &items, true);
    let trigger = titles(&c);
    assert_eq!(trigger.len(), 1);
    click(&mut c, trigger[0].center());
    assert!(draw(&mut c, &items, true).open);
    let rows_root = rows(&c, root(&c));
    assert_eq!(rows_root.len(), 2); // Empty is disabled but still a row.
    c.move_pointer(rows_root[1].center()); // Edit
    draw(&mut c, &items, true);
    let sub = rows(&c, panel(&c, 0));
    assert_eq!(sub.len(), 1);
    click(&mut c, sub[0].center());
    assert_eq!(draw(&mut c, &items, true).selected, Some(Id::new("undo")));
}

#[test]
fn keyboard_navigates_levels_and_skips_disabled_rows() {
    let mut c = setup();
    let items = items();
    open_file(&mut c, &items);
    key(&mut c, KeyCode::ArrowDown); // New
    key(&mut c, KeyCode::ArrowDown); // Recent
    key(&mut c, KeyCode::ArrowRight); // enter, highlights "one"
    draw(&mut c, &items, false);
    assert_eq!(c.probe().popup.as_ref().unwrap().extra.len(), 1);
    key(&mut c, KeyCode::ArrowDown); // skips the disabled "two"
    key(&mut c, KeyCode::Enter);
    assert_eq!(draw(&mut c, &items, false).selected, Some(Id::new("r3")));
    assert!(c.probe().popup.is_none());
}

#[test]
fn left_and_right_switch_titles_and_leave_submenus() {
    let mut c = setup();
    let items = items();
    open_file(&mut c, &items);
    key(&mut c, KeyCode::ArrowDown);
    key(&mut c, KeyCode::ArrowDown);
    key(&mut c, KeyCode::ArrowRight);
    draw(&mut c, &items, false);
    key(&mut c, KeyCode::ArrowLeft); // back to the File panel
    draw(&mut c, &items, false);
    assert!(c.probe().popup.as_ref().unwrap().extra.is_empty());
    key(&mut c, KeyCode::ArrowLeft); // wraps over the disabled title to Edit
    draw(&mut c, &items, false);
    assert_eq!(rows(&c, root(&c)).len(), 1);
    key(&mut c, KeyCode::Enter);
    assert_eq!(draw(&mut c, &items, false).selected, Some(Id::new("undo")));
}

#[test]
fn escape_and_outside_presses_close_without_selecting() {
    let mut c = setup();
    let items = items();
    open_file(&mut c, &items);
    key(&mut c, KeyCode::Escape);
    let out = draw(&mut c, &items, false);
    assert!(!out.open && out.selected.is_none());
    let file = titles(&c)[0].center();
    click(&mut c, file);
    assert!(draw(&mut c, &items, false).open);
    click(&mut c, vec2(500.0, 350.0));
    let out = draw(&mut c, &items, false);
    assert!(!out.open && out.selected.is_none());
}

#[test]
fn a_hovered_submenu_row_is_highlighted_in_the_same_pass() {
    let mut c = setup();
    let items = items();
    open_file(&mut c, &items);
    let parent = rows(&c, root(&c));
    c.move_pointer(parent[1].center());
    draw(&mut c, &items, false);
    let layer = panel(&c, 0);
    let hits: Vec<_> = c
        .probe()
        .previous_hits
        .iter()
        .filter(|h| h.window == layer && h.action == HitAction::Activate)
        .copied()
        .collect();
    let painted = |c: &Context, id: Id| {
        c.probe()
            .elements
            .iter()
            .find(|e| e.id == id.with("body"))
            .is_some_and(|e| !e.mesh.vertices.is_empty())
    };
    assert!(!painted(&c, hits[0].id) && !painted(&c, hits[1].id));
    c.move_pointer(hits[1].rect.center());
    // One pass, like a single redraw after a CursorMoved event.
    c.run(|c| {
        Root::new().show(c, |ui| {
            MenuBar::new("menu", &items).show(ui);
        });
    });
    assert!(
        painted(&c, hits[1].id),
        "the hovered submenu row has a fill"
    );
    assert!(!painted(&c, hits[0].id));
    assert_eq!(c.probe().popup.as_ref().unwrap().extra.len(), 1);
}
