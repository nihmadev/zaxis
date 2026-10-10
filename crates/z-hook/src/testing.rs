//! A scripted [`PresentBackend`] for tests of hosts and of the driver: records the order of
//! calls and fails on demand. Not a stable API.

use crate::backend::{BackendError, PresentBackend, SurfaceInfo};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use zaxis::DrawData;

/// One call the driver made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Call {
    Begin,
    /// `render` with the revision of the draw data and whether it holds geometry.
    Render {
        revision: u64,
        geometry: bool,
    },
    End,
}

/// A backend over a backbuffer that does not exist.
pub struct MockBackend {
    pub size: [u32; 2],
    pub scale_factor: Option<f64>,
    pub calls: Vec<Call>,
    pub begin_error: Option<BackendError>,
    pub render_error: Option<BackendError>,
    /// Time added to the shared clock by `begin`, and by `render`.
    pub begin_cost: Duration,
    pub render_cost: Duration,
    pub clock: Option<FakeClock>,
    /// Called inside `render`, for tests that re-enter or remove hooks mid-frame.
    pub during_render: Option<Box<dyn FnMut() + Send>>,
    pub panic_in_render: bool,
}

impl MockBackend {
    pub fn new(size: [u32; 2]) -> Self {
        Self {
            size,
            scale_factor: None,
            calls: Vec::new(),
            begin_error: None,
            render_error: None,
            begin_cost: Duration::ZERO,
            render_cost: Duration::ZERO,
            clock: None,
            during_render: None,
            panic_in_render: false,
        }
    }

    pub fn renders(&self) -> usize {
        self.calls
            .iter()
            .filter(|call| matches!(call, Call::Render { .. }))
            .count()
    }

    pub fn clear(&mut self) {
        self.calls.clear();
    }
}

impl PresentBackend for MockBackend {
    fn begin(&mut self) -> Result<SurfaceInfo, BackendError> {
        self.calls.push(Call::Begin);
        if let Some(clock) = &self.clock {
            clock.advance(self.begin_cost);
        }
        if let Some(error) = self.begin_error.clone() {
            return Err(error);
        }
        Ok(SurfaceInfo {
            size: self.size,
            scale_factor: self.scale_factor,
        })
    }

    fn render(&mut self, data: &DrawData) -> Result<(), BackendError> {
        self.calls.push(Call::Render {
            revision: data.revision,
            geometry: !data.indices.is_empty(),
        });
        if let Some(clock) = &self.clock {
            clock.advance(self.render_cost);
        }
        if let Some(hook) = &mut self.during_render {
            hook();
        }
        assert!(!self.panic_in_render, "mock render panic");
        match self.render_error.clone() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    fn end(&mut self) -> Result<(), BackendError> {
        self.calls.push(Call::End);
        Ok(())
    }
}

/// A clock that only moves when told to.
#[derive(Clone)]
pub struct FakeClock {
    base: Instant,
    offset: Arc<Mutex<Duration>>,
}

impl Default for FakeClock {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeClock {
    pub fn new() -> Self {
        Self {
            base: Instant::now(),
            offset: Arc::default(),
        }
    }

    pub fn advance(&self, by: Duration) {
        *self.offset.lock().unwrap() += by;
    }

    pub fn now(&self) -> Instant {
        self.base + *self.offset.lock().unwrap()
    }

    /// The closure form an overlay takes.
    pub fn as_clock(&self) -> Arc<dyn Fn() -> Instant + Send + Sync> {
        let clock = self.clone();
        Arc::new(move || clock.now())
    }
}
