//! Fixed-height virtualized tree. The application owns the model and loading.
mod access;
mod drag;
mod input;
mod model;
mod show;
mod state;
pub use super::disclosure::TreeStyle;
use super::Ui;
use crate::{Id, Rect, Vec2};
pub(crate) use input::TreeInput;
pub use model::{TreeChildren, TreeEvent, TreeIssue, TreeModel, TreeNode};
pub(crate) use state::TreeState;
use std::{collections::HashSet, hash::Hash, ops::Range};

pub struct TreeView<'a> {
    id: Id,
    initial_open: Vec<Id>,
    initial_selected: Option<Id>,
    controlled_open: Option<&'a mut HashSet<Id>>,
    controlled_selected: Option<&'a mut Option<Id>>,
    follows_focus: bool,
    double_click_expand: bool,
    enabled: bool,
    drag: bool,
    height: f32,
    style: Option<TreeStyle>,
    reveal: Option<Id>,
    label: String,
}
#[derive(Debug)]
pub struct TreeOutput {
    /// Focus-engine owner. All rows share this single Tab stop.
    pub id: Id,
    pub selected: Option<Id>,
    pub focused: Option<Id>,
    pub events: Vec<TreeEvent>,
    pub issues: Vec<TreeIssue>,
    pub visible_rows: Range<usize>,
    pub logical_rows: usize,
    pub rows_built: usize,
    pub rebuilt: bool,
    pub viewport: Rect,
    pub scroll_offset: Vec2,
}
impl<'a> TreeView<'a> {
    pub fn new(source: impl Hash) -> Self {
        Self {
            id: Id::new(source),
            initial_open: Vec::new(),
            initial_selected: None,
            controlled_open: None,
            controlled_selected: None,
            follows_focus: false,
            double_click_expand: false,
            enabled: true,
            drag: false,
            height: 300.0,
            style: None,
            reveal: None,
            label: String::new(),
        }
    }
    pub fn id_source(mut self, source: impl Hash) -> Self {
        self.id = Id::new(source);
        self
    }
    pub fn default_open(mut self, nodes: impl IntoIterator<Item = Id>) -> Self {
        self.initial_open = nodes.into_iter().collect();
        self
    }
    pub fn default_selected(mut self, node: Option<Id>) -> Self {
        self.initial_selected = node;
        self
    }
    /// Applied each pass; gestures mutate this set and emit one OpenChanged.
    pub fn open(mut self, nodes: &'a mut HashSet<Id>) -> Self {
        self.controlled_open = Some(nodes);
        self
    }
    pub fn selected(mut self, node: &'a mut Option<Id>) -> Self {
        self.controlled_selected = Some(node);
        self
    }
    pub fn selection_follows_focus(mut self, follows: bool) -> Self {
        self.follows_focus = follows;
        self
    }
    pub fn expand_on_double_click(mut self, expand: bool) -> Self {
        self.double_click_expand = expand;
        self
    }
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    /// Let the user drag nodes before, after or inside other nodes. The tree
    /// reports `TreeEvent::Moved` and never changes the model; dropping a node
    /// into its own subtree is rejected. Open and selected state follow node ids.
    /// Ctrl+Space picks up the focused node, then the arrow keys choose the place.
    pub fn drag_nodes(mut self, enabled: bool) -> Self {
        self.drag = enabled;
        self
    }
    pub fn max_height(mut self, height: f32) -> Self {
        self.height = super::disclosure::dimension(height);
        self
    }
    pub fn style(mut self, style: TreeStyle) -> Self {
        self.style = Some(style);
        self
    }
    #[track_caller]
    pub fn row_height(mut self, height: f32) -> Self {
        if let Some(height) = super::sanitize::positive("TreeView::row_height", height) {
            self.style.get_or_insert_with(Default::default).row.height = Some(height);
        }
        self
    }
    /// Explicit operation. Expand ancestors, then focus and scroll when available.
    /// Pass only when requested (for example `pending_reveal.take()`).
    pub fn reveal_node(mut self, node: Id) -> Self {
        self.reveal = Some(node);
        self
    }
    /// The name a screen reader speaks for the tree ("Project files"). The tree draws no
    /// caption of its own, so without this it is announced as an unnamed tree.
    pub fn accessible_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }
    pub fn show(self, ui: &mut Ui<'_>, model: &impl TreeModel) -> TreeOutput {
        self.show_with_actions(ui, model, |_, _| ())
    }
    /// Called only for viewport rows, under tree+node scopes independent of index.
    pub fn show_with_actions(
        self,
        ui: &mut Ui<'_>,
        model: &impl TreeModel,
        actions: impl FnMut(&mut Ui<'_>, Id),
    ) -> TreeOutput {
        ui.layout_item(|ui| self.show_inner(ui, model, actions))
    }
}
impl Ui<'_> {
    pub fn tree(&mut self, source: impl Hash, model: &impl TreeModel) -> TreeOutput {
        TreeView::new(source).show(self, model)
    }
}
