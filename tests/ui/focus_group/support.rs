//! A toolbar of buttons between two outside buttons, driven by real input events.

pub use crate::keys::support::{again, click, down, mods, pass, setup, tap, up};
use crate::prelude::*;
use winit::keyboard::ModifiersState;
use zaxis::{Button, FocusAxis, FocusGroup, Root, Ui};

#[derive(Clone)]
pub struct Opts {
    pub axis: FocusAxis,
    pub wrap: bool,
    pub disabled: Vec<&'static str>,
    pub hidden: Vec<&'static str>,
}

impl Default for Opts {
    fn default() -> Self {
        Self {
            axis: FocusAxis::Horizontal,
            wrap: false,
            disabled: Vec::new(),
            hidden: Vec::new(),
        }
    }
}

pub struct Bar {
    pub before: Response,
    pub after: Response,
    pub items: Vec<(&'static str, Response)>,
    pub group: Response,
    pub stop: Option<Id>,
    pub navigated: Option<Id>,
}

impl Bar {
    /// The name of the item that has focus, or "before" and "after" for the outside ones.
    pub fn focused(&self) -> Option<&'static str> {
        if self.before.has_focus {
            return Some("before");
        }
        if self.after.has_focus {
            return Some("after");
        }
        self.items
            .iter()
            .find(|(_, r)| r.has_focus)
            .map(|(n, _)| *n)
    }
    pub fn item(&self, name: &str) -> Response {
        self.items.iter().find(|(n, _)| *n == name).unwrap().1
    }
}

pub fn bar_in(ui: &mut Ui<'_>, names: &[&'static str], opts: &Opts) -> Bar {
    let before = ui.add(Button::new("before"));
    let mut items = Vec::new();
    let out = FocusGroup::new("bar")
        .axis(opts.axis)
        .wrap(opts.wrap)
        .label("Bar")
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                for name in names.iter().filter(|n| !opts.hidden.contains(n)) {
                    let enabled = !opts.disabled.contains(name);
                    let r = ui.add_enabled_ui(enabled, |ui| ui.add(Button::new(*name)));
                    items.push((*name, r));
                }
            });
        });
    let after = ui.add(Button::new("after"));
    Bar {
        before,
        after,
        items,
        group: out.response,
        stop: out.stop(),
        navigated: out.navigated(),
    }
}

pub fn bar(c: &mut Context, names: &[&'static str], opts: &Opts) -> Bar {
    let mut out = None;
    c.run(|c| {
        Root::new().show(c, |ui| out = Some(bar_in(ui, names, opts)));
    });
    out.unwrap()
}

pub const ABC: [&str; 3] = ["a", "b", "c"];

pub fn tab(c: &mut Context) -> bool {
    tap(c, KeyCode::Tab)
}

pub fn shift_tab(c: &mut Context) -> bool {
    mods(c, ModifiersState::SHIFT);
    let consumed = tap(c, KeyCode::Tab);
    mods(c, ModifiersState::empty());
    consumed
}

/// Press `code`, then run a pass and report who has focus.
pub fn key_then_look(
    c: &mut Context,
    code: KeyCode,
    names: &[&'static str],
    opts: &Opts,
) -> Option<&'static str> {
    tap(c, code);
    bar(c, names, opts).focused()
}
