pub use crate::layout::Align;
use crate::Id;
use std::hash::Hash;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColumnWidth {
    Fixed(f32),
    Content,
    Remainder(f32),
}

/// Shared column policy for Grid and Table. Widths include cell padding.
#[derive(Clone, Debug, PartialEq)]
pub struct Column {
    pub id: Id,
    pub width: ColumnWidth,
    pub minimum: f32,
    pub horizontal: Align,
    pub vertical: Align,
    pub(crate) title: String,
    pub(crate) sortable: bool,
    pub(crate) resizable: bool,
    pub(crate) tabular: bool,
}
impl Column {
    #[track_caller]
    pub fn new(source: impl Hash, width: ColumnWidth) -> Self {
        let width = match width {
            ColumnWidth::Fixed(n) => ColumnWidth::Fixed(dimension(n)),
            ColumnWidth::Remainder(n) => {
                ColumnWidth::Remainder(super::sanitize::weight("Column weight", n))
            }
            ColumnWidth::Content => width,
        };
        Self {
            id: Id::new(source),
            width,
            minimum: 0.0,
            horizontal: Align::Start,
            vertical: Align::Center,
            title: String::new(),
            sortable: false,
            resizable: true,
            tabular: false,
        }
    }
    #[track_caller]
    pub fn fixed(source: impl Hash, width: f32) -> Self {
        Self::new(source, ColumnWidth::Fixed(width))
    }
    pub fn content(source: impl Hash) -> Self {
        Self::new(source, ColumnWidth::Content)
    }
    pub fn remainder(source: impl Hash) -> Self {
        Self::new(source, ColumnWidth::Remainder(1.0))
    }
    #[track_caller]
    pub fn min_width(mut self, width: f32) -> Self {
        self.minimum = dimension(width);
        self
    }
    pub fn align(mut self, horizontal: Align, vertical: Align) -> Self {
        self.horizontal = horizontal;
        self.vertical = vertical;
        self
    }
    /// A column of numbers: right-aligned cells whose digits are tabular, so decimal
    /// places line up. Text in cells inherits it through the cell's text style.
    pub fn numeric(mut self, numeric: bool) -> Self {
        self.tabular = numeric;
        self.horizontal = if numeric { Align::End } else { Align::Start };
        self
    }
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }
    pub fn sortable(mut self, sortable: bool) -> Self {
        self.sortable = sortable;
        self
    }
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }
}

/// A non-negative length; invalid values become zero.
#[track_caller]
pub(crate) fn dimension(value: f32) -> f32 {
    super::sanitize::length("size", value)
}

/// Resolve shared widths without shrinking fixed/content columns below their minima.
/// If minima exceed the viewport, the layout overflows and its container clips/scrolls.
pub fn resolve(columns: &[Column], measured: &[f32], available: f32, spacing: f32) -> Vec<f32> {
    let mut widths: Vec<_> = columns
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let preferred = match c.width {
                ColumnWidth::Fixed(n) => dimension(n),
                ColumnWidth::Content => measured.get(i).copied().unwrap_or(0.0),
                ColumnWidth::Remainder(_) => 0.0,
            };
            preferred.max(dimension(c.minimum))
        })
        .collect();
    let gaps = spacing.max(0.0) * columns.len().saturating_sub(1) as f32;
    let spare = (available - gaps - widths.iter().sum::<f32>()).max(0.0);
    let weight: f64 = columns
        .iter()
        .map(|c| match c.width {
            ColumnWidth::Remainder(n) if n.is_finite() && n > 0.0 => n as f64,
            ColumnWidth::Remainder(_) => 1.0,
            _ => 0.0,
        })
        .sum();
    if weight > 0.0 {
        for (c, width) in columns.iter().zip(&mut widths) {
            if let ColumnWidth::Remainder(n) = c.width {
                let n = if n.is_finite() && n > 0.0 { n } else { 1.0 };
                *width += (spare as f64 * n as f64 / weight) as f32;
            }
        }
    }
    widths
}
