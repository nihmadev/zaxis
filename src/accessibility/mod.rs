//! The accessibility layer: what each widget is, says and accepts, for screen readers and
//! other assistive technology.
//!
//! Widgets describe themselves while the UI is built ([`collect`]): a node with a role, a
//! name, a value, state and the actions it accepts, parented by the scope it was built in.
//! Geometry travels with the widget's hit regions, so placement, scrolling and visual
//! transforms apply to a node exactly as they apply to input. At the end of the pass the
//! nodes become an AccessKit tree (`tree`, `convert`): only nodes that changed are sent.
//! Requests from assistive technology come back as ordinary input for the next pass
//! (`actions`). The platform adapter lives in `adapter`.
//!
//! Nothing is collected until assistive technology asks for the tree: the cost of the
//! layer without a screen reader is one flag test per widget.

#![cfg_attr(not(feature = "accesskit"), allow(dead_code))]

#[cfg(feature = "accesskit")]
mod actions;
#[cfg(all(feature = "accesskit", not(target_arch = "wasm32")))]
pub(crate) mod adapter;
#[cfg(feature = "accesskit")]
mod audit;
mod collect;
#[cfg(feature = "accesskit")]
mod convert;
mod ids;
mod node;
mod role;
#[cfg(feature = "accesskit")]
#[doc(hidden)]
pub mod testing;
pub(crate) mod text;
#[cfg(feature = "accesskit")]
mod tree;
mod widget;

pub use node::AccessNode;
pub use role::{
    AccessAction, AccessActionKind, AccessLive, AccessOrientation, AccessRole, AccessToggled,
};
#[cfg(feature = "accesskit")]
#[doc(hidden)]
pub use tree::AccessStats;
pub use widget::Accessible;

use crate::{text::TextLayout, Context, Id, Vec2};
use std::{collections::HashMap, sync::Arc};

/// A node that is a direct child of the window.
pub(crate) const NO_PARENT: u32 = u32::MAX;

/// Where a node's bounds come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Geometry {
    /// The union of its children: a container that never placed itself.
    Derived,
    /// A geometry hit region was registered and has not come back yet.
    Pending,
    Placed,
}

/// Bookkeeping of one collected node that is not part of its description.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Slot {
    pub parent: u32,
    pub geometry: Geometry,
    /// Hidden with `accessibility_hidden`: neither it nor its children are published.
    pub removed: bool,
}

/// An opened container; give it back to [`Context::a11y_end`].
#[must_use = "an opened accessibility scope must be closed with a11y_end"]
pub(crate) struct Scope(pub(crate) Option<u32>);

impl Scope {
    pub(crate) const NONE: Self = Self(None);
}

/// A message for screen readers that belongs to no widget.
#[derive(Default)]
pub(crate) struct Announcement {
    pub text: String,
    /// Changes with every message so a repeated text is announced again.
    pub serial: u64,
}

#[derive(Default)]
pub(crate) struct State {
    /// Nodes are being collected in this pass.
    pub active: bool,
    /// What `active` becomes at the start of the next pass.
    wanted: bool,
    pub nodes: Vec<AccessNode>,
    pub slots: Vec<Slot>,
    pub stack: Vec<u32>,
    /// Geometry hit regions of this pass, by their own id.
    pub geometry: HashMap<Id, u32>,
    /// Requests from assistive technology waiting for the widget they target.
    pub pending: HashMap<Id, Vec<AccessAction>>,
    /// Clicks on widgets that had to be scrolled into view first: delivered to the pass
    /// after the one that lays them out in view.
    pub late_clicks: Vec<Id>,
    /// Hit regions of this pass that take clicks, including those scrolled out of view,
    /// which the published hit list drops.
    pub clickable: std::collections::HashSet<Id>,
    /// Polite, then assertive.
    pub announcements: [Announcement; 2],
    pub title: Option<String>,
    /// Text models of static text by the layout they were built from, with the pass that
    /// last used them. A label that does not change is described without rebuilding its
    /// lines; an entry not used in a pass is dropped at its end.
    texts: HashMap<usize, (Arc<TextLayout>, text::TextRef, u64)>,
    #[cfg(feature = "accesskit")]
    pub tree: tree::Snapshot,
}

impl State {
    pub(crate) fn begin_pass(&mut self) {
        self.active = self.wanted;
        self.nodes.clear();
        self.slots.clear();
        self.stack.clear();
        self.geometry.clear();
        self.clickable.clear();
    }

    /// Requests nobody took and text models nobody used do not outlive the pass.
    pub(crate) fn end_pass(&mut self, frame: u64) {
        self.pending.clear();
        self.texts.retain(|_, entry| entry.2 == frame);
    }

    pub(crate) fn take_late_clicks(&mut self) -> Vec<Id> {
        std::mem::take(&mut self.late_clicks)
    }
}

impl Context {
    /// Collect the accessibility tree from the next pass on, or stop collecting.
    ///
    /// [`run`](crate::run) calls this when assistive technology connects to a window and
    /// when it disconnects; a custom host does the same from its AccessKit adapter. Turning
    /// it on by hand is also how an application audits itself: nameless controls are then
    /// reported through [`Context::diagnostics`]. Without the `accesskit` feature the tree
    /// is never built and this only stores the flag.
    pub fn set_accessibility_active(&mut self, active: bool) {
        if self.a11y.wanted == active {
            return;
        }
        self.a11y.wanted = active;
        if !active {
            self.a11y.pending.clear();
            #[cfg(feature = "accesskit")]
            self.a11y.tree.reset();
        }
        self.request_repaint();
    }

    /// Whether widgets are describing themselves to assistive technology.
    pub fn accessibility_active(&self) -> bool {
        self.a11y.wanted
    }

    /// The name of the window in the accessibility tree; [`run`](crate::run) sets it to the
    /// native window's title.
    pub fn set_accessibility_title(&mut self, title: impl Into<String>) {
        let title = title.into();
        let title = (!title.is_empty()).then_some(title);
        if self.a11y.title != title {
            self.a11y.title = title;
            if self.a11y.wanted {
                self.request_repaint();
            }
        }
    }

    /// Have a screen reader speak `text` when it is idle, without moving focus: the result
    /// of an action, a status that changed. A [`Toast`](crate::Toast) announces itself.
    /// Does nothing while no assistive technology is connected.
    pub fn announce(&mut self, text: impl Into<String>) {
        self.push_announcement(0, text.into());
    }

    /// Like [`Self::announce`], interrupting what is being spoken. For errors and warnings
    /// the user has to hear now.
    pub fn announce_assertive(&mut self, text: impl Into<String>) {
        self.push_announcement(1, text.into());
    }

    /// The visual lines of `text` as shaped in `layout`, relative to its top left corner.
    pub(crate) fn a11y_text(&mut self, text: &str, layout: &Arc<TextLayout>) -> text::TextRef {
        let frame = self.frame;
        let entry = self
            .a11y
            .texts
            .entry(Arc::as_ptr(layout) as usize)
            .or_insert_with(|| {
                let mut model = text::TextModel::default();
                model.push_text(text, &layout.lines, Vec2::ZERO);
                (Arc::clone(layout), model.into(), frame)
            });
        entry.2 = frame;
        entry.1.clone()
    }

    fn push_announcement(&mut self, slot: usize, text: String) {
        if !self.a11y.wanted || text.is_empty() {
            return;
        }
        let announcement = &mut self.a11y.announcements[slot];
        announcement.text = text;
        announcement.serial += 1;
        self.request_repaint();
    }
}
