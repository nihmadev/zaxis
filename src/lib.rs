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
// wgpu's browser handles are not `Send`; its single-threaded wasm build shares them with `Arc`.
#![cfg_attr(target_arch = "wasm32", allow(clippy::arc_with_non_send_sync))]

pub mod accessibility;
pub mod actions;
pub mod animation;
pub mod app;
pub mod components;
pub mod context;
pub mod files;
pub mod images;
pub mod layout;
pub mod material;
pub mod protocol;
pub mod renderer;
pub mod shapes;
#[doc(hidden)]
pub mod text;
pub(crate) mod time;
pub mod widgets;

pub use accessibility::{
    AccessAction, AccessActionKind, AccessLive, AccessNode, AccessOrientation, AccessRole,
    AccessToggled, Accessible,
};
pub use actions::{
    Action, Actions, Binding, Chord, ChordError, Conflict, ConflictKind, Kbd, Keymap, KeymapIssue,
    Mods, Platform, Stroke,
};
pub use animation::{
    Animated, Animation, AnimationOptions, AnimationSample, AnimationStatus, Decay, DecayOptions,
    Delay, Easing, Interpolate, Keyframe, Keyframes, MotionStyle, Oklab, Parallel, Path,
    PathFollow, PathPose, Procedural, Pulse, Repeat, Rotation, Sequence, Spring, SpringOptions,
    SpringState, SpringValue, Stagger, Tween, TweenOptions, Wake,
};
pub use app::{
    run, run_with_options, App, AppStats, CloseRequested, CloseSource, ExitPolicy, Frame,
    GlobalShortcut, OpenOutcome, RunError, RunOptions, WebBackend, WebOptions, WindowError,
    WindowInfo, WindowKey, WindowOptions, WindowPlan, WindowStatus, Windows,
};
pub use components::dock::{
    Dock, DockActions, DockChild, DockDropZone, DockError, DockEvent, DockFloat, DockIssue,
    DockMotion, DockNode, DockOutput, DockPanelOutput, DockPreset, DockSide, DockState, DockStyle,
    DockViewer, PanelId,
};
pub use components::theme::*;
pub use components::{
    move_item, DragEffect, DragEnd, DragOutput, DragReason, DragSource, DragStyle, DropOutput,
    DropTarget, DropZones, Dropped, Insertion, PreviewKind, RowDrag, RowMove, TreeNodeDrag,
};
pub use components::{
    tab_after_close, Align, Badge, Blur, Button, ButtonVariant, Card, CardOutput, CardVariant,
    Carousel, CarouselIndicator, CarouselOrientation, CarouselOutput, CarouselPage, CarouselStyle,
    CarouselVariant, Checkbox, ColorPicker, ColorPickerType, Column, ColumnWidth, ComboBox,
    ComboBoxOption, ComboBoxStyle, ContextMenu, ContextMenuItem, ContextMenuOutput,
    ContextMenuStyle, ContextMenuWidget, DragValue, Field, Grid, GridOutput, GridRow, GridStyle,
    GridUi, Hover, HoverFill, HoverStyle, IconTabs, Image, ImageFit, ImageOutput,
    IndicatorPosition, KeyBinding, KeyBox, ListBox, ListBoxStyle, ListDensity, ListEntry,
    ListEntryKind, ListEvent, ListMode, ListModel, ListOutput, ListRow, Loader, MaterialPaint,
    MenuBar, MenuBarOutput, MenuItem, MouseBinding, NumberInput, NumberStyle, Numeric, Popup,
    PopupOutput, Presence, Progress, ProgressState, RadioGroup, RadioLayout, RadioOption,
    RadioSize, Reorder, ReorderUi, Response, Root, RowPresence, ScrollArea, ScrollAreaOutput,
    ScrollStyle, SegmentOption, SegmentWidth, SegmentedControl, SegmentedOrientation,
    SegmentedSize, SegmentedVariant, SelectionIndicator, Sense, Separator, Skeleton, Slider,
    SliderStatus, SortDirection, SortRequest, SplitBoundaryOutput, SplitHandle, SplitHandleStyle,
    SplitOutput, SplitPane, SplitPanel, SplitPanelOutput, SplitSize, SplitStyle, SplitSurface,
    SplitUi, StackDirection, Style, Switch, TabActivation, TabBar, TabBarOutput, TabDrag,
    TabDropZone, TabDropZoneOutput, TabItem, TabMove, TabRelease, TabVariant, TabWidth, Table,
    TableBody, TableOutput, TableStyle, TabsStyle, Text, TextEdit, TitleBar, TitleBarResponse,
    Toast, Tooltip, TooltipStyle, TooltipWidget, Ui, Validation, Widget, WidgetState, Window,
};
pub use components::{
    CloseReason, Confirm, Confirmation, Dialog, DialogAction, DialogOutput, Modal, ModalAnchor,
    ModalOutput,
};
pub use components::{
    CollapsingHeader, CollapsingOutput, CollapsingStyle, DisclosureStyle, TreeChildren, TreeEvent,
    TreeIssue, TreeModel, TreeNode, TreeOutput, TreeStyle, TreeView,
};
pub use components::{FocusGroup, FocusGroupOutput};
pub use components::{
    Hyperlink, HyperlinkOutput, HyperlinkStyle, LabelOutput, LinkActivation, LinkReport,
    LinkTarget, RichText, SelectableLabel, SelectionScope, Span, SpanStyle, Underline, UrlError,
    UrlPolicy,
};
pub use components::{PanZoom, PanZoomOutput, PanZoomState, ZoomWheel};
pub use context::popup::MAX_POPUP_DEPTH;
pub use context::testing;
pub use context::{
    ActionSource, ActionsHandle, CacheStats, ClipboardBackend, ClipboardError, Context,
    DebugOverlay, Diagnostic, DiagnosticKind, EventResponse, FocusAxis, Id, ImeEvent, InputEvent,
    InputState, InputStats, KeyEvent, KeyInput, KeyInterest, PendingChord, SharedResources,
    WheelDelta,
};
#[cfg(feature = "file-dialogs")]
pub use files::{
    DialogBackend, DialogLaunch, DialogReply, DialogRequest, DialogResult, FileDialog,
    FileDialogKind, MemoryDialogs, SystemDialogs,
};
pub use files::{FileError, FileFilter, FileTask, PickedFile};
pub use glam::{vec2, Vec2};
pub use images::{
    DecodedImage, ImageDecoder, ImageError, ImageHandle, ImageLimits, ImageMetrics, ImageSource,
    ImageStage, ImageState, ImageTiming,
};
pub use layout::{Layout, Padding};
pub use material::{
    Material, MaterialError, MaterialErrorKind, ParamKind, ParamLayout, ParamValue, Params,
    FRAME_BYTES, MAX_MATERIALS, MAX_PARAM_BYTES, MAX_UNIFORM_BYTES,
};
pub use protocol::{
    DrawCommand, DrawData, GeometryUpdate, MaterialDraw, MaterialId, MaterialSource, TextureFilter,
    TextureId, TextureImage, TextureOptions, Vertex,
};
pub use renderer::{
    EmbedAlpha, EmbedColorSpace, EmbedError, EmbedLoad, EmbedOptions, EmbedViewport,
    EmbeddedRenderer, PhysicalRect, PresentationMode, RecordReport, RenderError, RenderStatus,
    Renderer, RendererStats,
};
pub use shapes::{
    Border, Color, CornerRadius, Gradient, GradientDirection, Rect, RectShape, Shadow, Shape,
    Transform,
};
pub use text::{FontFamily, FontWeight, MonospaceMetrics, TextFamily};
pub use time::Instant;
#[cfg(feature = "bundled-icons")]
pub use z_icons as icons;
// The AccessKit version whose types `Context::take_accessibility_update` and
// `Context::on_accessibility_action` exchange with a platform adapter.
#[cfg(feature = "accesskit")]
pub use accesskit;
// The winit adapter `run` uses, for hosts that drive their own event loop.
#[cfg(all(feature = "accesskit", not(target_arch = "wasm32")))]
pub use accesskit_winit;
// The window and GPU integrations are always available.
pub use {wgpu, winit};
