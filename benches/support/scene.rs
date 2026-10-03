use std::{
    hint::black_box,
    sync::Arc,
    time::{Duration, Instant},
};
use zaxis::winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{DeviceId, ElementState, Ime, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent},
    keyboard::KeyCode,
};
use zaxis::{
    vec2, Blur, Border, Button, Checkbox, Color, ColorPicker, ColorPickerType, Context,
    CornerRadius, DrawCommand, DrawData, Padding, Rect, Response, Separator, Shape, Slider,
    SliderStatus, Text, TextureId, TextureImage, Vec2, Vertex, Window,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Case {
    Images(crate::images::ImageCase),
    Cold,
    Cached,
    Repaint,
    Schedule,
    Dynamic,
    Button,
    Followup,
    Checkbox,
    Slider,
    Hover,
    Press,
    Drag,
    Resize,
    Tabs,
    Disabled,
    Cancel,
    FocusLoss,
    Wheel,
    ScrollSettings,
    ScrollRows,
    ScrollNested,
    ScrollMiddle,
    ScrollCached,
    ComboClosed,
    ComboOpen,
    ComboToggle,
    ComboKeys,
    ComboFilter,
    ComboUpdates,
    ComboScroll,
    Ime,
    Theme,
    Dpi,
    Lifecycle,
    Text,
    WrappedText,
    Shapes,
    Blur0,
    Blur8,
    Blur24,
    Blur64,
    BlurDynamic,
    BlurStack,
    ControlBlur,
    ProtocolCached,
    ProtocolGeometry,
    ProtocolTexture,
    KeyboardButton,
    KeyboardCheckbox,
    KeyboardSlider,
    KeyboardTab,
    ColorPickerClosed,
    ColorPickerInternal,
    ColorPickerFloating,
    ColorPickerPalette,
    ColorPickerHue,
    ColorPickerInput,
    ColorPickerDrag,
    ColorPickerFloatingDrag,
    ColorPickerParentDrag,
    BlurWindowDrag,
    ColorPickerInternalBlurDrag,
    ColorPickerFloatingBlurDrag,
}

impl Case {
    pub const ALL: [Self; 83] = [
        Self::Images(crate::images::ImageCase::Empty),
        Self::Images(crate::images::ImageCase::Warm),
        Self::Images(crate::images::ImageCase::ColdPng),
        Self::Images(crate::images::ImageCase::ColdJpeg),
        Self::Images(crate::images::ImageCase::ColdWebp),
        Self::Images(crate::images::ImageCase::ColdFile),
        Self::Images(crate::images::ImageCase::Shared),
        Self::Images(crate::images::ImageCase::Unique),
        Self::Images(crate::images::ImageCase::Pixels),
        Self::Images(crate::images::ImageCase::Dimensions),
        Self::Images(crate::images::ImageCase::SvgSmall),
        Self::Images(crate::images::ImageCase::SvgLarge),
        Self::Images(crate::images::ImageCase::SvgDpi),
        Self::Images(crate::images::ImageCase::SvgResize),
        Self::Images(crate::images::ImageCase::Downsample),
        Self::Images(crate::images::ImageCase::Scroll),
        Self::Images(crate::images::ImageCase::Evict),
        Self::Images(crate::images::ImageCase::GpuRecovery),
        Self::Images(crate::images::ImageCase::FirstShow),
        Self::Images(crate::images::ImageCase::RepeatedShow),
        Self::Cold,
        Self::Cached,
        Self::Repaint,
        Self::Schedule,
        Self::Dynamic,
        Self::Button,
        Self::Followup,
        Self::Checkbox,
        Self::Slider,
        Self::Hover,
        Self::Press,
        Self::Drag,
        Self::Resize,
        Self::Tabs,
        Self::Disabled,
        Self::Cancel,
        Self::FocusLoss,
        Self::Wheel,
        Self::ScrollSettings,
        Self::ScrollRows,
        Self::ScrollNested,
        Self::ScrollMiddle,
        Self::ScrollCached,
        Self::ComboClosed,
        Self::ComboOpen,
        Self::ComboToggle,
        Self::ComboKeys,
        Self::ComboFilter,
        Self::ComboUpdates,
        Self::ComboScroll,
        Self::Ime,
        Self::Theme,
        Self::Dpi,
        Self::Lifecycle,
        Self::Text,
        Self::WrappedText,
        Self::Shapes,
        Self::Blur0,
        Self::Blur8,
        Self::Blur24,
        Self::Blur64,
        Self::BlurDynamic,
        Self::BlurStack,
        Self::ControlBlur,
        Self::ProtocolCached,
        Self::ProtocolGeometry,
        Self::ProtocolTexture,
        Self::KeyboardButton,
        Self::KeyboardCheckbox,
        Self::KeyboardSlider,
        Self::KeyboardTab,
        Self::ColorPickerClosed,
        Self::ColorPickerInternal,
        Self::ColorPickerFloating,
        Self::ColorPickerPalette,
        Self::ColorPickerHue,
        Self::ColorPickerInput,
        Self::ColorPickerDrag,
        Self::ColorPickerFloatingDrag,
        Self::ColorPickerParentDrag,
        Self::BlurWindowDrag,
        Self::ColorPickerInternalBlurDrag,
        Self::ColorPickerFloatingBlurDrag,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Images(case) => case.name(),
            Self::Cold => "cold_ui",
            Self::Cached => "cached_ui",
            Self::Repaint => "explicit_repaint",
            Self::Schedule => "repaint_deadlines",
            Self::Dynamic => "dynamic_widgets",
            Self::Button => "button_click",
            Self::Followup => "model_followup",
            Self::Checkbox => "checkbox_click",
            Self::Slider => "slider_drag",
            Self::Hover => "hover",
            Self::Press => "press_release",
            Self::Drag => "window_drag",
            Self::Resize => "window_resize",
            Self::Tabs => "tab_switch",
            Self::Disabled => "disabled_click",
            Self::Cancel => "release_outside",
            Self::FocusLoss => "focus_loss",
            Self::Wheel => "wheel_input",
            Self::ScrollSettings => "scroll_settings",
            Self::ScrollRows => "scroll_virtual_rows",
            Self::ScrollNested => "scroll_nested",
            Self::ScrollMiddle => "scroll_middle",
            Self::ScrollCached => "scroll_cached",
            Self::ComboClosed => "combo_closed_cached",
            Self::ComboOpen => "combo_open_cached",
            Self::ComboToggle => "combo_toggle_animation",
            Self::ComboKeys => "combo_keyboard",
            Self::ComboFilter => "combo_filter",
            Self::ComboUpdates => "combo_options_update",
            Self::ComboScroll => "combo_scroll",
            Self::Ime => "ime_input",
            Self::Theme => "theme_change",
            Self::Dpi => "dpi_change",
            Self::Lifecycle => "object_lifecycle",
            Self::Text => "text_dynamic",
            Self::WrappedText => "text_wrapped",
            Self::Shapes => "vector_shapes",
            Self::Blur0 => "blur_0",
            Self::Blur8 => "blur_8",
            Self::Blur24 => "blur_24",
            Self::Blur64 => "blur_64",
            Self::BlurDynamic => "blur_radius_change",
            Self::BlurStack => "blur_stack",
            Self::ControlBlur => "control_blur",
            Self::ProtocolCached => "protocol_cached",
            Self::ProtocolGeometry => "protocol_geometry",
            Self::ProtocolTexture => "protocol_texture",
            Self::KeyboardButton => "keyboard_button",
            Self::KeyboardCheckbox => "keyboard_checkbox",
            Self::KeyboardSlider => "keyboard_slider",
            Self::KeyboardTab => "keyboard_tab",
            Self::ColorPickerClosed => "color_picker_closed",
            Self::ColorPickerInternal => "color_picker_internal_cached",
            Self::ColorPickerFloating => "color_picker_floating_cached",
            Self::ColorPickerPalette => "color_picker_palette_drag",
            Self::ColorPickerHue => "color_picker_hue_drag",
            Self::ColorPickerInput => "color_picker_rgb_hex_input",
            Self::ColorPickerDrag => "color_picker_internal_window_drag",
            Self::ColorPickerFloatingDrag => "color_picker_floating_window_drag",
            Self::ColorPickerParentDrag => "color_picker_parent_window_drag",
            Self::BlurWindowDrag => "blur_window_drag",
            Self::ColorPickerInternalBlurDrag => "color_picker_internal_blur_window_drag",
            Self::ColorPickerFloatingBlurDrag => "color_picker_floating_blur_window_drag",
        }
    }
    pub fn scrolling(self) -> bool {
        matches!(
            self,
            Self::ScrollSettings
                | Self::ScrollRows
                | Self::ScrollNested
                | Self::ScrollMiddle
                | Self::ScrollCached
        )
    }
    pub fn interactive(self) -> bool {
        self.combo()
            || matches!(
                self,
                Self::Button
                    | Self::Followup
                    | Self::Checkbox
                    | Self::Slider
                    | Self::Hover
                    | Self::Press
                    | Self::Drag
                    | Self::Resize
                    | Self::Tabs
                    | Self::Disabled
                    | Self::Cancel
                    | Self::FocusLoss
                    | Self::KeyboardButton
                    | Self::KeyboardCheckbox
                    | Self::KeyboardSlider
                    | Self::KeyboardTab
                    | Self::ColorPickerInternal
                    | Self::ColorPickerFloating
                    | Self::ColorPickerPalette
                    | Self::ColorPickerHue
                    | Self::ColorPickerInput
                    | Self::ColorPickerDrag
                    | Self::ColorPickerFloatingDrag
                    | Self::ColorPickerParentDrag
                    | Self::BlurWindowDrag
                    | Self::ColorPickerInternalBlurDrag
                    | Self::ColorPickerFloatingBlurDrag
            )
    }
    pub fn combo(self) -> bool {
        matches!(
            self,
            Self::ComboClosed
                | Self::ComboOpen
                | Self::ComboToggle
                | Self::ComboKeys
                | Self::ComboFilter
                | Self::ComboUpdates
                | Self::ComboScroll
        )
    }
    fn color_picker_probe(self) -> bool {
        matches!(
            self,
            Self::ColorPickerInternal
                | Self::ColorPickerFloating
                | Self::ColorPickerPalette
                | Self::ColorPickerHue
                | Self::ColorPickerInput
                | Self::ColorPickerDrag
                | Self::ColorPickerFloatingDrag
                | Self::ColorPickerParentDrag
                | Self::ColorPickerInternalBlurDrag
                | Self::ColorPickerFloatingBlurDrag
        )
    }
    fn floating_picker(self) -> bool {
        matches!(
            self,
            Self::ColorPickerFloating
                | Self::ColorPickerFloatingDrag
                | Self::ColorPickerParentDrag
                | Self::ColorPickerFloatingBlurDrag
        )
    }
    pub fn protocol(self) -> bool {
        matches!(
            self,
            Self::ProtocolCached | Self::ProtocolGeometry | Self::ProtocolTexture
        )
    }
    pub fn radius(self, step: usize) -> f32 {
        match self {
            Self::Blur8 => 8.0,
            Self::Blur24 => 24.0,
            Self::Blur64 => 64.0,
            Self::BlurDynamic => {
                if step.is_multiple_of(2) {
                    8.0
                } else {
                    24.0
                }
            }
            _ => 0.0,
        }
    }
}

