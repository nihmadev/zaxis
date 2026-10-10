use std::hash::Hash;

use super::{
    item::TabItem,
    options::{Config, TabActivation, TabBarOutput, TabVariant, TabWidth},
    show,
};
use crate::{
    components::{sanitize, theme::TabsStyle},
    Id, Ui,
};

type Trailing<'a> = Box<dyn FnOnce(&mut Ui<'_>) + 'a>;

/// A strip of tabs bound to a selection, in the manner of a browser or an editor: an icon,
/// a label and a close button per tab, tabs that can be dragged to a new place or to
/// another strip, and a hairline under the row. It edits an application-owned `&mut T`;
/// each tab is matched by value, with an identity taken from the value's hash, so
/// reordering, renaming and moving tabs keep their focus and animation.
///
/// The library never changes the application's model. Closing a tab and moving it arrive
/// as events in the [`TabBarOutput`]; the application removes or reorders its tabs, and
/// the next pass shows the result. [`tab_after_close`](crate::tab_after_close) picks the
/// tab to select when the active one is closed.
///
/// Sizing: a tab is as wide as its content between the minimum and maximum width
/// ([`TabWidth`]). A row that does not fit shrinks its tabs evenly down to the minimum,
/// cutting labels with an ellipsis (the tooltip shows the whole label), and only then
/// scrolls, revealing the selected, focused or dragged tab.
///
/// Keyboard: the strip is a single Tab stop. Tab lands on the selected tab; Left and Right
/// move the focus (and, with [`TabActivation::OnFocus`], the selection), Home and End jump
/// to the ends, disabled tabs are skipped and nothing wraps. Delete closes the focused
/// tab. Ctrl+Shift with Left or Right moves it one place. Ctrl+Space picks it up for a
/// keyboard drag.
///
/// ```no_run
/// # fn view(ui: &mut zaxis::Ui<'_>) {
/// # use zaxis::{TabBar, TabItem};
/// # let mut open = vec!["Files", "Records"];
/// # let mut page = "Files";
/// let out = TabBar::new("tabs", &mut page, open.iter().map(|name| TabItem::new(*name, *name)))
///     .closable(true)
///     .reorderable(true)
///     .show(ui);
/// if let Some(closed) = out.closed {
///     open.retain(|name| zaxis::Id::new(name) != closed);
/// }
/// # }
/// ```
pub struct TabBar<'a, T> {
    pub(super) selected: &'a mut T,
    pub(super) tabs: Vec<TabItem<T>>,
    pub(super) cfg: Config,
    pub(super) trailing: Option<Trailing<'a>>,
}

impl<'a, T: PartialEq + Hash> TabBar<'a, T> {
    /// `id_source` names the strip among its siblings (and in [`TabMove::from_bar`](crate::TabMove)
    /// as `Id::new(id_source)`). Tabs are [`TabItem`]s or `(value, label)` and
    /// `(value, label, icon)` tuples.
    pub fn new<I: Into<TabItem<T>>>(
        id_source: impl Hash,
        selected: &'a mut T,
        tabs: impl IntoIterator<Item = I>,
    ) -> Self {
        Self {
            selected,
            tabs: tabs.into_iter().map(Into::into).collect(),
            cfg: Config::new(Id::new(id_source)),
            trailing: None,
        }
    }

    /// [`TabVariant::Folder`] by default.
    pub fn variant(mut self, variant: TabVariant) -> Self {
        self.cfg.variant = variant;
        self
    }
    /// Give every tab a close button (off by default); [`TabItem::closable`] overrides it.
    pub fn closable(mut self, closable: bool) -> Self {
        self.cfg.closable = closable;
        self
    }
    /// [`TabWidth::Content`] by default.
    #[track_caller]
    pub fn width(mut self, width: TabWidth) -> Self {
        match width {
            TabWidth::Fixed(w) => {
                if let Some(w) = sanitize::positive("TabBar::width", w) {
                    self.cfg.width = TabWidth::Fixed(w);
                }
            }
            other => self.cfg.width = other,
        }
        self
    }
    /// Lower bound of a labelled tab's width before the row scrolls (96).
    #[track_caller]
    pub fn min_width(mut self, width: f32) -> Self {
        self.cfg.min_width =
            sanitize::non_negative("TabBar::min_width", width).or(self.cfg.min_width);
        self
    }
    /// Upper bound of a tab's width (240).
    #[track_caller]
    pub fn max_width(mut self, width: f32) -> Self {
        self.cfg.max_width =
            sanitize::non_negative("TabBar::max_width", width).or(self.cfg.max_width);
        self
    }
    /// [`TabActivation::OnFocus`] by default.
    pub fn activation(mut self, activation: TabActivation) -> Self {
        self.cfg.activation = activation;
        self
    }
    /// Strips with the same group exchange tabs by dragging. By default a strip has a group
    /// of its own, so tabs only change places inside it.
    pub fn group(mut self, group: impl Hash) -> Self {
        self.cfg.group = Some(Id::new(group));
        self
    }
    /// Let the user drag the tabs of this strip, and move the focused one with
    /// Ctrl+Shift and the arrow keys (off by default). A strip accepts tabs of its group
    /// whether or not its own can be dragged.
    pub fn reorderable(mut self, reorderable: bool) -> Self {
        self.cfg.reorderable = reorderable;
        self
    }
    /// For a window without system decorations: the free space of the strip drags the
    /// native window, like a [`TitleBar`](crate::TitleBar). Off by default.
    pub fn window_drag(mut self, drag: bool) -> Self {
        self.cfg.window_drag = drag;
        self
    }
    /// Close a focused tab with Delete (default: true). Disable this when an enclosing
    /// workspace supplies application actions; pointer and accessibility closing remain.
    pub fn keyboard_close(mut self, enabled: bool) -> Self {
        self.cfg.keyboard_close = enabled;
        self
    }
    /// Content at the right end of the strip, fixed there while the tabs scroll: an add
    /// button, for example. It is built every pass by the application.
    pub fn trailing(mut self, build: impl FnOnce(&mut Ui<'_>) + 'a) -> Self {
        self.trailing = Some(Box::new(build));
        self
    }
    /// A button after a scrolling row that lists every tab in a popup (off by default).
    pub fn overflow_menu(mut self, menu: bool) -> Self {
        self.cfg.overflow_menu = menu;
        self
    }
    /// Component style; it overrides `Style::tabs`.
    pub fn style(mut self, style: TabsStyle) -> Self {
        self.cfg.style = style;
        self
    }

    /// Show the strip.
    pub fn show(self, ui: &mut Ui<'_>) -> TabBarOutput {
        show::run(self, ui)
    }
}

impl Ui<'_> {
    /// Tabs with identities derived from their values, in the default look: a folder strip
    /// whose tabs fit their labels (see [`TabBar`] for every option). When the tabs do not
    /// fit they shrink to their minimum width and then the strip scrolls horizontally
    /// (wheel, touchpad, thumb), revealing a tab when it is clicked. Returns one response
    /// per tab; `changed()` marks the one the user selected.
    pub fn tab_bar<T: PartialEq + Hash, L: Into<String>>(
        &mut self,
        selected: &mut T,
        tabs: impl IntoIterator<Item = (T, L)>,
    ) -> Vec<crate::Response> {
        let tabs: Vec<TabItem<T>> = tabs.into_iter().map(Into::into).collect();
        let first = tabs.first().map(TabItem::key);
        TabBar::new(first, selected, tabs).show(self).responses
    }
}
