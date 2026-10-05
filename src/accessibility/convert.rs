//! Turning collected nodes into AccessKit nodes. The only place that knows AccessKit's
//! roles and properties.

use super::{
    node::{
        BUSY, CLIPS, DISABLED, HAS_POPUP, HIDDEN, INVALID, MODAL, MULTISELECTABLE, READ_ONLY,
        REQUIRED,
    },
    tree::Entry,
    AccessActionKind as Kind, AccessLive, AccessNode, AccessOrientation, AccessRole, AccessToggled,
    Announcement,
};
use crate::{Id, Rect, Vec2};
use accesskit::{
    Action, HasPopup, Invalid, Live, Node, NodeId, Orientation, Role, TextDirection, TextPosition,
    TextSelection, Toggled,
};

fn role(node: &AccessNode) -> Role {
    match node.role {
        // A group nobody named only structures the tree; screen readers skip over it.
        AccessRole::Group if node.label.is_none() => Role::GenericContainer,
        AccessRole::Group => Role::Group,
        AccessRole::Window => Role::Window,
        AccessRole::Pane => Role::Pane,
        AccessRole::Region => Role::Region,
        AccessRole::Dialog => Role::Dialog,
        AccessRole::AlertDialog => Role::AlertDialog,
        AccessRole::Label => Role::Label,
        AccessRole::Paragraph => Role::Paragraph,
        AccessRole::Heading => Role::Heading,
        AccessRole::Link => Role::Link,
        AccessRole::Image => Role::Image,
        AccessRole::Button => Role::Button,
        AccessRole::CheckBox => Role::CheckBox,
        AccessRole::Switch => Role::Switch,
        AccessRole::RadioGroup => Role::RadioGroup,
        AccessRole::RadioButton => Role::RadioButton,
        AccessRole::Slider => Role::Slider,
        AccessRole::SpinButton => Role::SpinButton,
        AccessRole::TextInput => Role::TextInput,
        AccessRole::MultilineTextInput => Role::MultilineTextInput,
        AccessRole::ComboBox => Role::ComboBox,
        AccessRole::ListBox => Role::ListBox,
        AccessRole::ListBoxOption => Role::ListBoxOption,
        AccessRole::List => Role::List,
        AccessRole::ListItem => Role::ListItem,
        AccessRole::Tree => Role::Tree,
        AccessRole::TreeItem => Role::TreeItem,
        AccessRole::Table => Role::Table,
        AccessRole::Grid => Role::Grid,
        AccessRole::Row => Role::Row,
        AccessRole::ColumnHeader => Role::ColumnHeader,
        AccessRole::RowHeader => Role::RowHeader,
        AccessRole::Cell => Role::Cell,
        AccessRole::TabList => Role::TabList,
        AccessRole::Tab => Role::Tab,
        AccessRole::TabPanel => Role::TabPanel,
        AccessRole::MenuBar => Role::MenuBar,
        AccessRole::Menu => Role::Menu,
        AccessRole::MenuItem => Role::MenuItem,
        AccessRole::MenuItemCheckBox => Role::MenuItemCheckBox,
        AccessRole::MenuItemRadio => Role::MenuItemRadio,
        AccessRole::Tooltip => Role::Tooltip,
        AccessRole::Status => Role::Status,
        AccessRole::Alert => Role::Alert,
        AccessRole::ScrollView => Role::ScrollView,
        AccessRole::ScrollBar => Role::ScrollBar,
        AccessRole::Splitter => Role::Splitter,
        AccessRole::ProgressIndicator => Role::ProgressIndicator,
        AccessRole::ColorWell => Role::ColorWell,
        AccessRole::Toolbar => Role::Toolbar,
        AccessRole::TitleBar => Role::TitleBar,
        AccessRole::Canvas => Role::Canvas,
    }
}

fn bounds(rect: Rect) -> accesskit::Rect {
    accesskit::Rect {
        x0: f64::from(rect.min.x),
        y0: f64::from(rect.min.y),
        x1: f64::from(rect.max.x.max(rect.min.x)),
        y1: f64::from(rect.max.y.max(rect.min.y)),
    }
}

