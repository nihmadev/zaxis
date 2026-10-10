use super::{ControlStyle, SurfaceStyle};
use crate::{Border, Color, FontWeight, Vec2};

/// Look of [`TabBar`](crate::TabBar). Missing properties inherit from the theme tokens
/// (see `Style::tabs`); metrics are logical pixels for the comfortable density.
///
/// Priority: builder ([`TabBar::style`](crate::TabBar::style)), then `Style::tabs`, then
/// the defaults noted below, which derive from the palette so the strip follows the theme.
///
/// States of `tab` and `close` map to the parts of the folder look:
///
/// * `tab.idle`: an inactive tab at rest (`foreground`, the muted label color; no fill).
/// * `tab.hover`: the rounded plate under the pointer (`fill`) and its label color.
/// * `tab.pressed`: the plate while the button is down.
/// * `tab.selected`: the active tab: `fill` is the surface of the page below (the strip's
///   darker fill ends under it), `border` frames it on top, left and right, and `rounding`
///   rounds its top corners.
/// * `tab.disabled`: label color of a disabled tab.
/// * `close.idle`: the glyph at rest; `close.hover`: the plate under the pointer and the
///   glyph color on it; `close.pressed`: the plate while pressed.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TabsStyle {
    /// The band behind the row: `fill` (window background) and `border`, the hairline under
    /// the whole row (the border color, `Style::border`).
    pub strip: SurfaceStyle,
    pub tab: ControlStyle,
    pub close: ControlStyle,
    /// The mark that selects a tab in the underline variant: `fill` (accent).
    pub indicator: SurfaceStyle,
    /// The separator between neighbouring inactive tabs (a hairline in the border color).
    pub divider: Option<Border>,
    /// Color and thickness of the line that shows where a dragged tab will land (the
    /// accent, 2).
    pub insertion_color: Option<Color>,
    pub insertion_width: Option<f32>,
    /// The inset outline that marks the area a dragged tab would land in, on a strip and on a
    /// [`TabDropZone`](crate::TabDropZone): `fill`, `border` and `rounding` (the accent at
    /// about 15%, a 1.5 px accent line, the control radius). It eases in while the tab is
    /// over the area.
    pub drop_area: SurfaceStyle,
    /// How far the tabs behind the insertion point move apart to open a gap (8).
    pub insertion_gap: Option<f32>,
    /// Height of the row, the hairline included (control height + 2, 34).
    pub height: Option<f32>,
    /// Horizontal padding inside a tab (12).
    pub padding_x: Option<f32>,
    /// Side of the square an icon is drawn in (16), and its distance from the label (6).
    pub icon_size: Option<f32>,
    pub icon_gap: Option<f32>,
    /// Side of the close button's hit area and its hover plate (18), the size of the
    /// cross inside (10) and its distance from the label (8).
    pub close_size: Option<f32>,
    pub close_glyph: Option<f32>,
    pub close_gap: Option<f32>,
    /// Lower and upper bound of a tab's width (96 and 240); a tab with a label is never
    /// narrower than the lower bound, and shrinks to it before the row scrolls.
    pub min_width: Option<f32>,
    pub max_width: Option<f32>,
    /// Radius of the active tab's top corners (8) and of the hover plate (6).
    pub tab_radius: Option<f32>,
    pub plate_radius: Option<f32>,
    /// Space between the hover plate and the slot of its tab, horizontal and vertical (2, 4).
    pub plate_inset: Option<Vec2>,
    /// Height of the separator, centered in the row (16).
    pub divider_height: Option<f32>,
    pub font_size: Option<f32>,
    pub font_weight: Option<FontWeight>,
}