#[derive(Default)]
struct Targets {
    button: Option<Response>,
    checkbox: Option<Response>,
    slider: Option<Response>,
    disabled: Option<Response>,
    tabs: Vec<Response>,
    window: Rect,
    color_picker: Option<Response>,
    palette: Rect,
    hue: Rect,
    fields: [Rect; 4],
    floating: Rect,
}

pub struct Scene {
    pub context: Context,
    pub case: Case,
    pub count: usize,
    pub step: usize,
    size: PhysicalSize<u32>,
    scale: f64,
    targets: Targets,
    previous_button: Option<Response>,
    previous_window: Rect,
    previous_palette: Rect,
    previous_floating: Rect,
    color: Color,
    previous_color: Color,
    colors: Vec<Color>,
    checked: bool,
    previous_checked: bool,
    previous_selected: usize,
    model_label: String,
    clicks: u64,
    previous_clicks: u64,
    value: f32,
    selected: usize,
    labels: Vec<String>,
    checks: Vec<bool>,
    values: Vec<f32>,
    protocol: DrawData,
    previous_revision: u64,
    previous_tessellations: u64,
    input_verified: bool,
    scroll: crate::scroll::Probe,
    combo: Option<crate::combo_box::Probe>,
    pub images: Option<crate::images::Probe>,
}

