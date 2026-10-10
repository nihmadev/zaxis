//! Typed themes resolve into the existing Style consumed by immediate mode UI.
mod controls;
mod modal;
mod overrides;
pub mod painter;
mod palette;
mod resolve;
mod tabs;
mod tokens;

pub use controls::*;
pub use modal::ModalStyle;
pub use overrides::*;
pub use painter::{ControlPaint, PaintMode, PaintPart, Painter};
pub use tabs::TabsStyle;
pub use tokens::*;

/// Theme inputs. `None` in overrides inherits; every `Some`, including zero,
/// is explicit. Resolve once or install with Context::set_theme.
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    pub palette: Palette,
    pub metrics: Metrics,
    pub typography: Typography,
    pub density: Density,
    pub overrides: StyleOverrides,
}
impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}
impl Theme {
    pub fn dark() -> Self {
        Self {
            palette: Palette::dark(),
            metrics: Metrics::default(),
            typography: Typography::default(),
            density: Density::Comfortable,
            overrides: StyleOverrides::default(),
        }
    }
    pub fn light() -> Self {
        Self {
            palette: Palette::light(),
            ..Self::dark()
        }
    }
    pub fn high_contrast() -> Self {
        Self {
            palette: Palette::high_contrast(),
            ..Self::dark()
        }
    }
    pub fn accent(mut self, color: crate::Color) -> Self {
        self.palette.accent = color;
        self.palette.on_accent = contrast_foreground(color);
        self
    }
    pub fn density(mut self, density: Density) -> Self {
        self.density = density;
        self
    }
    pub fn resolve(&self) -> super::Style {
        self.resolve_style()
    }
}