fn ids(targets: &[Id], resolve: &impl Fn(Id) -> Option<u64>) -> Vec<NodeId> {
    targets
        .iter()
        .filter_map(|id| resolve(*id))
        .map(NodeId)
        .collect()
}

/// The AccessKit node of `node`, whose bounds are already physical pixels.
pub(super) fn node(
    node: &AccessNode,
    entry: &Entry,
    children: &[u64],
    runs: &[u64],
    scale: f32,
    resolve: &impl Fn(Id) -> Option<u64>,
) -> Node {
    let mut out = Node::new(role(node));
    out.set_bounds(bounds(node.rect));
    if !children.is_empty() {
        out.set_children(children.iter().copied().map(NodeId).collect::<Vec<_>>());
    }
    // The text of a label is its value. A heading or a paragraph is named by its text
    // instead: published as a value it would look editable to assistive technology.
    let named_by_text = node.role.is_text() && node.role != AccessRole::Label;
    match (&node.label, &node.value) {
        (Some(label), _) => out.set_label(label.as_str()),
        (None, Some(value)) if named_by_text => out.set_label(value.as_str()),
        _ => {}
    }
    if let Some(value) = node.value.as_ref().filter(|_| !named_by_text) {
        out.set_value(value.as_str());
    }
    if let Some(description) = &node.description {
        out.set_description(description.as_str());
    }
    if let Some(toggled) = node.toggled {
        out.set_toggled(match toggled {
            AccessToggled::False => Toggled::False,
            AccessToggled::True => Toggled::True,
            AccessToggled::Mixed => Toggled::Mixed,
        });
    }
    if let Some(selected) = node.selected {
        out.set_selected(selected);
    }
    if let Some(expanded) = node.expanded {
        out.set_expanded(expanded);
    }
    if let Some((position, size)) = node.set {
        out.set_position_in_set(position as usize);
        out.set_size_of_set(size as usize);
    }
    if let Some(level) = node.level {
        out.set_level(level as usize);
    }
    let disabled = node.has(DISABLED);
    // A widget that is laid out but not shown (fully transparent) has an empty clip; one
    // merely scrolled out of view keeps a real one.
    let vanished = node.clip == Rect::default() && node.rect != Rect::default();
    if node.has(HIDDEN) || vanished {
        out.set_hidden();
    }
    if disabled {
        out.set_disabled();
    }
    if node.has(READ_ONLY) {
        out.set_read_only();
    }
    if node.has(REQUIRED) {
        out.set_required();
    }
    if node.has(INVALID) {
        out.set_invalid(Invalid::True);
    }
    if node.has(MODAL) {
        out.set_modal();
    }
    if node.has(MULTISELECTABLE) {
        out.set_multiselectable();
    }
    if node.has(BUSY) {
        out.set_busy();
    }
    if node.has(CLIPS) {
        out.set_clips_children();
    }
    if node.has(super::node::VISITED) {
        out.set_visited();
    }
    if node.has(HAS_POPUP) {
        out.set_has_popup(match node.role {
            AccessRole::ComboBox => HasPopup::Listbox,
            _ => HasPopup::Menu,
        });
    }
    if !disabled {
        actions(node, entry, &mut out);
    }
    let Some(more) = &node.more else {
        return out;
    };
    if let Some(numeric) = more.numeric {
        out.set_numeric_value(numeric.value);
        out.set_min_numeric_value(numeric.min);
        out.set_max_numeric_value(numeric.max);
        if let Some(step) = numeric.step {
            out.set_numeric_value_step(step);
        }
        if let Some(jump) = numeric.jump {
            out.set_numeric_value_jump(jump);
        }
    }
    if let Some(placeholder) = &more.placeholder {
        out.set_placeholder(placeholder.as_str());
        // A field nothing else names is called by its placeholder, the last resort of
        // the accessible name computation.
        if node.label.is_none() && more.labelled_by.is_empty() {
            out.set_label(placeholder.as_str());
        }
    }
    if let Some(url) = &more.url {
        out.set_url(url.as_str());
    }
    if let Some(shortcut) = &more.shortcut {
        out.set_keyboard_shortcut(shortcut.as_str());
    }
    if let Some(description) = &more.role_description {
        out.set_role_description(description.as_str());
    }
    if let Some(orientation) = more.orientation {
        out.set_orientation(match orientation {
            AccessOrientation::Horizontal => Orientation::Horizontal,
            AccessOrientation::Vertical => Orientation::Vertical,
        });
    }
    if let Some(live) = more.live {
        out.set_live(match live {
            AccessLive::Polite => Live::Polite,
            AccessLive::Assertive => Live::Assertive,
        });
    }
    if let Some(scroll) = more.scroll {
        let physical = |v: f32| f64::from(if v.is_finite() { v * scale } else { 0.0 });
        out.set_scroll_x(physical(scroll.offset.x));
        out.set_scroll_y(physical(scroll.offset.y));
        out.set_scroll_x_min(0.0);
        out.set_scroll_y_min(0.0);
        out.set_scroll_x_max(physical(scroll.max.x));
        out.set_scroll_y_max(physical(scroll.max.y));
    }
    let table = more.table;
    let index = |value: Option<u32>, set: &mut dyn FnMut(usize)| {
        if let Some(value) = value {
            set(value as usize);
        }
    };
    index(table.row_count, &mut |v| out.set_row_count(v));
    index(table.column_count, &mut |v| out.set_column_count(v));
    index(table.row, &mut |v| out.set_row_index(v));
    index(table.column, &mut |v| out.set_column_index(v));
    index(table.row_span, &mut |v| out.set_row_span(v));
    index(table.column_span, &mut |v| out.set_column_span(v));
    if let Some(ascending) = table.sort_ascending {
        out.set_sort_direction(if ascending {
            accesskit::SortDirection::Ascending
        } else {
            accesskit::SortDirection::Descending
        });
    }
    if !more.labelled_by.is_empty() {
        out.set_labelled_by(ids(&more.labelled_by, resolve));
    }
    if !more.described_by.is_empty() {
        out.set_described_by(ids(&more.described_by, resolve));
    }
    if !more.controls.is_empty() {
        out.set_controls(ids(&more.controls, resolve));
    }
    if let Some(id) = more.error_message.and_then(resolve) {
        out.set_error_message(NodeId(id));
    }
    if let Some(id) = more.active_descendant.and_then(resolve) {
        out.set_active_descendant(NodeId(id));
    }
    if let Some(color) = more.color {
        let [red, green, blue, alpha] = color.0;
        out.set_color_value(accesskit::Color {
            red,
            green,
            blue,
            alpha,
        });
    }
    if let (Some(text), Some((anchor, focus))) = (&more.text, more.selection) {
        let position = |offset: usize| {
            let (run, character_index) = text.0.position(offset)?;
            Some(TextPosition {
                node: NodeId(*runs.get(run)?),
                character_index,
            })
        };
        if let (Some(anchor), Some(focus)) = (position(anchor), position(focus)) {
            out.set_text_selection(TextSelection { anchor, focus });
        }
    }
    out
}