impl Scene {
    pub fn new(case: Case, count: usize, size: PhysicalSize<u32>, scale: f64) -> Self {
        let mut context = Context::new();
        configure(&mut context, size, scale);
        if matches!(
            case,
            Case::BlurWindowDrag
                | Case::ColorPickerInternalBlurDrag
                | Case::ColorPickerFloatingBlurDrag
        ) {
            context.set_style(context.style().clone().blur(12.0));
        }
        let images = if let Case::Images(case) = case {
            Some(crate::images::Probe::new(&mut context, case, count))
        } else {
            None
        };
        let mut scene = Self {
            context,
            case,
            count,
            size,
            scale,
            step: 0,
            targets: Targets::default(),
            previous_button: None,
            previous_window: Rect::default(),
            previous_palette: Rect::default(),
            previous_floating: Rect::default(),
            color: Color::rgba(255, 0, 0, 73),
            previous_color: Color::rgba(255, 0, 0, 73),
            colors: vec![Color::rgb(78, 133, 190); count],
            checked: false,
            previous_checked: false,
            previous_selected: 0,
            model_label: String::new(),
            clicks: 0,
            previous_clicks: 0,
            value: 0.0,
            selected: 0,
            labels: (0..count).map(|i| format!("Item {i}")).collect(),
            checks: vec![false; count],
            values: vec![0.25; count],
            protocol: DrawData::new(
                vec2(
                    size.width as f32 / scale as f32,
                    size.height as f32 / scale as f32,
                ),
                scale as f32,
            ),
            previous_revision: 0,
            previous_tessellations: 0,
            input_verified: true,
            scroll: crate::scroll::Probe::default(),
            combo: case.combo().then(|| crate::combo_box::Probe::new(count)),
            images,
        };
        if case.protocol() {
            scene.make_protocol();
        }
        scene.build();
        if let Some(images) = &mut scene.images {
            images.await_ready(&mut scene.context);
        }
        scene
    }

