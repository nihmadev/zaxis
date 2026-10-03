//! Single-pass multi-panel layout. See `docs/content/docs/components/split-pane.mdx`.
mod allocation;
mod interaction;
mod paint;
mod show;
mod style;

use super::Ui;
use crate::{Id, Layout, Rect, Vec2};
pub(crate) use interaction::{SplitInput, SplitState};
use std::hash::Hash;
pub use style::{SplitHandle, SplitHandleStyle, SplitStyle, SplitSurface};

/// Initial preference, or an authoritative preference when passed to `size`.
/// Pixels stay fixed across viewport changes; Fraction and Weight are flexible.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SplitSize {
    Pixels(f32),
    Fraction(f32),
    Weight(f32),
}
impl SplitSize {
    pub(crate) fn validate(self) {
        match self {
            Self::Pixels(n) | Self::Fraction(n) => {
                allocation::dimension(n);
            }
            Self::Weight(n) => assert!(
                n.is_finite() && n > 0.0,
                "split weight must be positive and finite"
            ),
        }
    }
}

/// Stable panel identity and sizing policy. Sizes include border and padding.
#[derive(Clone, Debug)]
pub struct SplitPanel {
    pub id: Id,
    pub initial: SplitSize,
    pub controlled: Option<SplitSize>,
    pub minimum: f32,
    pub maximum: Option<f32>,
    /// Enables the boundary following this panel; the final panel has no boundary.
    pub resizable_after: bool,
    pub surface: Option<SplitSurface>,
    pub handle: Option<SplitHandleStyle>,
}
impl SplitPanel {
    pub fn new(source: impl Hash) -> Self {
        Self {
            id: Id::new(source),
            initial: SplitSize::Weight(1.0),
            controlled: None,
            minimum: 0.0,
            maximum: None,
            resizable_after: true,
            surface: None,
            handle: None,
        }
    }
    pub fn default_size(mut self, size: SplitSize) -> Self {
        size.validate();
        self.initial = size;
        self
    }
    /// Applied every pass. Resize proposes sizes in the output; the application
    /// must feed those back to accept them. External changes rebase a live drag.
    pub fn size(mut self, size: SplitSize) -> Self {
        size.validate();
        self.controlled = Some(size);
        self
    }
    pub fn min_size(mut self, size: f32) -> Self {
        self.minimum = allocation::dimension(size);
        self
    }
    pub fn max_size(mut self, size: f32) -> Self {
        self.maximum = Some(allocation::dimension(size));
        self
    }
    pub fn resizable_after(mut self, enabled: bool) -> Self {
        self.resizable_after = enabled;
        self
    }
    pub fn surface(mut self, surface: SplitSurface) -> Self {
        self.surface = Some(surface);
        self
    }
    pub fn handle(mut self, handle: SplitHandleStyle) -> Self {
        self.handle = Some(handle);
        self
    }
}

pub struct SplitPane {
    id: Id,
    axis: Layout,
    panels: Vec<SplitPanel>,
    style: Option<SplitStyle>,
    gap: Option<f32>,
    handle: Option<SplitHandleStyle>,
    size: Option<Vec2>,
    resizable: bool,
    reset: bool,
    double_click_reset: bool,
}
impl SplitPane {
    pub fn horizontal(source: impl Hash) -> Self {
        Self::new(source, Layout::Horizontal)
    }
    pub fn vertical(source: impl Hash) -> Self {
        Self::new(source, Layout::Vertical)
    }
    pub fn new(source: impl Hash, axis: Layout) -> Self {
        Self {
            id: Id::new(source),
            axis,
            panels: Vec::new(),
            style: None,
            gap: None,
            handle: None,
            size: None,
            resizable: true,
            reset: false,
            double_click_reset: false,
        }
    }
    pub fn panels(mut self, panels: impl IntoIterator<Item = SplitPanel>) -> Self {
        self.panels.extend(panels);
        self
    }
    pub fn panel(mut self, panel: SplitPanel) -> Self {
        self.panels.push(panel);
        self
    }
    pub fn style(mut self, style: SplitStyle) -> Self {
        self.style = Some(style);
        self
    }
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = Some(allocation::dimension(gap));
        self
    }
    pub fn handle(mut self, handle: SplitHandleStyle) -> Self {
        self.handle = Some(handle);
        self
    }
    /// Defaults to the remaining Ui rectangle; explicit size is capped to it.
    pub fn size(mut self, size: Vec2) -> Self {
        allocation::dimension(size.x);
        allocation::dimension(size.y);
        self.size = Some(size);
        self
    }
    /// Disables only boundary interaction; child controls remain enabled.
    pub fn resizable(mut self, enabled: bool) -> Self {
        self.resizable = enabled;
        self
    }
    /// Level-triggered reset of retained preferences. Controlled values still win.
    pub fn reset(mut self, reset: bool) -> Self {
        self.reset = reset;
        self
    }
    pub fn double_click_reset(mut self, enabled: bool) -> Self {
        self.double_click_reset = enabled;
        self
    }
}

#[derive(Clone, Debug)]
pub struct SplitPanelOutput {
    pub id: Id,
    pub bounds: Rect,
    pub content_bounds: Rect,
    pub size: f32,
}
#[derive(Clone, Debug)]
pub struct SplitBoundaryOutput {
    pub id: Id,
    pub before: Id,
    pub after: Id,
    pub bounds: Rect,
    pub enabled: bool,
    pub hovered: bool,
    pub focused: bool,
    pub resize_started: bool,
    pub changed: bool,
    pub resize_ended: bool,
    pub cancelled: bool,
}
pub struct SplitOutput<R> {
    pub inner: R,
    pub rect: Rect,
    pub panels: Vec<SplitPanelOutput>,
    pub boundaries: Vec<SplitBoundaryOutput>,
    pub resize_started: bool,
    /// Includes viewport, topology, constraints, external sizes and resets.
    pub changed: bool,
    pub resize_ended: bool,
}

/// Build content by stable panel Id, in any order, at most once per panel.
pub struct SplitUi<'a, 'ctx> {
    ui: &'a mut Ui<'ctx>,
    id: Id,
    specs: &'a [SplitPanel],
    outputs: &'a [SplitPanelOutput],
    style: SplitStyle,
    built: std::collections::HashSet<Id>,
}
impl Ui<'_> {
    pub fn split_horizontal<R>(
        &mut self,
        source: impl Hash,
        panels: impl IntoIterator<Item = SplitPanel>,
        build: impl FnOnce(&mut SplitUi<'_, '_>) -> R,
    ) -> SplitOutput<R> {
        SplitPane::horizontal(source)
            .panels(panels)
            .show(self, build)
    }
    pub fn split_vertical<R>(
        &mut self,
        source: impl Hash,
        panels: impl IntoIterator<Item = SplitPanel>,
        build: impl FnOnce(&mut SplitUi<'_, '_>) -> R,
    ) -> SplitOutput<R> {
        SplitPane::vertical(source).panels(panels).show(self, build)
    }
}
