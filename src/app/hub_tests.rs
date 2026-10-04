use super::commands::Command;
use super::hub::Hub;
use super::registry::OpenRequest;
use super::{
    App, CloseRequested, CloseSource, ExitPolicy, Frame, GlobalShortcut, OpenOutcome, WindowError,
    WindowKey, WindowOptions, WindowPlan, WindowStatus, Windows,
};
use crate::{Context, SharedResources};
use std::collections::HashSet;
use winit::keyboard::{KeyCode, ModifiersState};

/// An application written against the original single-window API: `update` only.
struct Plain;
impl App for Plain {
    fn update(&mut self, _: &mut Context, _: &mut Frame<'_>) {}
}

#[derive(Default)]
struct Multi {
    veto: HashSet<&'static str>,
    wanted: Vec<(&'static str, Option<&'static str>)>,
    seen: Vec<(String, CloseSource, bool)>,
    failed: Vec<String>,
    taken: Option<KeyCode>,
    close_siblings_on: Option<&'static str>,
}

impl App for Multi {
    fn update(&mut self, _: &mut Context, _: &mut Frame<'_>) {}

    fn windows(&mut self, plan: &mut WindowPlan) {
        for (key, parent) in &self.wanted {
            let mut options = WindowOptions::new(*key);
            if let Some(parent) = parent {
                options = options.with_parent(*parent);
            }
            plan.window(*key, options);
        }
    }

    fn close_requested(&mut self, request: &mut CloseRequested<'_>) {
        self.seen.push((
            request.window().to_string(),
            request.source(),
            request.is_main(),
        ));
        if self.veto.contains(request.window().as_str()) {
            request.reject();
        }
        if self.close_siblings_on == Some(request.window().as_str()) {
            request.windows().close("b");
        }
    }

    fn window_failed(&mut self, key: &WindowKey, _: &WindowError) {
        self.failed.push(key.to_string());
    }

    fn global_shortcut(
        &mut self,
        shortcut: &GlobalShortcut<'_>,
        windows: &mut Windows<'_>,
    ) -> bool {
        if self.taken == Some(shortcut.key) {
            windows.open("palette", WindowOptions::new("Palette"));
            return true;
        }
        false
    }
}

fn hub(policy: ExitPolicy) -> Hub<u32> {
    let mut hub = Hub::new(WindowKey::main(), policy, SharedResources::new());
    hub.begin_open(&WindowKey::main(), None, false);
    hub.opened(&WindowKey::main(), 1);
    hub
}

fn open(hub: &mut Hub<u32>, name: &str, id: u32) {
    assert_eq!(
        hub.begin_open(&WindowKey::new(name), None, false),
        OpenRequest::Queued
    );
    hub.opened(&WindowKey::new(name), id);
}

#[test]
fn single_window_apps_keep_their_behaviour_closing_the_window_ends_the_run() {
    let mut hub = hub(ExitPolicy::default());
    let outcome = hub.close_requested(&mut Plain, &WindowKey::main(), CloseSource::System);
    assert!(outcome.delivered && !outcome.rejected);
    assert!(outcome.closed.exit);
    assert_eq!(hub.control.stats.windows_open, 0);
}

#[test]
fn rejecting_keeps_the_window_and_a_later_request_asks_again() {
    let mut hub = hub(ExitPolicy::default());
    open(&mut hub, "doc", 2);
    let mut app = Multi {
        veto: HashSet::from(["doc"]),
        ..Default::default()
    };
    let doc = WindowKey::new("doc");
    for _ in 0..2 {
        let outcome = hub.close_requested(&mut app, &doc, CloseSource::System);
        assert!(outcome.rejected && outcome.closed.keys.is_empty() && !outcome.closed.exit);
        assert!(hub.registry.contains(&doc));
    }
    assert_eq!(app.seen.len(), 2, "each user action is one event");
    app.veto.clear();
    let outcome = hub.close_requested(&mut app, &doc, CloseSource::Application);
    assert_eq!(outcome.closed.keys, [doc.clone()]);
    assert_eq!(app.seen[2].1, CloseSource::Application);
    assert!(
        hub.registry.contains(&WindowKey::main()),
        "closing a secondary window keeps the app"
    );
}

#[test]
fn closing_a_window_that_is_already_closed_raises_no_event() {
    let mut hub = hub(ExitPolicy::default());
    open(&mut hub, "doc", 2);
    let mut app = Multi::default();
    let doc = WindowKey::new("doc");
    assert!(
        hub.close_requested(&mut app, &doc, CloseSource::System)
            .delivered
    );
    let again = hub.close_requested(&mut app, &doc, CloseSource::System);
    assert!(!again.delivered && again.closed.keys.is_empty() && !again.closed.exit);
    assert_eq!(app.seen.len(), 1);
    assert!(hub.close(&doc).keys.is_empty());
    assert_eq!(hub.control.stats.windows_closed, 1, "counted once");
}

#[test]
fn the_main_window_is_identified_in_the_request_and_ends_the_app_when_accepted() {
    let mut hub = hub(ExitPolicy::MainWindow);
    open(&mut hub, "doc", 2);
    let mut app = Multi::default();
    let outcome = hub.close_requested(&mut app, &WindowKey::main(), CloseSource::System);
    assert_eq!(app.seen, [("main".to_string(), CloseSource::System, true)]);
    assert!(outcome.closed.exit);
    assert_eq!(outcome.closed.keys.len(), 2);
    assert_eq!(
        hub.control.status.len(),
        0,
        "no status leaks after the windows close"
    );
}

#[test]
fn the_decision_can_close_other_windows_through_the_request() {
    let mut hub = hub(ExitPolicy::MainWindow);
    open(&mut hub, "a", 2);
    open(&mut hub, "b", 3);
    let mut app = Multi {
        close_siblings_on: Some("a"),
        ..Default::default()
    };
    hub.close_requested(&mut app, &WindowKey::new("a"), CloseSource::System);
    let commands = hub.take_commands();
    assert!(matches!(commands.as_slice(), [Command::Close(key)] if key.as_str() == "b"));
}

#[test]
fn opening_a_key_twice_requests_one_window() {
    let mut hub = hub(ExitPolicy::default());
    let mut windows = Windows {
        control: &mut hub.control,
    };
    let key = WindowKey::new("inspector");
    assert_eq!(
        windows.open(&key, WindowOptions::new("Inspector")),
        OpenOutcome::Requested
    );
    assert_eq!(
        windows.open(&key, WindowOptions::new("Inspector")),
        OpenOutcome::AlreadyOpen
    );
    assert_eq!(windows.status(&key), Some(&WindowStatus::Pending));
    assert_eq!(hub.take_commands().len(), 1);
    // Once open it stays a no-op: no second window, no new surface.
    hub.begin_open(&key, None, false);
    hub.opened(&key, 5);
    let mut windows = Windows {
        control: &mut hub.control,
    };
    assert_eq!(
        windows.open(&key, WindowOptions::new("Inspector")),
        OpenOutcome::AlreadyOpen
    );
    assert!(hub.take_commands().is_empty());
    assert_eq!(hub.control.stats.windows_opened, 2);
}

#[test]
fn a_failed_window_is_reported_and_may_be_requested_again_without_touching_others() {
    let mut hub = hub(ExitPolicy::default());
    open(&mut hub, "stable", 2);
    let key = WindowKey::new("broken");
    Windows {
        control: &mut hub.control,
    }
    .open(&key, WindowOptions::new("Broken"));
    hub.begin_open(&key, None, false);
    let closed = hub.open_failed(&key, "no surface".into());
    assert_eq!(closed.keys, [key.clone()]);
    assert!(!closed.exit);
    assert_eq!(
        hub.control.status[&key],
        WindowStatus::Failed("no surface".into())
    );
    assert!(hub.registry.contains(&WindowKey::new("stable")));
    assert_eq!(hub.control.stats.windows_failed, 1);
    assert_eq!(
        Windows {
            control: &mut hub.control
        }
        .open(&key, WindowOptions::new("Broken")),
        OpenOutcome::Requested
    );
}

#[test]
fn declared_windows_open_once_close_when_undeclared_and_stay_closed_after_the_user_closes_them() {
    let mut hub = hub(ExitPolicy::default());
    let mut app = Multi {
        wanted: vec![("tools", None), ("inspector", Some("tools"))],
        ..Default::default()
    };
    hub.declare(&mut app);
    let commands = hub.take_commands();
    let opened: Vec<_> = commands
        .iter()
        .map(|c| match c {
            Command::Open { key, declared, .. } => (key.as_str().to_owned(), *declared),
            _ => panic!("only opens expected"),
        })
        .collect();
    assert_eq!(opened, [("tools".into(), true), ("inspector".into(), true)]);
    // Declaring again before the windows exist must not queue duplicates.
    hub.declare(&mut app);
    assert!(hub.take_commands().is_empty());
    hub.begin_open(&"tools".into(), None, true);
    hub.opened(&"tools".into(), 2);
    hub.begin_open(&"inspector".into(), Some(&"tools".into()), true);
    hub.opened(&"inspector".into(), 3);
    hub.declare(&mut app);
    assert!(hub.take_commands().is_empty());
    // The user closes the parent: the child goes too, and neither reopens by itself.
    hub.close_requested(&mut app, &"tools".into(), CloseSource::System);
    assert_eq!(hub.control.stats.windows_open, 1);
    hub.declare(&mut app);
    assert!(hub.take_commands().is_empty());
    // Undeclared once, then declared again: reopened.
    app.wanted.clear();
    hub.declare(&mut app);
    app.wanted = vec![("tools", None)];
    hub.declare(&mut app);
    assert!(matches!(
        hub.take_commands().as_slice(),
        [Command::Open { .. }]
    ));
}

#[test]
fn undeclaring_a_window_queues_its_close() {
    let mut hub = hub(ExitPolicy::default());
    let mut app = Multi {
        wanted: vec![("tools", None)],
        ..Default::default()
    };
    hub.declare(&mut app);
    hub.take_commands();
    hub.begin_open(&"tools".into(), None, true);
    hub.opened(&"tools".into(), 2);
    app.wanted.clear();
    hub.declare(&mut app);
    assert!(
        matches!(hub.take_commands().as_slice(), [Command::Close(key)] if key.as_str() == "tools")
    );
}

#[test]
fn shortcuts_are_window_local_unless_the_application_takes_them() {
    let mut hub = hub(ExitPolicy::default());
    let main = WindowKey::main();
    let shortcut = |key| GlobalShortcut {
        window: &main,
        key,
        modifiers: ModifiersState::CONTROL,
        repeat: false,
    };
    assert!(!hub.shortcut(&mut Plain, &shortcut(KeyCode::KeyK)));
    let mut app = Multi {
        taken: Some(KeyCode::Comma),
        ..Default::default()
    };
    assert!(!hub.shortcut(&mut app, &shortcut(KeyCode::KeyK)));
    assert!(hub.take_commands().is_empty());
    assert!(hub.shortcut(&mut app, &shortcut(KeyCode::Comma)));
    assert_eq!(hub.take_commands().len(), 1);
}

#[test]
fn window_info_and_keys_are_addressed_by_key() {
    let mut hub = hub(ExitPolicy::default());
    open(&mut hub, "a", 2);
    let windows = Windows {
        control: &mut hub.control,
    };
    let mut keys: Vec<_> = windows.keys().map(WindowKey::to_string).collect();
    keys.sort();
    assert_eq!(keys, ["a", "main"]);
    assert!(windows.is_open(&"a".into()) && !windows.is_open(&"b".into()));
    assert!(
        windows.info(&"a".into()).is_none(),
        "no snapshot before the runner publishes one"
    );
}

#[test]
fn window_keys_compare_by_name() {
    assert_eq!(WindowKey::from("a"), WindowKey::new(String::from("a")));
    assert_ne!(WindowKey::new("a"), WindowKey::new("b"));
    assert_eq!(WindowKey::default(), WindowKey::main());
    assert_eq!(format!("{}", WindowKey::new("x")), "x");
}
