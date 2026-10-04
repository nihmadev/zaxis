//! Immediate mode desktop UI with retained interaction state and cached geometry.
//!
//! Implement [`App::update`] and call [`run`] to open a desktop window.
//! The runner handles events, repaint deadlines, and GPU recovery.
//!
//! For custom integrations, feed window events to [`Context::on_window_event`],
//! run [`Context::run`] on redraw, then present with [`Renderer::render`]. Every redraw evaluates the UI;
//! unchanged geometry stays cached. Widget activation and value changes automatically
//! schedule a follow-up redraw. Outside model changes only need a native redraw;
//! the host should sleep between events and honor [`Context::needs_repaint`].

#![forbid(unsafe_code)]

pub mod animation;
pub mod app;
pub mod components;
pub mod context;
pub mod images;
pub mod layout;
pub mod protocol;
pub mod renderer;
pub mod shapes;
mod text;
pub mod widgets;

pub use animation::{
    Animated, Animation, AnimationOptions, AnimationSample, AnimationStatus, Decay, DecayOptions,
    Delay, Easing, Interpolate, Keyframe, Keyframes, MotionStyle, Oklab, Parallel, Path,
    PathFollow, PathPose, Procedural, Pulse, Repeat, Rotation, Sequence, Spring, SpringOptions,
    SpringState, SpringValue, Stagger, Tween, TweenOptions, Wake,
};
pub use app::{
    run, run_with_options, App, AppStats, CloseRequested, CloseSource, ExitPolicy, Frame,
    GlobalShortcut, OpenOutcome, RunError, RunOptions, WindowError, WindowInfo, WindowKey,
    WindowOptions, WindowPlan, WindowStatus, Windows,
};
pub use components::theme::*;
pub use components::{
    move_item, DragEffect, DragEnd, DragOutput, DragReason, DragSource, DragStyle, DropOutput,
    DropTarget, DropZones, Dropped, Insertion, PreviewKind, RowDrag, RowMove, TreeNodeDrag,
};
pub use components::{
    Align, Badge, Blur, Button, ButtonVariant, Card, CardOutput, CardVariant, Checkbox,
    ColorPicker, ColorPickerType, Column, ColumnWidth, ComboBox, ComboBoxOption, ComboBoxStyle,
    ContextMenu, ContextMenuItem, ContextMenuOutput, ContextMenuStyle, ContextMenuWidget,
    DragValue, Field, Grid, GridOutput, GridRow, GridStyle, GridUi, Hover, HoverFill, HoverStyle,
    IconTabs, Image, ImageFit, ImageOutput, Loader, NumberInput, NumberStyle, Numeric, Popup, PopupOutput, Presence,
    Progress, ProgressState, Reorder, ReorderUi, Response, Root, RowPresence, ScrollArea,
    ScrollAreaOutput, ScrollStyle, SelectionIndicator, Sense, Separator, Skeleton, Slider,
    SliderStatus, SortDirection, SortRequest, SplitBoundaryOutput, SplitHandle, SplitHandleStyle,
    SplitOutput, SplitPane, SplitPanel, SplitPanelOutput, SplitSize, SplitStyle, SplitSurface,
    SplitUi, Style, Switch, Table, TableBody, TableOutput, TableStyle, Text, TextEdit, TitleBar,
    TitleBarResponse, Tooltip, TooltipStyle, TooltipWidget, Ui, Validation, Widget,
    WidgetState, Window,
};
pub use components::{
    CloseReason, Confirm, Confirmation, Dialog, DialogAction, DialogOutput, Modal, ModalAnchor,
    ModalOutput,
};
pub use components::{
    CollapsingHeader, CollapsingOutput, CollapsingStyle, DisclosureStyle, TreeChildren, TreeEvent,
    TreeIssue, TreeModel, TreeNode, TreeOutput, TreeStyle, TreeView,
};
pub use context::{
    CacheStats, Context, DebugOverlay, Diagnostic, DiagnosticKind, EventResponse, Id, InputState,
    SharedResources,
};
pub use glam::{vec2, Vec2};
pub use images::{
    DecodedImage, ImageDecoder, ImageError, ImageHandle, ImageLimits, ImageMetrics, ImageSource,
    ImageStage, ImageState, ImageTiming,
};
pub use layout::{Layout, Padding};
pub use protocol::{
    DrawCommand, DrawData, GeometryUpdate, TextureFilter, TextureId, TextureImage, TextureOptions,
    Vertex,
};
pub use renderer::{PresentationMode, RenderError, RenderStatus, Renderer, RendererStats};
pub use shapes::{
    Border, Color, CornerRadius, Gradient, GradientDirection, Rect, RectShape, Shadow, Shape,
    Transform,
};
pub use text::{FontFamily, FontWeight, MonospaceMetrics, TextFamily};
#[cfg(feature = "bundled-icons")]
pub use z_icons as icons;
// Integrations are always available; zaxis has no optional backend features.
pub use {wgpu, winit};