fn actions(node: &AccessNode, entry: &Entry, out: &mut Node) {
    if entry.focusable || node.supports(Kind::Focus) {
        out.add_action(Action::Focus);
    }
    // A click is promised only where something answers it: a hit region that takes
    // clicks, or a widget that reads the request itself.
    if entry.clickable || node.supports(Kind::Click) {
        out.add_action(Action::Click);
    }
    for (kind, action) in [
        (Kind::Expand, Action::Expand),
        (Kind::Collapse, Action::Collapse),
        (Kind::Increment, Action::Increment),
        (Kind::Decrement, Action::Decrement),
        (Kind::SetValue, Action::SetValue),
        (Kind::ReplaceSelectedText, Action::ReplaceSelectedText),
        (Kind::SetTextSelection, Action::SetTextSelection),
        (Kind::ShowContextMenu, Action::ShowContextMenu),
    ] {
        if node.supports(kind) {
            out.add_action(action);
        }
    }
    if node.supports(Kind::ShowTooltip) {
        out.add_action(Action::ShowTooltip);
        out.add_action(Action::HideTooltip);
    }
    if entry.scrolled || node.supports(Kind::ScrollIntoView) {
        out.add_action(Action::ScrollIntoView);
    }
    let scroll = node.more.as_ref().and_then(|more| more.scroll);
    if let Some(scroll) = scroll.filter(|_| node.supports(Kind::Scroll)) {
        out.add_action(Action::SetScrollOffset);
        for (possible, action) in [
            (scroll.offset.x > 0.0, Action::ScrollLeft),
            (scroll.offset.x < scroll.max.x, Action::ScrollRight),
            (scroll.offset.y > 0.0, Action::ScrollUp),
            (scroll.offset.y < scroll.max.y, Action::ScrollDown),
        ] {
            if possible {
                out.add_action(action);
            }
        }
    }
}

