use std::{hint::black_box, sync::Arc, time::Duration};
use zaxis::testing::Inspect;
use zaxis::winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{DeviceId, ElementState, Ime, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent},
    keyboard::KeyCode,
};
use zaxis::Instant;
use zaxis::{
    vec2, Blur, Border, Button, Checkbox, Color, ColorPicker, ColorPickerType, Context,
    CornerRadius, DrawCommand, DrawData, FontWeight, Padding, Rect, Response, Separator, Shape,
    Slider, SliderStatus, Text, TextureId, TextureImage, Vec2, Vertex, Window,
};

#[path = "scene/build.rs"]
mod build;
#[path = "scene/cases.rs"]
mod cases;
#[path = "scene/input.rs"]
mod input;
#[path = "scene/split.rs"]
mod split;
#[path = "scene/verify.rs"]
mod verify;
pub use cases::Case;

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
    modal: Option<crate::modal::Probe>,
    text_area: Option<crate::text_area::Probe>,
    number: Option<crate::number::Probe>,
    split: Option<split::Probe>,
    disclosure: Option<crate::disclosure::Probe>,
    list_box: Option<crate::list_box::Probe>,
    carousel: Option<crate::carousel::Probe>,
    dnd: Option<crate::dnd::Probe>,
    access: Option<crate::access::Probe>,
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
        let modal = if let Case::Modal(kind) = case {
            Some(crate::modal::Probe::new(&mut context, kind, count))
        } else {
            None
        };
        let text_area = if let Case::TextArea(kind) = case {
            Some(crate::text_area::Probe::new(&mut context, kind, count))
        } else {
            None
        };
        let number = if let Case::Number(kind) = case {
            Some(crate::number::Probe::new(&mut context, kind, count))
        } else {
            None
        };
        let access = if let Case::Access(kind) = case {
            Some(crate::access::Probe::new(&mut context, kind, count))
        } else {
            None
        };
        let mut scene = Self {
            context,
            access,
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
            modal,
            text_area,
            number,
            dnd: if let Case::Dnd(kind) = case {
                Some(crate::dnd::Probe::new(kind, count))
            } else {
                None
            },
            carousel: if let Case::Carousel(kind) = case {
                Some(crate::carousel::Probe::new(kind, count))
            } else {
                None
            },
            list_box: if let Case::ListBox(kind) = case {
                Some(crate::list_box::Probe::new(kind, count))
            } else {
                None
            },
            disclosure: if let Case::Disclosure(kind) = case {
                Some(crate::disclosure::Probe::new(kind, count))
            } else {
                None
            },
            split: if let Case::Split(kind) = case {
                Some(split::Probe::new(kind, count))
            } else {
                None
            },
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