    /// Inject native-format events through the same public dispatcher as a host.
    pub fn input(&mut self, step: usize) {
        self.step = step;
        self.previous_button = self.targets.button;
        self.previous_window = self.targets.window;
        self.previous_palette = self.targets.palette;
        self.previous_floating = self.targets.floating;
        self.previous_color = self.color;
        self.previous_checked = self.checked;
        self.previous_selected = self.selected;
        self.previous_clicks = self.clicks;
        self.previous_revision = self.draw_data().revision;
        self.previous_tessellations = self.context.cache_stats().tessellated_elements;
        self.input_verified = true;
        if let Some(images) = &mut self.images {
            images.input(&mut self.context, step);
            return;
        }
        if let Some(combo) = &mut self.combo {
            combo.input(&mut self.context, self.case, step);
            return;
        }
        if self.case.scrolling() {
            self.scroll.input(&mut self.context, self.case, step);
            return;
        }
        let forward = step.is_multiple_of(2);
        match self.case {
            Case::Cold => {
                self.context = Context::new();
                configure(&mut self.context, self.size, self.scale);
            }
            Case::Repaint => self.context.request_repaint(),
            Case::Schedule => {
                self.context.request_repaint_after(Duration::from_secs(10));
                let first = self.context.next_repaint().unwrap();
                self.context.request_repaint_after(Duration::from_secs(20));
                self.input_verified &= self.context.next_repaint() == Some(first);
                self.context.request_repaint_after(Duration::from_secs(5));
                self.input_verified &= self.context.next_repaint().unwrap() <= first;
            }
            Case::Button | Case::Followup | Case::Checkbox | Case::Disabled | Case::Tabs => {
                let response = match self.case {
                    Case::Button | Case::Followup => self.targets.button.unwrap(),
                    Case::Checkbox => self.targets.checkbox.unwrap(),
                    Case::Disabled => self.targets.disabled.unwrap(),
                    _ => self.targets.tabs[1 - self.selected],
                };
                self.move_to(response.rect.center());
                self.mouse(ElementState::Pressed);
                self.mouse(ElementState::Released);
            }
            Case::Hover => {
                let target = if forward {
                    self.targets.button.unwrap().rect.center()
                } else {
                    vec2(1.0, 1.0)
                };
                self.move_to(target);
            }
            Case::Press => {
                self.move_to(self.targets.button.unwrap().rect.center());
                self.mouse(if forward {
                    ElementState::Pressed
                } else {
                    ElementState::Released
                });
            }
            Case::Slider => {
                let rect = self.targets.slider.unwrap().rect;
                self.move_to(rect.center());
                self.mouse(ElementState::Pressed);
                self.move_to(vec2(
                    if forward {
                        rect.max.x + 30.0
                    } else {
                        rect.min.x - 30.0
                    },
                    rect.center().y,
                ));
                self.mouse(ElementState::Released);
            }
            Case::Drag
            | Case::Resize
            | Case::ColorPickerDrag
            | Case::ColorPickerFloatingDrag
            | Case::ColorPickerParentDrag
            | Case::BlurWindowDrag
            | Case::ColorPickerInternalBlurDrag
            | Case::ColorPickerFloatingBlurDrag => {
                let rect = if matches!(
                    self.case,
                    Case::ColorPickerFloatingDrag | Case::ColorPickerFloatingBlurDrag
                ) {
                    self.targets.floating
                } else {
                    self.targets.window
                };
                let point = if self.case != Case::Resize {
                    rect.min + vec2(30.0, 12.0)
                } else {
                    rect.max - vec2(5.0, 5.0)
                };
                self.move_to(point);
                self.mouse(ElementState::Pressed);
                self.move_to(
                    point
                        + if forward {
                            vec2(3.0, 2.0)
                        } else {
                            vec2(-3.0, -2.0)
                        },
                );
                self.mouse(ElementState::Released);
            }
            Case::ColorPickerPalette | Case::ColorPickerHue => {
                let rect = if self.case == Case::ColorPickerPalette {
                    self.targets.palette
                } else {
                    self.targets.hue
                };
                let t = if forward {
                    vec2(0.8, 0.2)
                } else {
                    vec2(0.2, 0.8)
                };
                self.move_to(rect.center());
                self.input_verified &= self.mouse(ElementState::Pressed);
                self.move_to(rect.min + rect.size() * t);
                self.input_verified &= self.mouse(ElementState::Released);
            }
            Case::ColorPickerInput => {
                let field = if forward { 3 } else { 0 };
                self.move_to(self.targets.fields[field].center());
                self.input_verified &= self.mouse(ElementState::Pressed);
                self.input_verified &= self.mouse(ElementState::Released);
                self.input_verified &= self
                    .context
                    .on_text_event(if forward { "#336699" } else { "222" })
                    .consumed;
                self.input_verified &= self
                    .context
                    .on_key_event(KeyCode::Enter, ElementState::Pressed, false)
                    .consumed;
                self.context
                    .on_key_event(KeyCode::Enter, ElementState::Released, false);
            }
            Case::Cancel | Case::FocusLoss => {
                self.move_to(self.targets.button.unwrap().rect.center());
                self.mouse(ElementState::Pressed);
                if self.case == Case::FocusLoss {
                    self.context.on_window_event(&WindowEvent::Focused(false));
                } else {
                    self.move_to(vec2(1.0, 1.0));
                }
                self.mouse(ElementState::Released);
            }
            Case::Wheel => {
                self.context.on_window_event(&WindowEvent::MouseWheel {
                    device_id: DeviceId::dummy(),
                    delta: MouseScrollDelta::LineDelta(0.0, 1.0),
                    phase: TouchPhase::Moved,
                });
                self.input_verified = self.context.input().scroll_delta.y > 0.0;
            }
            Case::Ime => {
                self.context
                    .on_window_event(&WindowEvent::Ime(Ime::Commit("Benchmark λ".into())));
                self.input_verified = self.context.input().text == "Benchmark λ";
            }
            Case::KeyboardButton
            | Case::KeyboardCheckbox
            | Case::KeyboardSlider
            | Case::KeyboardTab => {
                // A cancelled pointer press focuses the desired widget without activating it.
                let target = match self.case {
                    Case::KeyboardCheckbox => self.targets.checkbox.unwrap(),
                    Case::KeyboardSlider => self.targets.slider.unwrap(),
                    _ => self.targets.button.unwrap(),
                };
                self.move_to(target.rect.center());
                self.mouse(ElementState::Pressed);
                self.move_to(vec2(1.0, 1.0));
                self.mouse(ElementState::Released);
                let code = match self.case {
                    Case::KeyboardButton => {
                        if forward {
                            KeyCode::Enter
                        } else {
                            KeyCode::Space
                        }
                    }
                    Case::KeyboardCheckbox => KeyCode::Space,
                    Case::KeyboardSlider => {
                        if forward {
                            KeyCode::End
                        } else {
                            KeyCode::Home
                        }
                    }
                    _ => KeyCode::Tab,
                };
                self.input_verified = self
                    .context
                    .on_key_event(code, ElementState::Pressed, false)
                    .consumed;
                self.context
                    .on_key_event(code, ElementState::Released, false);
            }
            Case::Theme => {
                let mut style = self.context.style().clone();
                style.button_fill = if forward {
                    Color::rgb(64, 76, 92)
                } else {
                    Color::gray(66)
                };
                self.context.set_style(style);
            }
            Case::Dpi => self.context.set_viewport(
                self.size,
                if forward {
                    self.scale * 2.0
                } else {
                    self.scale
                },
            ),
            _ => {}
        }
    }

