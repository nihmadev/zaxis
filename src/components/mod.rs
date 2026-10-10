//! Immediate mode components and the shared UI API.
//!
//! Each component owns its builders, rendering, and convenience methods on [`Ui`].

mod actions;
pub mod appearance;
pub mod badge;
pub mod blur;
pub mod button;
mod button_variant;
pub mod card;
pub mod carousel;
pub mod checkbox;
pub mod collapsing_header;
pub mod color_picker;
pub mod columns;
pub mod combo_box;
pub mod context_menu;
pub(crate) mod disclosure;
pub mod dock;
pub mod drag_drop;
pub mod drag_value;
pub(crate) mod edit_buffer;
mod edit_history;
pub mod effects;
pub mod field;
pub mod focus_group;
pub mod grid;
pub mod hover;
pub mod hyperlink;
pub mod icon_tabs;
pub mod image;
pub mod key_box;
pub mod list_box;
pub mod loader;
pub mod material;
pub mod menu_bar;
pub mod modal;
pub(crate) mod motion;
mod moving;
pub mod number_input;
mod numeric;
pub mod pan_zoom;
pub mod popup;
pub mod progress;
pub mod radio;
pub mod reorder;
pub mod response;
pub mod rich_text;
pub mod root;
pub(crate) mod sanitize;
pub mod scroll_area;
pub mod segmented;
pub mod selectable_label;
pub mod selection_scope;
mod sense;
pub mod separator;
pub mod skeleton;
pub mod slider;
pub mod split_pane;
pub mod style;
pub mod switch;
pub mod tab_bar;
pub mod table;
pub mod text;
pub(crate) mod text_block;
pub mod text_edit;
pub(crate) mod text_geometry;
pub mod theme;
pub mod title_bar;
pub mod toast;
pub mod tooltip;
pub mod tree_view;
pub mod ui;
pub mod widget;
pub mod window;

pub use badge::Badge;
pub use blur::Blur;
pub use button::Button;
pub use button_variant::ButtonVariant;
pub use card::{Card, CardOutput, CardVariant};
pub use carousel::{
    Carousel, CarouselIndicator, CarouselOrientation, CarouselOutput, CarouselPage, CarouselStyle,
    CarouselVariant, IndicatorPosition, StackDirection,
};
pub use checkbox::Checkbox;
pub use collapsing_header::{CollapsingHeader, CollapsingOutput};
pub use color_picker::{ColorPicker, ColorPickerType};
pub use columns::{Align, Column, ColumnWidth};
pub use combo_box::{ComboBox, ComboBoxOption, ComboBoxStyle};
pub use context_menu::{
    ContextMenu, ContextMenuItem, ContextMenuOutput, ContextMenuStyle, ContextMenuWidget,
};
pub use disclosure::{CollapsingStyle, DisclosureStyle, TreeStyle};
pub use dock::{
    Dock, DockActions, DockChild, DockDropZone, DockError, DockEvent, DockFloat, DockIssue,
    DockMotion, DockNode, DockOutput, DockPanelOutput, DockPreset, DockSide, DockState, DockStyle,
    DockViewer, PanelId,
};
pub use drag_drop::{
    move_item, DragEffect, DragEnd, DragOutput, DragReason, DragSource, DragStyle, DropOutput,
    DropTarget, DropZones, Dropped, Insertion, PreviewKind, RowDrag, RowMove, TreeNodeDrag,
};
pub use drag_value::DragValue;
pub use effects::Presence;
pub use field::{Field, Validation};
pub use focus_group::{FocusGroup, FocusGroupOutput};
pub use grid::{Grid, GridOutput, GridRow, GridStyle, GridUi};
pub use hover::{Hover, HoverFill, HoverStyle};
pub use hyperlink::{
    Hyperlink, HyperlinkOutput, HyperlinkStyle, LinkActivation, UrlError, UrlPolicy,
};
pub use icon_tabs::IconTabs;
pub use image::{Image, ImageFit, ImageOutput};
pub use key_box::{KeyBinding, KeyBox, MouseBinding};
pub use list_box::{
    ListBox, ListBoxStyle, ListDensity, ListEntry, ListEntryKind, ListEvent, ListMode, ListModel,
    ListOutput, ListRow,
};
pub use loader::Loader;
pub use material::MaterialPaint;
pub use menu_bar::{MenuBar, MenuBarOutput, MenuItem};
pub use modal::{
    CloseReason, Confirm, Confirmation, Dialog, DialogAction, DialogOutput, Modal, ModalAnchor,
    ModalOutput,
};
pub use number_input::{NumberInput, NumberStyle};
pub use numeric::Numeric;
pub use pan_zoom::{PanZoom, PanZoomOutput, PanZoomState, ZoomWheel};
pub use popup::{Popup, PopupOutput};
pub use progress::{Progress, ProgressState};
pub use radio::{RadioGroup, RadioLayout, RadioOption, RadioSize};
pub use reorder::{Reorder, ReorderUi, RowPresence, SelectionIndicator};
pub use response::{Response, WidgetState};
pub use rich_text::{LinkTarget, RichText, Span, SpanStyle, Underline};
pub use root::Root;
pub use scroll_area::{ScrollArea, ScrollAreaOutput, ScrollStyle};
pub use segmented::{
    SegmentOption, SegmentWidth, SegmentedControl, SegmentedOrientation, SegmentedSize,
    SegmentedVariant,
};
pub use selectable_label::{LabelOutput, SelectableLabel};
pub use selection_scope::SelectionScope;
pub use sense::Sense;
pub use separator::Separator;
pub use skeleton::Skeleton;
pub use slider::{Slider, SliderStatus};
pub use split_pane::{
    SplitBoundaryOutput, SplitHandle, SplitHandleStyle, SplitOutput, SplitPane, SplitPanel,
    SplitPanelOutput, SplitSize, SplitStyle, SplitSurface, SplitUi,
};
pub use style::Style;
pub use switch::Switch;
pub use tab_bar::{
    tab_after_close, TabActivation, TabBar, TabBarOutput, TabDrag, TabDropZone, TabDropZoneOutput,
    TabItem, TabMove, TabRelease, TabVariant, TabWidth,
};
pub use table::{SortDirection, SortRequest, Table, TableBody, TableOutput, TableStyle};
pub use text::Text;
pub use text_block::LinkReport;
pub use text_edit::TextEdit;
pub use theme::*;
pub use title_bar::{TitleBar, TitleBarResponse};
pub use toast::Toast;
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
