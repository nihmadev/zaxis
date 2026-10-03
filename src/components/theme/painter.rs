use super::{ControlState, SurfaceStyle};
use crate::{context::Paint, Rect, Shape, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaintMode {
    Replace,
    Before,
    After,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaintPart {
    Button,
    CheckboxBody,
    CheckboxIndicator,
    SliderTrack,
    SliderThumb,
}
/// Real component geometry and resolved state, never a mutable model reference.
#[derive(Clone, Copy, Debug)]
pub struct ControlPaint {
    pub bounds: Rect,
    pub part: PaintPart,
    pub style: SurfaceStyle,
    pub state: ControlState,
    pub value: f32,
}
/// Safe paint-only API. Input, allocation and Id are owned by the component.
/// Commands join the usual element cache; closure addresses are never keys.
pub struct Painter<'a> {
    pub(crate) paint: &'a mut Vec<Paint>,
    pub(crate) clip: Rect,
}
impl Painter<'_> {
    pub fn clip_rect(&self) -> Rect {
        self.clip
    }
    pub fn paint(&mut self, shape: impl Into<Shape>) {
        self.paint.push(Paint::Shape(shape.into()));
    }
    pub fn text(
        &mut self,
        text: impl Into<String>,
        position: Vec2,
        size: f32,
        color: crate::Color,
    ) {
        self.paint.push(Paint::Text {
            text: text.into(),
            position,
            size,
            wrap_width: f32::INFINITY,
            color,
        });
    }
}
pub(crate) type PaintCallback<'a> = dyn Fn(&mut Painter<'_>, ControlPaint) + 'a;
pub(crate) struct PaintCallbackHook<F> {
    pub mode: PaintMode,
    pub callback: F,
}
pub(crate) type PaintHook<'a> = PaintCallbackHook<Box<PaintCallback<'a>>>;
impl<F: Fn(&mut Painter<'_>, ControlPaint)> PaintCallbackHook<F> {
    pub(crate) fn run(&self, paint: &mut Vec<Paint>, clip: Rect, info: ControlPaint) {
        (self.callback)(&mut Painter { paint, clip }, info);
    }
}