    pub fn build(&mut self) {
        self.build_once();
        if self.case == Case::Followup && self.context.needs_repaint() {
            self.build_once();
        }
    }

    fn build_once(&mut self) {
        if let Some(images) = &mut self.images {
            images.build(&mut self.context);
            return;
        }
        if let Some(combo) = &mut self.combo {
            combo.build(&mut self.context, self.case, self.step);
            return;
        }
        if self.case.scrolling() {
            self.scroll.build(
                &mut self.context,
                self.case,
                self.step,
                &self.labels,
                &mut self.checks,
                &mut self.values,
            );
            return;
        }
        if self.case.protocol() {
            match self.case {
                Case::ProtocolGeometry => {
                    self.protocol.vertices[0].position[0] = (self.step % 2) as f32;
                    self.protocol.revision += 1;
                }
                Case::ProtocolTexture => {
                    Arc::make_mut(&mut self.protocol.textures[0].pixels)[0] =
                        (self.step % 256) as u8;
                    self.protocol.textures[0].revision += 1;
                }
                _ => {}
            }
            black_box(&self.protocol);
            return;
        }
        let Self {
            context,
            case,
            count,
            step,
            targets,
            checked,
            clicks,
            value,
            selected,
            labels,
            checks,
            values,
            colors,
            color,
            model_label,
            ..
        } = self;
        *targets = Targets::default();
        context.run(|context| {
            let bounds = context.viewport();
            // Detailed backdrop makes blur cost and paint ordering representative.
            for i in 0..32 {
                let p = vec2((i % 8) as f32 * bounds.size().x / 8.0, (i / 8) as f32 * bounds.size().y / 4.0);
                context.paint_background(Shape::rect(Rect::from_min_size(p, bounds.size() / vec2(8.0, 4.0)),
                    Color::rgb(30 + (i * 7) as u8, 55, 90)).corner_radius(12.0));
            }
            let columns = ((*count as f32 * bounds.size().x / bounds.size().y).sqrt().ceil() as usize).clamp(1, 64);
            let rows = count.div_ceil(columns);
            let width = bounds.size().x / columns as f32;
            let visible_count = if *case == Case::Lifecycle && *step % 2 == 0 { *count / 2 } else { *count };
            for column in 0..columns {
                Window::new(format!("Load##{column}"))
                    .default_position(vec2(column as f32 * width, 0.0))
                    .default_size(vec2(width, bounds.size().y))
                    .min_size(vec2(64.0, 64.0)).padding(Padding::all(3.0))
                    .draggable(false).resizable(false).blur(case.radius(*step))
                    .show(context, |ui| {
                        for row in 0..rows {
                            let i = column * rows + row;
                            if i >= visible_count { break; }
                            ui.push_id(i, |ui| {
                                match case {
                                    Case::ColorPickerClosed => {
                                        ui.add(ColorPicker::new(&mut colors[i], &labels[i]).width(width - 6.0));
                                    }
                                    Case::Text | Case::WrappedText => {
                                        let caption = format!("Row {i}: frame {step:06} · text wrapping and glyph cache λ");
                                        ui.add(Text::new(caption).size(12.0).wrap(*case == Case::WrappedText));
                                    }
                                    Case::Shapes => {
                                        let rect = ui.allocate_space(vec2(width - 6.0, 24.0));
                                        let color = Color::rgb(70, 100 + (*step % 2) as u8 * 80, 160);
                                        match i % 3 {
                                            0 => ui.paint(Shape::rect(rect, color).corner_radius(CornerRadius {
                                                top_left: 2.0, top_right: 7.0, bottom_right: 12.0, bottom_left: 4.0,
                                            }).border(Border::new(1.0, Color::WHITE))),
                                            1 => ui.paint(Shape::Circle { center: rect.center(), radius: 10.0,
                                                fill: color, border: Border::new(2.0, Color::WHITE) }),
                                            _ => ui.paint(Shape::Line { start: rect.min, end: rect.max, width: 2.0, color }),
                                        }
                                    }
                                    _ => {
                                        if *case == Case::Dynamic {
                                            checks[i] = *step % 2 == 0;
                                            values[i] = (*step % 100) as f32 / 100.0;
                                        }
                                        let blur = if *case == Case::ControlBlur { 8.0 } else { 0.0 };
                                        match i % 4 {
                                            0 => {
                                                let caption = if *case == Case::Dynamic { format!("{i}:{step}") } else { labels[i].clone() };
                                                ui.add(Button::new(caption).blur(blur).selected(*case == Case::Dynamic && *step % 2 == 0));
                                            }
                                            1 => { ui.add(Checkbox::new(&mut checks[i], &labels[i]).size(14.0).blur(blur)); }
                                            2 => { ui.add(Slider::new(&mut values[i], 0.0..=1.0).step(0.01).status(SliderStatus::Success).blur(blur)); }
                                            _ => {
                                                let caption = if *case == Case::Dynamic { format!("{i}:{step}") } else { labels[i].clone() };
                                                ui.add(Text::new(caption).muted().wrap(false));
                                            }
                                        }
                                    }
                                }
                            });
                        }
                        ui.add(Separator::new().thickness(1.0).spacing(0.0).inset(2.0));
                    });
            }
            if *case == Case::BlurStack {
                Window::new("Layered glass").default_position(vec2(40.0, 40.0))
                    .default_size(bounds.size() - vec2(80.0, 80.0)).blur(8.0).show(context, |ui| {
                        let rect = ui.clip_rect();
                        for i in 0..8 {
                            Blur::new(rect.shrink(i as f32 * 4.0)).radius(8.0 + i as f32 * 4.0)
                                .corner_radius(12.0).tint(Color::rgba(80, 130, 180, 12)).show(ui);
                        }
                        ui.label("Eight overlapping filters with foreground text");
                    });
            }
            if case.interactive() {
                Window::new("Probe").default_position(vec2(32.0, 32.0))
                    .default_size(vec2(420.0, 360.0)).padding(Padding::all(8.0))
                    .show(context, |ui| {
                        // Content bounds include the resize reserve of 12 logical px.
                        let clip = ui.clip_rect();
                        targets.window = Rect::from_min_max(
                            clip.min - vec2(8.0, ui.style().title_height + 8.0),
                            clip.max + vec2(12.0, 12.0),
                        );
                        if case.color_picker_probe() {
                            let response = ui.add(ColorPicker::new(color, "Probe color")
                                .width(300.0).default_open(true)
                                .picker_type(if case.floating_picker() { ColorPickerType::Floating } else { ColorPickerType::Internal }));
                            targets.color_picker = Some(response);
                            targets.button = Some(ui.button("React"));
                            if !case.floating_picker() {
                                targets.palette = Rect::from_min_size(response.rect.min + vec2(0.0, 34.0), vec2(300.0, 112.0));
                            }
                            return;
                        }
                        *model_label = format!("Model: {clicks} / {checked} / {value:.2}");
                        ui.label(model_label.clone());
                        let button = ui.button("React");
                        if button.clicked() { *clicks += 1; }
                        targets.button = Some(button);
                        targets.checkbox = Some(ui.checkbox(checked, "Toggle"));
                        targets.slider = Some(ui.add(Slider::new(value, 0.0..=1.0).step(0.01)));
                        targets.tabs = ui.tab_bar(selected, [(0, "First"), (1, "Second")]);
                        targets.disabled = Some(ui.add_enabled_ui(false, |ui| {
                            ui.add_enabled_ui(true, |ui| ui.button("Disabled group"))
                        }));
                        ui.horizontal(|ui| {
                            ui.selectable(false, "Selection");
                            ui.add(Separator::vertical(20.0));
                            ui.muted("Nested row");
                        });
                    });
            }
        });
        if self.case.color_picker_probe() {
            if self.case.floating_picker() {
                // Identify the popup's content clip from public draw data, so the probe
                // observes its actual retained position after a title-bar drag.
                let clip = self
                    .context
                    .draw_data()
                    .commands
                    .iter()
                    .map(|command| command.clip_rect)
                    .find(|clip| (clip.size() - vec2(300.0, 176.0)).length() < 0.01)
                    .expect("open color picker popup was not rendered");
                self.targets.palette = Rect::from_min_size(clip.min, vec2(300.0, 112.0));
                self.targets.floating = Rect::from_min_max(
                    clip.min - vec2(12.0, self.context.style().title_height + 12.0),
                    clip.max + Vec2::splat(12.0),
                );
            }
            let origin = self.targets.palette.min;
            self.targets.hue = Rect::from_min_size(origin + vec2(0.0, 122.0), vec2(300.0, 14.0));
            self.targets.fields = std::array::from_fn(|field| {
                Rect::from_min_size(origin + vec2(field as f32 * 75.0, 150.0), vec2(70.0, 26.0))
            });
        }
        black_box(self.context.draw_data());
    }

