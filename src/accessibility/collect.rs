//! Collecting nodes while the UI is built.
//!
//! A leaf describes itself with [`Ui::a11y`]; a container opens a scope with
//! [`Ui::a11y_begin`] and closes it with [`Ui::a11y_end`], and everything described in
//! between becomes its child. Layers (windows, popups, dialogs) open their scope straight
//! on the [`Context`]. Every entry point returns at once while collection is off.

use super::{
    node::DISABLED, AccessAction, AccessNode, AccessRole, Geometry, Scope, Slot, State, NO_PARENT,
};
use crate::{
    context::{HitAction, HitRegion},
    Context, Id, Rect, Response, Ui,
};
use std::hash::Hash;

impl Context {
    /// Whether nodes are collected in this pass. The one test widgets pay for when no
    /// assistive technology is connected.
    #[inline]
    pub(crate) fn a11y_on(&self) -> bool {
        self.a11y.active
    }

    /// Number of nodes collected so far, to find the ones a widget adds.
    pub(crate) fn a11y_len(&self) -> usize {
        self.a11y.nodes.len()
    }

    pub(crate) fn a11y_node_mut(&mut self, index: usize) -> Option<&mut AccessNode> {
        self.a11y.nodes.get_mut(index)
    }

    /// The node of an open (or declared) scope, to complete its description.
    pub(crate) fn a11y_scope_mut(&mut self, scope: &Scope) -> Option<&mut AccessNode> {
        self.a11y.nodes.get_mut(scope.0? as usize)
    }

    /// The first node collected at or after `start` that satisfies `test`.
    pub(crate) fn a11y_find(
        &mut self,
        start: usize,
        test: impl Fn(&AccessNode) -> bool,
    ) -> Option<&mut AccessNode> {
        self.a11y
            .nodes
            .get_mut(start..)?
            .iter_mut()
            .find(|node| test(node))
    }

    fn a11y_add(&mut self, node: AccessNode, top_level: bool) -> u32 {
        let index = self.a11y.nodes.len() as u32;
        let parent = if top_level {
            NO_PARENT
        } else {
            self.a11y.stack.last().copied().unwrap_or(NO_PARENT)
        };
        self.a11y.nodes.push(node);
        self.a11y.slots.push(Slot {
            parent,
            geometry: Geometry::Derived,
            removed: false,
        });
        index
    }

    /// Give node `index` its bounds. They ride a passive hit region, so the placement,
    /// scroll and visual transforms of the enclosing scopes apply to them as to input.
    fn a11y_place(&mut self, index: u32, rect: Rect, clip: Rect) {
        let id = Id::new(("zaxis-a11y-geometry", index));
        let window = self.a11y.nodes[index as usize].layer;
        self.a11y.geometry.insert(id, index);
        self.a11y.slots[index as usize].geometry = Geometry::Pending;
        self.route_hit(HitRegion {
            id,
            window,
            rect,
            clip,
            action: HitAction::Semantic,
        });
    }

    /// Open a layer: a direct child of the window, whatever scope is being built.
    pub(crate) fn a11y_begin_layer(
        &mut self,
        layer: Id,
        id: Id,
        role: AccessRole,
        describe: impl FnOnce(&mut AccessNode),
    ) -> Scope {
        if !self.a11y.active {
            return Scope::NONE;
        }
        let mut node = AccessNode::new(id, layer, role);
        describe(&mut node);
        let index = self.a11y_add(node, true);
        self.a11y.stack.push(index);
        Scope(Some(index))
    }

    /// Describe a node outside any `Ui`, in the scope that is open (or as a layer child).
    pub(crate) fn a11y_leaf(
        &mut self,
        layer: Id,
        id: Id,
        role: AccessRole,
        bounds: (Rect, Rect),
        describe: impl FnOnce(&mut AccessNode),
    ) {
        if !self.a11y.active {
            return;
        }
        let mut node = AccessNode::new(id, layer, role);
        describe(&mut node);
        let index = self.a11y_add(node, false);
        self.a11y_place(index, bounds.0, bounds.1);
    }

    /// Close a scope. With `bounds` (rect, clip) the container is placed there; without,
    /// its bounds are the union of its children.
    pub(crate) fn a11y_end(&mut self, scope: Scope, bounds: Option<(Rect, Rect)>) {
        let Some(index) = scope.0 else {
            return;
        };
        if let Some(at) = self.a11y.stack.iter().rposition(|open| *open == index) {
            self.a11y.stack.truncate(at);
        }
        if let Some((rect, clip)) = bounds {
            self.a11y_place(index, rect, clip);
        }
    }

    /// Requests from assistive technology for the node `id`, in arrival order. Each is
    /// delivered once, on the pass after it arrived.
    pub(crate) fn take_access_actions(&mut self, id: Id) -> Vec<AccessAction> {
        if self.a11y.pending.is_empty() {
            return Vec::new();
        }
        self.a11y.pending.remove(&id).unwrap_or_default()
    }