/// The text runs of `node` as AccessKit nodes with ids `ids`, appended to `out`.
pub(super) fn runs(node: &AccessNode, ids: &[u64], scale: f32, out: &mut Vec<(NodeId, Node)>) {
    let Some(text) = node.more.as_ref().and_then(|more| more.text.as_ref()) else {
        return;
    };
    let scaled = |v: f32| if v.is_finite() { v * scale } else { 0.0 };
    let origin = node
        .more
        .as_ref()
        .map_or(Vec2::ZERO, |more| more.text_origin);
    for (k, (run, id)) in text.0.runs.iter().zip(ids).enumerate() {
        let mut item = Node::new(Role::TextRun);
        let min = node.rect.min
            + Vec2::new(
                scaled(origin.x + run.rect.min.x),
                scaled(origin.y + run.rect.min.y),
            );
        let size = Vec2::new(scaled(run.rect.size().x), scaled(run.rect.size().y));
        item.set_bounds(bounds(Rect::from_min_size(min, size.max(Vec2::ZERO))));
        item.set_value(run.text.as_str());
        item.set_text_direction(if run.rtl {
            TextDirection::RightToLeft
        } else {
            TextDirection::LeftToRight
        });
        item.set_character_lengths(run.lengths.clone());
        item.set_character_positions(run.positions.iter().map(|x| scaled(*x)).collect::<Vec<_>>());
        item.set_character_widths(run.widths.iter().map(|w| scaled(*w)).collect::<Vec<_>>());
        item.set_word_starts(run.word_starts.clone());
        if run.continues {
            if let Some(previous) = k.checked_sub(1).and_then(|k| ids.get(k)) {
                item.set_previous_on_line(NodeId(*previous));
            }
        }
        if text.0.runs.get(k + 1).is_some_and(|next| next.continues) {
            if let Some(next) = ids.get(k + 1) {
                item.set_next_on_line(NodeId(*next));
            }
        }
        out.push((NodeId(*id), item));
    }
}

/// The native window: the parent of every layer.
pub(super) fn root(title: Option<&str>, size: Vec2, children: &[u64]) -> Node {
    let mut out = Node::new(Role::Window);
    out.set_bounds(bounds(Rect::from_min_size(Vec2::ZERO, size)));
    out.set_children(children.iter().copied().map(NodeId).collect::<Vec<_>>());
    if let Some(title) = title {
        out.set_label(title);
    }
    out
}

/// A live region whose text is the last message of `Context::announce`.
pub(super) fn announcement(announcement: &Announcement, assertive: bool) -> Node {
    let mut out = Node::new(if assertive { Role::Alert } else { Role::Status });
    // A repeated message has to differ for assistive technology to speak it again.
    let mark = if announcement.serial.is_multiple_of(2) {
        "\u{200b}"
    } else {
        ""
    };
    out.set_label(format!("{}{mark}", announcement.text));
    out.set_live(if assertive {
        Live::Assertive
    } else {
        Live::Polite
    });
    out.set_live_atomic();
    out
}