    /// Assertions run outside measured intervals; every interaction must actually work.
    pub fn verify(&self) {
        if let Some(images) = &self.images {
            images.verify(&self.context);
            return;
        }
        if let Some(combo) = &self.combo {
            combo.verify(&self.context, self.case, self.step);
            return;
        }
        if self.case.scrolling() {
            self.scroll.verify(&self.context, self.case);
        }
        assert!(
            self.input_verified,
            "input was not delivered or repaint scheduling failed"
        );
        let forward = self.step.is_multiple_of(2);
        match self.case {
            Case::Button | Case::Followup | Case::KeyboardButton => {
                assert_eq!(
                    self.clicks,
                    self.previous_clicks + 1,
                    "button did not react"
                );
                if self.case == Case::Followup {
                    assert_eq!(
                        self.model_label,
                        format!(
                            "Model: {} / {} / {:.2}",
                            self.clicks, self.checked, self.value
                        )
                    );
                    assert!(
                        !self.context.needs_repaint(),
                        "follow-up model frame did not settle"
                    );
                }
            }
            Case::Checkbox | Case::KeyboardCheckbox => {
                assert_ne!(
                    self.checked, self.previous_checked,
                    "checkbox did not toggle"
                );
                assert!(self.targets.checkbox.unwrap().changed());
            }
            Case::Slider | Case::KeyboardSlider => {
                assert_eq!(
                    self.value,
                    if forward { 1.0 } else { 0.0 },
                    "slider lost capture outside its bounds"
                );
                assert!(self.targets.slider.unwrap().changed());
            }
            Case::Hover => assert_eq!(self.targets.button.unwrap().hovered, forward),
            Case::Press => {
                assert_eq!(self.targets.button.unwrap().pressed, forward);
                assert_eq!(self.targets.button.unwrap().clicked(), !forward);
            }
            Case::Drag
            | Case::ColorPickerDrag
            | Case::ColorPickerParentDrag
            | Case::BlurWindowDrag
            | Case::ColorPickerInternalBlurDrag => {
                let delta = if forward {
                    vec2(3.0, 2.0)
                } else {
                    vec2(-3.0, -2.0)
                };
                assert!(
                    (self.targets.button.unwrap().rect.min
                        - self.previous_button.unwrap().rect.min
                        - delta)
                        .length()
                        < 0.01,
                    "title-bar drag did not move children"
                );
                if matches!(
                    self.case,
                    Case::ColorPickerDrag | Case::ColorPickerInternalBlurDrag
                ) {
                    assert!(
                        (self.targets.palette.min - self.previous_palette.min - delta).length()
                            < 0.01,
                        "internal color picker did not move with its parent"
                    );
                } else if self.case == Case::ColorPickerParentDrag {
                    assert_eq!(
                        self.targets.floating, self.previous_floating,
                        "independent popup moved with its parent"
                    );
                }
                assert_eq!(
                    self.color, self.previous_color,
                    "window drag changed the color"
                );
            }
            Case::ColorPickerFloatingDrag | Case::ColorPickerFloatingBlurDrag => {
                let delta = if forward {
                    vec2(3.0, 2.0)
                } else {
                    vec2(-3.0, -2.0)
                };
                assert!(
                    (self.targets.floating.min - self.previous_floating.min - delta).length()
                        < 0.01,
                    "floating color picker did not move"
                );
                assert!(
                    (self.targets.palette.min - self.previous_palette.min - delta).length() < 0.01
                );
                assert_eq!(self.targets.window, self.previous_window);
                assert_eq!(self.color, self.previous_color);
            }
            Case::ColorPickerPalette | Case::ColorPickerHue | Case::ColorPickerInput => {
                let expected = match self.case {
                    Case::ColorPickerPalette if forward => Color::rgba(204, 41, 41, 73),
                    Case::ColorPickerPalette => Color::rgba(51, 41, 41, 73),
                    Case::ColorPickerHue if forward => Color::rgba(204, 0, 255, 73),
                    Case::ColorPickerHue => Color::rgba(204, 255, 0, 73),
                    Case::ColorPickerInput if forward => Color::rgba(51, 102, 153, 73),
                    _ => Color::rgba(222, 102, 153, 73),
                };
                assert_eq!(
                    self.color, expected,
                    "color picker input did not update its bound color"
                );
                assert!(self.targets.color_picker.unwrap().changed());
            }
            Case::Resize => {
                let delta = if forward {
                    vec2(3.0, 2.0)
                } else {
                    vec2(-3.0, -2.0)
                };
                assert!(
                    (self.targets.window.size() - self.previous_window.size() - delta).length()
                        < 0.01,
                    "resize grip did not resize window"
                );
            }
            Case::Tabs => {
                assert_ne!(self.selected, self.previous_selected);
                assert!(self.targets.tabs[self.selected].clicked());
            }
            Case::KeyboardTab => assert!(self.targets.checkbox.unwrap().has_focus),
            Case::Disabled => {
                let r = self.targets.disabled.unwrap();
                assert!(!r.clicked() && !r.pressed && !r.has_focus);
                assert_eq!(self.clicks, self.previous_clicks);
            }
            Case::Cancel | Case::FocusLoss => {
                assert_eq!(self.clicks, self.previous_clicks);
                assert!(
                    !self.targets.button.unwrap().pressed && !self.context.input().primary_down
                );
                if self.case == Case::FocusLoss {
                    assert!(!self.context.input().focused);
                }
            }
            Case::Cached
            | Case::Repaint
            | Case::Blur0
            | Case::Blur8
            | Case::Blur24
            | Case::Blur64
            | Case::ControlBlur
            | Case::BlurStack
            | Case::ColorPickerClosed
            | Case::ColorPickerInternal
            | Case::ColorPickerFloating => {
                assert_eq!(
                    self.draw_data().revision,
                    self.previous_revision,
                    "static geometry changed"
                );
                assert_eq!(
                    self.context.cache_stats().tessellated_elements,
                    self.previous_tessellations
                );
            }
            Case::BlurDynamic => {
                assert_eq!(
                    self.context.cache_stats().tessellated_elements,
                    self.previous_tessellations,
                    "changing blur sigma retessellated geometry"
                );
                if self.step > 0 {
                    assert_ne!(self.draw_data().revision, self.previous_revision);
                }
            }
            Case::Dynamic
            | Case::Theme
            | Case::Dpi
            | Case::Lifecycle
            | Case::Text
            | Case::WrappedText
            | Case::Shapes => {
                if self.step > 0 {
                    assert!(
                        self.draw_data().revision > self.previous_revision,
                        "changing scene reused stale geometry"
                    );
                }
            }
            Case::Schedule => {
                // A long GPU present can cross the deadline after Context::run.
                // An expired deadline may also have been consumed at pass start.
                let deadline = self.context.next_repaint();
                let before = Instant::now();
                let repaint = self.context.needs_repaint();
                let after = Instant::now();
                match deadline {
                    Some(time) if time <= before => assert!(repaint),
                    Some(time) if time > after => assert!(!repaint),
                    None => assert!(!repaint),
                    _ => {} // Deadline crossed during the observation itself.
                }
            }
            Case::Wheel => assert_eq!(self.context.input().scroll_delta, Vec2::ZERO),
            Case::Ime => assert!(self.context.input().text.is_empty()),
            _ => {}
        }
        if !self.case.protocol() {
            assert!(!self.draw_data().vertices.is_empty());
            assert_eq!(self.draw_data().scale_factor, self.context.scale_factor());
        }
        if self.case.interactive() && !matches!(self.case, Case::Press | Case::Hover) {
            assert!(!self.context.input().primary_down);
        }
    }

