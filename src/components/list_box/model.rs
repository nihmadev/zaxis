use crate::Id;
use std::hash::Hash;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListEntryKind {
    /// A selectable row built by the application.
    Item,
    /// A section title drawn by the list; it sticks to the top while its section scrolls.
    Header,
    /// A thin divider drawn by the list.
    Separator,
}

/// What the list needs to know about one row without building it: identity, kind,
/// availability and the text used for type-ahead (and by [`ListBox::show_text`]).
#[derive(Clone, Copy, Debug)]
pub struct ListEntry<'a> {
    pub key: Id,
    pub kind: ListEntryKind,
    pub enabled: bool,
    pub text: &'a str,
}
impl<'a> ListEntry<'a> {
    /// `key` must be stable across insertions, removals and reordering.
    pub fn item(key: impl Hash, text: &'a str) -> Self {
        Self {
            key: Id::new(key),
            kind: ListEntryKind::Item,
            enabled: true,
            text,
        }
    }
    pub fn header(key: impl Hash, text: &'a str) -> Self {
        Self {
            kind: ListEntryKind::Header,
            ..Self::item(key, text)
        }
    }
    pub fn separator(key: impl Hash) -> Self {
        Self {
            kind: ListEntryKind::Separator,
            ..Self::item(key, "")
        }
    }
    /// A disabled row is skipped by the keyboard, cannot be selected and ignores the pointer.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    pub(crate) fn selectable(&self) -> bool {
        self.enabled && self.kind == ListEntryKind::Item
    }
}

/// The application's rows. Only the rows near the viewport are ever asked for, except
/// when the list sees a new [`revision`](Self::revision), which scans the entries once.
pub trait ListModel {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn entry(&self, index: usize) -> ListEntry<'_>;
    /// Change this whenever entries change without the length changing (a re-sort, a
    /// filter with the same number of matches). Length changes are always noticed.
    fn revision(&self) -> u64 {
        0
    }
}

/// `count` rows described by a closure; see [`ListBox::show_rows`].
pub(super) struct FnModel<'a, F> {
    pub len: usize,
    pub revision: u64,
    pub entry: F,
    pub borrowed: std::marker::PhantomData<&'a ()>,
}
impl<'a, F: Fn(usize) -> ListEntry<'a>> ListModel for FnModel<'a, F> {
    fn len(&self) -> usize {
        self.len
    }
    fn entry(&self, index: usize) -> ListEntry<'_> {
        (self.entry)(index)
    }
    fn revision(&self) -> u64 {
        self.revision
    }
}

/// A slice with an entry function; see [`ListBox::show_slice`].
pub(super) struct SliceModel<'a, T, F> {
    pub items: &'a [T],
    pub revision: u64,
    pub entry: F,
}
impl<T, F: for<'x> Fn(&'x T) -> ListEntry<'x>> ListModel for SliceModel<'_, T, F> {
    fn len(&self) -> usize {
        self.items.len()
    }
    fn entry(&self, index: usize) -> ListEntry<'_> {
        (self.entry)(&self.items[index])
    }
    fn revision(&self) -> u64 {
        self.revision
    }
}