    /// Passive geometry regions leave the hit list before it routes input, carrying their
    /// final rect and clip to the node they belong to.
    pub(crate) fn a11y_resolve_geometry(&mut self) {
        let State {
            geometry,
            nodes,
            slots,
            clickable,
            ..
        } = &mut self.a11y;
        self.interaction.retain_hits(|hit| {
            let Some(&index) = geometry.get(&hit.id) else {
                if hit.action.sense().click() {
                    clickable.insert(hit.id);
                }
                return hit.action != HitAction::Semantic;
            };
            let node = &mut nodes[index as usize];
            node.rect = hit.rect;
            // A region of content that is laid out but not shown (fully transparent)
            // comes back blocked: an empty clip marks its node hidden.
            node.clip = if hit.action == HitAction::Semantic {
                hit.clip
            } else {
                Rect::default()
            };
            slots[index as usize].geometry = Geometry::Placed;
            false
        });
    }
}

impl Ui<'_> {
    fn a11y_new(
        &mut self,
        id: Id,
        role: AccessRole,
        describe: impl FnOnce(&mut AccessNode),
    ) -> AccessNode {
        let mut node = AccessNode::new(id, self.window, role);
        describe(&mut node);
        if !self.enabled {
            node.flags |= DISABLED;
        }
        node
    }

    /// Describe a widget without children. `describe` runs only while assistive technology
    /// is connected, so building a label costs nothing otherwise.
    #[inline]
    pub(crate) fn a11y(
        &mut self,
        id: Id,
        rect: Rect,
        role: AccessRole,
        describe: impl FnOnce(&mut AccessNode),
    ) {
        if self.context.a11y.active {
            let node = self.a11y_new(id, role, describe);
            let index = self.context.a11y_add(node, false);
            self.context.a11y_place(index, rect, self.clip);
        }
    }

    /// Open a container; nodes described until [`Self::a11y_end`] are its children.
    #[inline]
    pub(crate) fn a11y_begin(
        &mut self,
        id: Id,
        role: AccessRole,
        describe: impl FnOnce(&mut AccessNode),
    ) -> Scope {
        if !self.context.a11y.active {
            return Scope::NONE;
        }
        let node = self.a11y_new(id, role, describe);
        let index = self.context.a11y_add(node, false);
        self.context.a11y.stack.push(index);
        Scope(Some(index))
    }

    /// Describe a widget whose rect is not known yet: the node takes its place in the tree
    /// now, ahead of whatever is described next, without becoming a container. Give it
    /// its bounds with [`Self::a11y_end`].
    #[inline]
    pub(crate) fn a11y_declare(
        &mut self,
        id: Id,
        role: AccessRole,
        describe: impl FnOnce(&mut AccessNode),
    ) -> Scope {
        if !self.context.a11y.active {
            return Scope::NONE;
        }
        let node = self.a11y_new(id, role, describe);
        Scope(Some(self.context.a11y_add(node, false)))
    }

    /// Close a container opened by [`Self::a11y_begin`], placing it at `rect` when given.
    #[inline]
    pub(crate) fn a11y_end(&mut self, scope: Scope, rect: Option<Rect>) {
        if scope.0.is_some() {
            let clip = self.clip;
            self.context.a11y_end(scope, rect.map(|rect| (rect, clip)));
        }
    }

    /// Describe a custom widget to assistive technology and receive what it asks for.
    ///
    /// Call it after [`Ui::interact`] (or with any [`Response`]) in the widget's own code:
    /// the node takes the response's `Id` and rect. `describe` sets the role, the name, the
    /// value, the state and the accepted actions; it runs only while a screen reader is
    /// connected. A region that takes focus is focusable in the tree as well, and a `Click`
    /// request on a region made with `interact` arrives as [`Response::clicked`] on the next
    /// pass. Every other request is returned here, once, on the pass after it arrived:
    ///
    /// ```
    /// # use zaxis::{AccessAction, AccessActionKind, AccessRole, Sense, Ui, Vec2};
    /// # fn dial(ui: &mut Ui<'_>, volume: &mut f64) {
    /// let rect = ui.allocate_space(Vec2::splat(48.0));
    /// let response = ui.interact(rect, "volume", Sense::DRAG | Sense::FOCUS);
    /// let requests = ui.accessible(&response, |node| {
    ///     node.role(AccessRole::Slider)
    ///         .label("Volume")
    ///         .numeric(*volume, 0.0, 1.0)
    ///         .step(0.1)
    ///         .action(AccessActionKind::Increment)
    ///         .action(AccessActionKind::Decrement);
    /// });
    /// for request in requests {
    ///     match request {
    ///         AccessAction::Increment => *volume = (*volume + 0.1).min(1.0),
    ///         AccessAction::Decrement => *volume = (*volume - 0.1).max(0.0),
    ///         _ => {}
    ///     }
    /// }
    /// # }
    /// ```
    ///
    /// A custom widget that never calls this is not in the tree at all.
    pub fn accessible(
        &mut self,
        response: &Response,
        describe: impl FnOnce(&mut AccessNode),
    ) -> Vec<AccessAction> {
        let id = response.id;
        self.a11y(id, response.rect, AccessRole::Group, |node| {
            node.click = Some(id);
            describe(node);
        });
        let requests = self.context.take_access_actions(id);
        if !requests.is_empty() {
            self.context.request_repaint();
        }
        requests
    }

    /// Group the widgets built by `build` under one named node: a toolbar, a form section,
    /// a card of related controls. Layout is not affected.
    pub fn accessible_group<R>(
        &mut self,
        id_source: impl Hash,
        role: AccessRole,
        label: &str,
        build: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let id = self.scope.with(("accessible-group", Id::new(id_source)));
        let scope = self.a11y_begin(id, role, |node| {
            node.label(label);
        });
        let result = build(self);
        self.a11y_end(scope, None);
        result
    }
}