    pub fn draw_data(&self) -> &DrawData {
        if self.case.protocol() {
            &self.protocol
        } else {
            self.context.draw_data()
        }
    }

    fn move_to(&mut self, p: Vec2) {
        let scale = self.context.scale_factor();
        self.context.on_window_event(&WindowEvent::CursorMoved {
            device_id: DeviceId::dummy(),
            position: PhysicalPosition::new(f64::from(p.x * scale), f64::from(p.y * scale)),
        });
    }
    fn mouse(&mut self, state: ElementState) -> bool {
        self.context
            .on_window_event(&WindowEvent::MouseInput {
                device_id: DeviceId::dummy(),
                state,
                button: MouseButton::Left,
            })
            .consumed
    }
    fn make_protocol(&mut self) {
        let data = &mut self.protocol;
        let columns = (self.count as f32).sqrt().ceil() as usize;
        let size = data.logical_size / vec2(columns as f32, self.count.div_ceil(columns) as f32);
        for i in 0..self.count {
            let p = vec2((i % columns) as f32, (i / columns) as f32) * size;
            let base = data.vertices.len() as u32;
            for (position, uv) in [
                (p, [0.0, 0.0]),
                (p + vec2(size.x, 0.0), [1.0, 0.0]),
                (p + size, [1.0, 1.0]),
                (p + vec2(0.0, size.y), [0.0, 1.0]),
            ] {
                data.vertices.push(Vertex {
                    position: position.to_array(),
                    uv,
                    color: [1.0; 4],
                });
            }
            data.indices
                .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
            data.commands.push(DrawCommand {
                indices: (i * 6) as u32..(i * 6 + 6) as u32,
                clip_rect: Rect::from_min_size(p, size),
                texture: TextureId(9000),
                blur: None,
                scroll_hint: false,
            });
        }
        data.textures.push(TextureImage {
            id: TextureId(9000),
            size: [256, 256],
            pixels: Arc::new(vec![180; 256 * 256 * 4]),
            revision: 1,
        });
        data.revision = 1;
    }
}

fn configure(context: &mut Context, size: PhysicalSize<u32>, scale: f64) {
    context.set_viewport(size, scale);
    let mut style = context.style().clone();
    style.font_size = 12.0;
    style.spacing = 2.0;
    style.button_padding = Padding::symmetric(4.0, 3.0);
    context.set_style(style);
    context.request_repaint_after(Duration::ZERO);
}
