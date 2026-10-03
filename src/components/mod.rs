//! Immediate mode components and the shared UI API.
//!
//! Each component owns its builders, rendering, and convenience methods on [`Ui`].

pub(crate) mod appearance;
pub mod blur;
pub mod button;
pub mod checkbox;
pub mod collapsing_header;
pub mod color_picker;
pub mod columns;
pub mod combo_box;
pub mod context_menu;
pub(crate) mod disclosure;
pub mod drag_value;
pub(crate) mod edit_buffer;
mod edit_history;
pub mod effects;
pub mod grid;
pub mod hover;
pub mod image;
pub mod loader;
pub(crate) mod motion;
pub mod number_input;
mod numeric;
pub mod popup;
pub mod progress;
pub mod reorder;
pub mod response;
pub mod root;
pub mod scroll_area;
pub mod separator;
pub mod slider;
pub mod split_pane;
pub mod style;
pub mod table;
pub mod text;
pub mod text_edit;
pub mod theme;
pub mod title_bar;
pub mod tooltip;
pub mod tree_view;
pub mod ui;
pub mod widget;
pub mod window;

pub use blur::Blur;
pub use button::Button;
pub use checkbox::Checkbox;
pub use collapsing_header::{CollapsingHeader, CollapsingOutput};
pub use color_picker::{ColorPicker, ColorPickerType};
pub use columns::{Align, Column, ColumnWidth};
pub use combo_box::{ComboBox, ComboBoxOption, ComboBoxStyle};
pub use context_menu::{ContextMenu, ContextMenuItem, ContextMenuOutput, ContextMenuStyle};
pub use disclosure::{CollapsingStyle, DisclosureStyle, TreeStyle};
pub use drag_value::DragValue;
pub use effects::Presence;
pub use grid::{Grid, GridOutput, GridRow, GridStyle, GridUi};
pub use hover::{Hover, HoverFill, HoverStyle};
pub use image::{Image, ImageFit, ImageOutput};
pub use loader::Loader;
pub use number_input::{NumberInput, NumberStyle};
pub use numeric::Numeric;
pub use popup::{Popup, PopupOutput};
pub use progress::{Progress, ProgressState};
pub use reorder::{Reorder, ReorderUi, SelectionIndicator};
pub use response::{Response, WidgetState};
pub use root::Root;
pub use scroll_area::{ScrollArea, ScrollAreaOutput, ScrollStyle};
pub use separator::Separator;
pub use slider::{Slider, SliderStatus};
pub use split_pane::{
    SplitBoundaryOutput, SplitHandle, SplitHandleStyle, SplitOutput, SplitPane, SplitPanel,
    SplitPanelOutput, SplitSize, SplitStyle, SplitSurface, SplitUi,
};
pub use style::Style;
pub use table::{SortDirection, SortRequest, Table, TableBody, TableOutput, TableStyle};
pub use text::Text;
pub use text_edit::TextEdit;
pub use theme::*;
pub use title_bar::{TitleBar, TitleBarResponse};
pub use tooltip::{Tooltip, TooltipStyle, TooltipWidget};
pub use tree_view::{
    TreeChildren, TreeEvent, TreeIssue, TreeModel, TreeNode, TreeOutput, TreeView,
};
pub use ui::Ui;
pub use widget::Widget;
pub use window::Window;

fn visible_label(text: &str) -> &str {
    text.split_once("##").map_or(text, |(visible, _)| visible)
}
fn font_size(size: f32) -> f32 {
    size.max(1.0).clamp(1.0, 256.0)
}
