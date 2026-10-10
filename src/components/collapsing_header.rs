//! A stable immediate mode disclosure; content and actions execute once per pass.
pub use super::disclosure::CollapsingStyle;
use super::{
    disclosure::{self, Header},
    Response, Ui,
};
use crate::{
    context::HitAction, AccessAction, AccessActionKind, AccessRole, Id, ImageSource, Layout,
    Padding, Rect, Vec2,
};
use std::hash::Hash;

pub(crate) struct CollapsingState {
    pub last_frame: u64,
    open: bool,
    pub(crate) descendants: Vec<Id>,
}
pub struct CollapsingHeader<'a> {
    id: Id,
    caption: String,
    initial: bool,
    controlled: Option<&'a mut bool>,
    enabled: bool,
    expandable: bool,
    icon: Option<ImageSource>,
    style: Option<CollapsingStyle>,
}
pub struct CollapsingOutput<R, A = ()> {
    pub header_response: Response,
    pub open: bool,
    pub changed: bool,
    pub body: Option<R>,
    pub actions: A,
}
impl<'a> CollapsingHeader<'a> {
    pub fn new(source: impl Hash, caption: impl Into<String>) -> Self {
        Self {
            id: Id::new(source),
            caption: caption.into(),
            initial: false,
            controlled: None,
            enabled: true,
            expandable: true,
            icon: None,
            style: None,
        }
    }
    pub fn id_source(mut self, source: impl Hash) -> Self {
        self.id = Id::new(source);
        self
    }
    /// Initial value only. Later passes retain user choice.
    pub fn default_open(mut self, open: bool) -> Self {
        self.initial = open;
        self
    }
    /// Authoritative each pass; user gestures write back once and set changed.
    pub fn open(mut self, open: &'a mut bool) -> Self {
        self.controlled = Some(open);
        self
    }
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    /// Lock disclosure while leaving the content and header actions enabled.
    pub fn expandable(mut self, expandable: bool) -> Self {
        self.expandable = expandable;
        self
    }
    pub fn icon(mut self, source: impl Into<ImageSource>) -> Self {
        self.icon = Some(source.into());
        self
    }
    pub fn style(mut self, style: CollapsingStyle) -> Self {
        self.style = Some(style);
        self
    }
    #[track_caller]
    pub fn header_height(mut self, height: f32) -> Self {
        self.style
            .get_or_insert_with(Default::default)
            .header
            .height = Some(disclosure::dimension(height));
        self
    }
    pub fn animate_height(mut self, animate: bool) -> Self {
        self.style
            .get_or_insert_with(Default::default)
            .animate_height = Some(animate);
        self
    }
    pub fn show<R>(
        self,
        ui: &mut Ui<'_>,
        body: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> CollapsingOutput<R> {
        self.show_with_actions(ui, |_| (), body)
    }
    pub fn show_with_actions<R, A>(
        self,
        ui: &mut Ui<'_>,
        actions: impl FnOnce(&mut Ui<'_>) -> A,
        body: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> CollapsingOutput<R, A> {
        ui.layout_item(|ui| self.show_inner(ui, actions, body))
    }
    fn show_inner<R, A>(
        mut self,
        ui: &mut Ui<'_>,
        actions: impl FnOnce(&mut Ui<'_>) -> A,
        body: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> CollapsingOutput<R, A> {
        let id = ui.scope.with(("collapsing", self.id));
        let mut state = ui
            .context
            .containers
            .collapsing_headers
            .remove(&id)
            .unwrap_or(CollapsingState {
                last_frame: 0,
                open: self.initial,
                descendants: Vec::new(),
            });
        if state.last_frame == ui.context.frame {
            ui.context
                .report(crate::DiagnosticKind::IdCollision, Some(id), None, || {
                    "duplicate id: two CollapsingHeaders share one id_source".into()
                });
        }
        state.last_frame = ui.context.frame;
        if let Some(open) = &self.controlled {
            state.open = **open;
        }
        let mut style = ui.style().collapsing.clone();
        if let Some(patch) = self.style {
            style.merge(&patch);
        }
        let enabled = self.enabled && ui.enabled;
        let before = state.open;
        // Assistive technology names the state it wants, so a repeated request changes nothing.
        for action in ui.context.take_access_actions(id) {
            match action {
                AccessAction::Expand if enabled && self.expandable => state.open = true,
                AccessAction::Collapse if enabled && self.expandable => state.open = false,
                _ => {}
            }
        }
        // The header is described before its row is built, so it precedes the row's own
        // actions in the tree without containing them; the row gives it its bounds below.
        let node = ui.a11y_declare(id, AccessRole::Button, |node| {
            node.label(self.caption.as_str()).disabled(!enabled);
            if self.expandable {
                node.clicks(id)
                    .action(AccessActionKind::Expand)
                    .action(AccessActionKind::Collapse);
            }
        });
        let output = disclosure::header(
            ui,
            Header {
                id,
                label: &self.caption,
                icon: self.icon.as_ref(),
                branch: self.expandable,
                open: state.open,
                enabled,
                selected: false,
                focus: false,
                indent: 0.0,
                action: if self.expandable {
                    HitAction::Activate
                } else {
                    HitAction::Focus
                },
                chevron_action: None,
                style: &style.header,
            },
            actions,
        );
        if enabled && self.expandable && output.response.clicked() {
            state.open = !state.open;
        }
        if let Some(node) = ui.context.a11y_scope_mut(&node) {
            node.expanded(state.open);
        }
        ui.a11y_end(node, Some(output.response.rect));
        let changed = state.open != before;
        if changed {
            if let Some(open) = self.controlled.as_mut() {
                **open = state.open;
            }
            ui.context.request_repaint();
        }
        if !state.open
            && ui
                .context
                .focused()
                .is_some_and(|f| state.descendants.contains(&f))
        {
            if ui.context.popups.branch.first().is_some_and(|p| {
                p.return_focus
                    .is_some_and(|f| state.descendants.contains(&f))
            }) {
                ui.context.dismiss_popup(false);
            }
            ui.context.set_focus(Some(id));
        }
        let padding = style.content_padding.unwrap_or(Padding {
            left: 18.0,
            ..Padding::default()
        });
        let spacing = disclosure::dimension(style.spacing.unwrap_or(ui.style().spacing));
        let mut motion = style
            .motion
            .clone()
            .unwrap_or_else(|| ui.style().motion.expand.clone());
        if !style.animate_height.unwrap_or(true) || ui.style().motion.reduced_motion {
            motion = crate::TweenOptions::new(std::time::Duration::ZERO);
        }
        ui.context.begin_placement(ui.window);
        let result = ui.push_id(id, |ui| {
            ui.add_enabled_ui(enabled, |ui| {
                ui.reveal_with("body", state.open, motion, |ui| {
                    ui.add_space(spacing);
                    let bounds = Rect::from_min_max(ui.layout.cursor, ui.layout.bounds.max);
                    let inner_bounds = padding.inset(bounds);
                    let mut child =
                        disclosure::child(ui, id.with("content"), inner_bounds, Layout::Vertical);
                    child.begin_layout(crate::Align::Start);
                    let result = body(&mut child);
                    child.finish_layout();
                    let used = child.layout.used + padding.size();
                    ui.allocate_space(used.max(Vec2::ZERO));
                    result
                })
            })
        });
        let placement = ui.context.end_placement();
        if state.open {
            state.descendants = ui.context.placement_hit_ids(&placement);
        }
        ui.context.place(placement, Vec2::ZERO, ui.clip);
        let mut response = output.response;
        response.changed = changed;
        let out = CollapsingOutput {
            header_response: response,
            open: state.open,
            changed,
            body: result,
            actions: output.actions,
        };
        ui.context.containers.collapsing_headers.insert(id, state);
        out
    }
}
impl Ui<'_> {
    pub fn collapsing<R>(
        &mut self,
        source: impl Hash,
        caption: impl Into<String>,
        body: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> CollapsingOutput<R> {
        CollapsingHeader::new(source, caption).show(self, body)
    }
}
