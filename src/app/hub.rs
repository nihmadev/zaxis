//! Window bookkeeping and application callbacks, independent of winit and the GPU so the
//! rules (unique keys, routing, close policy, vetoes, declarations) can be tested directly.

use super::{
    callbacks::{CloseRequested, CloseSource, GlobalShortcut, WindowPlan},
    commands::{Command, Control},
    registry::{Closed, OpenRequest, Reconcile, Registry},
    App, ExitPolicy, WindowKey, WindowStatus, Windows,
};
use crate::SharedResources;
use std::hash::Hash;

/// What happened to a close request.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct CloseOutcome {
    /// The application was asked. False when the window was already closed.
    pub delivered: bool,
    /// The application kept the window open.
    pub rejected: bool,
    pub closed: Closed,
}

pub(crate) struct Hub<I> {
    pub registry: Registry<I>,
    pub control: Control,
}

impl<I: Copy + Eq + Hash> Hub<I> {
    pub fn new(main: WindowKey, policy: ExitPolicy, resources: SharedResources) -> Self {
        Self {
            registry: Registry::new(main.clone(), policy),
            control: Control::new(main, resources),
        }
    }

    /// Register a window before its native counterpart exists.
    pub fn begin_open(
        &mut self,
        key: &WindowKey,
        parent: Option<&WindowKey>,
        declared: bool,
    ) -> OpenRequest {
        let request = self.registry.request(key, parent, declared);
        if request == OpenRequest::Queued {
            self.control
                .status
                .insert(key.clone(), WindowStatus::Pending);
        }
        request
    }

    pub fn opened(&mut self, key: &WindowKey, id: I) {
        self.registry.attach(key, id);
        self.control.status.insert(key.clone(), WindowStatus::Open);
        self.control.stats.windows_opened += 1;
        self.control.stats.windows_open = self.registry.len();
    }

    /// Creation failed: forget the window (and any child waiting for it) and remember why.
    pub fn open_failed(&mut self, key: &WindowKey, message: String) -> Closed {
        let closed = self.forget(key);
        self.control.stats.windows_failed += 1;
        self.control
            .status
            .insert(key.clone(), WindowStatus::Failed(message));
        closed
    }

    /// Close `key` and its children without asking. Repeating it removes nothing.
    pub fn close(&mut self, key: &WindowKey) -> Closed {
        let closed = self.forget(key);
        self.control.stats.windows_closed += closed.keys.len() as u64;
        closed
    }

    fn forget(&mut self, key: &WindowKey) -> Closed {
        let closed = self.registry.close(key);
        for key in &closed.keys {
            self.control.status.remove(key);
            self.control.infos.remove(key);
        }
        self.control.stats.windows_open = self.registry.len();
        closed
    }

    /// Deliver a close request to the application and close the window unless it objects.
    pub fn close_requested(
        &mut self,
        app: &mut (impl App + ?Sized),
        key: &WindowKey,
        source: CloseSource,
    ) -> CloseOutcome {
        if !self.registry.contains(key) {
            return CloseOutcome::default();
        }
        let mut request = CloseRequested::new(key, source, &mut self.control);
        app.close_requested(&mut request);
        if request.rejected {
            return CloseOutcome {
                delivered: true,
                rejected: true,
                closed: Closed::default(),
            };
        }
        CloseOutcome {
            delivered: true,
            rejected: false,
            closed: self.close(key),
        }
    }

    /// Offer a key press to the application before the focused window sees it.
    pub fn shortcut(
        &mut self,
        app: &mut (impl App + ?Sized),
        shortcut: &GlobalShortcut<'_>,
    ) -> bool {
        app.global_shortcut(
            shortcut,
            &mut Windows {
                control: &mut self.control,
            },
        )
    }

    /// Ask the application which windows it wants and queue the difference. Windows are
    /// requested in declaration order, so a child can be declared after its parent.
    pub fn declare(&mut self, app: &mut (impl App + ?Sized)) -> Reconcile {
        let mut plan = WindowPlan::default();
        app.windows(&mut plan);
        let declared: Vec<_> = plan
            .declared
            .iter()
            .map(|(key, options)| (key.clone(), options.parent.clone()))
            .collect();
        let result = self.registry.reconcile(&declared);
        let mut windows = Windows {
            control: &mut self.control,
        };
        for (key, options) in plan.declared {
            if result.open.contains(&key) {
                windows.request(key, options, true);
            }
        }
        for key in &result.close {
            self.control.commands.push(Command::Close(key.clone()));
        }
        result
    }

    pub fn take_commands(&mut self) -> Vec<Command> {
        std::mem::take(&mut self.control.commands)
    }
}
