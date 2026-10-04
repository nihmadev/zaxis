use super::registry::{Closed, OpenRequest, Reconcile, Registry};
use super::{ExitPolicy, WindowKey};

fn key(name: &str) -> WindowKey {
    WindowKey::new(name)
}

fn registry(policy: ExitPolicy) -> Registry<u32> {
    let mut registry = Registry::new(WindowKey::main(), policy);
    assert_eq!(registry.request(&WindowKey::main(), None, false), OpenRequest::Queued);
    registry.attach(&WindowKey::main(), 1);
    registry
}

fn open(registry: &mut Registry<u32>, name: &str, parent: Option<&str>, id: u32) {
    let parent = parent.map(key);
    assert_eq!(registry.request(&key(name), parent.as_ref(), false), OpenRequest::Queued);
    registry.attach(&key(name), id);
}

fn names(closed: &Closed) -> Vec<&str> {
    let mut names: Vec<_> = closed.keys.iter().map(WindowKey::as_str).collect();
    names.sort_unstable();
    names
}

#[test]
fn a_key_is_one_window_however_often_it_is_requested() {
    let mut registry = registry(ExitPolicy::MainWindow);
    open(&mut registry, "inspector", None, 2);
    assert_eq!(registry.len(), 2);
    for _ in 0..3 {
        assert_eq!(registry.request(&key("inspector"), None, false), OpenRequest::Exists);
    }
    // A second request before the window is attached is just as harmless.
    assert_eq!(registry.request(&key("palette"), None, false), OpenRequest::Queued);
    assert_eq!(registry.request(&key("palette"), None, false), OpenRequest::Exists);
    assert_eq!(registry.len(), 3);
    assert!(!registry.is_open(&key("palette")));
    assert!(registry.is_open(&key("inspector")));
}

#[test]
fn events_are_routed_by_window_id_and_unknown_ids_belong_to_nobody() {
    let mut registry = registry(ExitPolicy::MainWindow);
    open(&mut registry, "a", None, 2);
    open(&mut registry, "b", None, 3);
    assert_eq!(registry.route(1), Some(&WindowKey::main()));
    assert_eq!(registry.route(2), Some(&key("a")));
    assert_eq!(registry.route(3), Some(&key("b")));
    assert_eq!(registry.route(99), None);
    registry.close(&key("a"));
    assert_eq!(registry.route(2), None, "a closed window receives no events");
    assert_eq!(registry.route(3), Some(&key("b")));
}

#[test]
fn closing_a_secondary_window_closes_only_it_and_its_children() {
    let mut registry = registry(ExitPolicy::MainWindow);
    open(&mut registry, "editor", None, 2);
    open(&mut registry, "inspector", Some("editor"), 3);
    open(&mut registry, "tooltip", Some("inspector"), 4);
    open(&mut registry, "settings", None, 5);
    let closed = registry.close(&key("editor"));
    assert_eq!(names(&closed), ["editor", "inspector", "tooltip"]);
    assert!(!closed.exit);
    assert!(registry.contains(&key("settings")) && registry.contains(&WindowKey::main()));
    assert_eq!(registry.len(), 2);
}

#[test]
fn closing_twice_is_a_no_op() {
    let mut registry = registry(ExitPolicy::MainWindow);
    open(&mut registry, "a", None, 2);
    assert_eq!(names(&registry.close(&key("a"))), ["a"]);
    assert_eq!(registry.close(&key("a")), Closed::default());
    assert_eq!(registry.close(&key("never-opened")), Closed::default());
    assert_eq!(registry.len(), 1);
}

#[test]
fn closing_main_ends_the_application_and_every_window_under_main_window_policy() {
    let mut registry = registry(ExitPolicy::MainWindow);
    open(&mut registry, "a", None, 2);
    open(&mut registry, "b", Some("a"), 3);
    let closed = registry.close(&WindowKey::main());
    assert_eq!(names(&closed), ["a", "b", "main"]);
    assert!(closed.exit);
    assert_eq!(registry.len(), 0);
}

#[test]
fn last_window_policy_keeps_unrelated_windows_alive_until_the_last_closes() {
    let mut registry = registry(ExitPolicy::LastWindow);
    open(&mut registry, "child", Some("main"), 2);
    open(&mut registry, "other", None, 3);
    let closed = registry.close(&WindowKey::main());
    assert_eq!(names(&closed), ["child", "main"], "children go with their parent");
    assert!(!closed.exit);
    assert_eq!(registry.route(3), Some(&key("other")));
    assert!(registry.close(&key("other")).exit);
}

#[test]
fn a_window_needs_an_open_parent() {
    let mut registry = registry(ExitPolicy::MainWindow);
    let ghost = key("ghost");
    assert_eq!(
        registry.request(&key("child"), Some(&ghost), false),
        OpenRequest::UnknownParent
    );
    assert!(!registry.contains(&key("child")));
}

#[test]
fn declarations_open_missing_windows_and_close_the_ones_no_longer_declared() {
    let mut registry = registry(ExitPolicy::MainWindow);
    open(&mut registry, "imperative", None, 2);
    let declare = |names: &[&str]| -> Vec<_> { names.iter().map(|n| (key(n), None)).collect() };
    let plan = registry.reconcile(&declare(&["tools", "inspector"]));
    assert_eq!(plan.open, [key("tools"), key("inspector")]);
    assert!(plan.close.is_empty());
    for (n, name) in ["tools", "inspector"].into_iter().enumerate() {
        registry.request(&key(name), None, true);
        registry.attach(&key(name), 10 + n as u32);
    }
    assert_eq!(registry.reconcile(&declare(&["tools", "inspector"])), Reconcile::default());
    let plan = registry.reconcile(&declare(&["tools"]));
    assert_eq!(plan.close, [key("inspector")], "imperative windows are never touched");
    assert!(plan.open.is_empty());
}

#[test]
fn a_declared_window_the_user_closed_stays_closed_until_undeclared_once() {
    let mut registry = registry(ExitPolicy::MainWindow);
    let declared = vec![(key("tools"), None)];
    registry.request(&key("tools"), None, true);
    registry.attach(&key("tools"), 2);
    registry.close(&key("tools"));
    assert_eq!(registry.reconcile(&declared), Reconcile::default(), "no reopen behind the app's back");
    assert_eq!(registry.reconcile(&declared), Reconcile::default());
    assert_eq!(registry.reconcile(&[]), Reconcile::default());
    assert_eq!(registry.reconcile(&declared).open, [key("tools")]);
}

#[test]
fn duplicate_declarations_open_one_window() {
    let mut registry = registry(ExitPolicy::MainWindow);
    let declared = vec![(key("a"), None), (key("a"), None)];
    assert_eq!(registry.reconcile(&declared).open, [key("a")]);
}

#[test]
fn suspension_detaches_native_windows_but_keeps_keys_and_parents() {
    let mut registry = registry(ExitPolicy::MainWindow);
    open(&mut registry, "a", Some("main"), 2);
    registry.detach_all();
    assert_eq!(registry.route(1), None);
    assert!(registry.contains(&key("a")) && !registry.is_open(&key("a")));
    registry.attach(&key("a"), 7);
    assert_eq!(registry.route(7), Some(&key("a")));
}
