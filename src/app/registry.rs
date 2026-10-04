//! The set of live windows: keys, platform ids, parents and the exit policy. No winit, no GPU.

use super::{ExitPolicy, WindowKey};
use std::{collections::HashSet, hash::Hash};

struct Entry<I> {
    key: WindowKey,
    id: Option<I>,
    parent: Option<WindowKey>,
    /// Opened because the application declared it, so it closes when no longer declared.
    declared: bool,
    /// Attached to a native window.
    open: bool,
}

/// Result of asking for a window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OpenRequest {
    /// New entry; the caller creates the native window.
    Queued,
    /// The key is open or already requested; nothing to do, nothing duplicated.
    Exists,
    UnknownParent,
}

/// Windows removed by one close and whether the application ends with them.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Closed {
    pub keys: Vec<WindowKey>,
    pub exit: bool,
}

/// What a declaration pass changes.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Reconcile {
    pub open: Vec<WindowKey>,
    pub close: Vec<WindowKey>,
}

pub(crate) struct Registry<I> {
    entries: Vec<Entry<I>>,
    /// Declared windows that closed (by the user, by failing, by their parent) while still
    /// declared. They stay closed until the application stops declaring them once.
    latched: HashSet<WindowKey>,
    main: WindowKey,
    policy: ExitPolicy,
}

impl<I: Copy + Eq + Hash> Registry<I> {
    pub fn new(main: WindowKey, policy: ExitPolicy) -> Self {
        Self {
            entries: Vec::new(),
            latched: HashSet::new(),
            main,
            policy,
        }
    }

    pub fn is_main(&self, key: &WindowKey) -> bool {
        *key == self.main
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn contains(&self, key: &WindowKey) -> bool {
        self.entries.iter().any(|e| e.key == *key)
    }

    #[cfg(test)]
    pub fn is_open(&self, key: &WindowKey) -> bool {
        self.entries.iter().any(|e| e.key == *key && e.open)
    }

    /// Register a window that is about to be created. A key that is already present is
    /// never registered twice, so one request yields at most one native window.
    pub fn request(
        &mut self,
        key: &WindowKey,
        parent: Option<&WindowKey>,
        declared: bool,
    ) -> OpenRequest {
        if self.contains(key) {
            return OpenRequest::Exists;
        }
        if parent.is_some_and(|parent| !self.contains(parent)) {
            return OpenRequest::UnknownParent;
        }
        self.latched.remove(key);
        self.entries.push(Entry {
            key: key.clone(),
            id: None,
            parent: parent.cloned(),
            declared,
            open: false,
        });
        OpenRequest::Queued
    }

    /// The native window now exists.
    pub fn attach(&mut self, key: &WindowKey, id: I) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.key == *key) {
            entry.id = Some(id);
            entry.open = true;
        }
    }

    /// The system dropped every native window (suspension); keys and parents stay.
    pub fn detach_all(&mut self) {
        for entry in &mut self.entries {
            entry.id = None;
            entry.open = false;
        }
    }

    /// The window an event is addressed to; events for unknown ids belong to no window.
    pub fn route(&self, id: I) -> Option<&WindowKey> {
        self.entries
            .iter()
            .find(|e| e.id == Some(id))
            .map(|e| &e.key)
    }

    /// Remove `key` and its descendants. Closing the main window ends the application under
    /// [`ExitPolicy::MainWindow`], taking every window with it; the application also ends
    /// when the last window closes. Closing a key that is not registered removes nothing,
    /// so a repeated close is harmless.
    pub fn close(&mut self, key: &WindowKey) -> Closed {
        if !self.contains(key) {
            return Closed::default();
        }
        let ends = self.policy == ExitPolicy::MainWindow && self.is_main(key);
        let mut doomed = vec![key.clone()];
        let mut next = 0;
        while next < doomed.len() {
            let parent = doomed[next].clone();
            let children: Vec<_> = self
                .entries
                .iter()
                .filter(|e| e.parent.as_ref() == Some(&parent) && !doomed.contains(&e.key))
                .map(|e| e.key.clone())
                .collect();
            doomed.extend(children);
            next += 1;
        }
        if ends {
            doomed = self.entries.iter().map(|e| e.key.clone()).collect();
        }
        for entry in self.entries.iter().filter(|e| doomed.contains(&e.key)) {
            if entry.declared {
                self.latched.insert(entry.key.clone());
            }
        }
        self.entries.retain(|e| !doomed.contains(&e.key));
        Closed {
            keys: doomed,
            exit: ends || self.entries.is_empty(),
        }
    }

    /// Compare the declared windows with the open ones. Windows declared for the first time
    /// are opened; windows that were opened by a declaration and are no longer declared are
    /// closed. Windows opened imperatively are never touched.
    pub fn reconcile(&mut self, declared: &[(WindowKey, Option<WindowKey>)]) -> Reconcile {
        let wanted: HashSet<&WindowKey> = declared.iter().map(|(key, _)| key).collect();
        self.latched.retain(|key| wanted.contains(key));
        let mut open: Vec<WindowKey> = Vec::new();
        for (key, _) in declared {
            if !self.contains(key) && !self.latched.contains(key) && !open.contains(key) {
                open.push(key.clone());
            }
        }
        let close = self
            .entries
            .iter()
            .filter(|e| e.declared && !wanted.contains(&e.key))
            .map(|e| e.key.clone())
            .collect();
        Reconcile { open, close }
    }
}
