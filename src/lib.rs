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
    Animated, Animation, AnimationOptions, AnimationSample, AnimationStatus, Delay, Easing,
    Interpolate, Keyframe, Keyframes, MotionStyle, Parallel, Procedural, Pulse, Repeat, Rotation,
    Sequence, Spring, SpringOptions, SpringState, SpringValue, Stagger, Tween, TweenOptions, Wake,
};
pub use app::{run, run_with_options, App, Frame, RunError, RunOptions};
pub use components::{
    Align, Blur, Button, Checkbox, ColorPicker, ColorPickerType, Column, ColumnWidth, ComboBox,
    ComboBoxOption, ComboBoxStyle, DragValue, Grid, GridOutput, GridRow, GridStyle, GridUi, Hover,
    HoverFill, HoverStyle, Image, ImageFit, ImageOutput, Loader, NumberInput, NumberStyle, Numeric,
    Popup, PopupOutput, Presence, Progress, ProgressState, Reorder, ReorderUi, Response, Root,
    ScrollArea, ScrollAreaOutput, ScrollStyle, SelectionIndicator, Separator, Slider, SliderStatus,
    SortDirection, SortRequest, Style, Table, TableBody, TableOutput, TableStyle, Text, TextEdit,
    TitleBar, TitleBarResponse, Ui, Widget, WidgetState, Window,
};
pub use context::{CacheStats, Context, EventResponse, Id, InputState};
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
// Integrations are always available; zaxis has no optional backend features.
pub use {wgpu, winit};
