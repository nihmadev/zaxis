use zaxis::winit::{
    dpi::PhysicalPosition,
    event::{DeviceId, ElementState, MouseButton, WindowEvent},
};
use zaxis::{
    vec2, Border, Button, Color, Column, Context, Image, Layout, Padding, Root, ScrollArea, Shadow,
    SplitHandle, SplitHandleStyle, SplitOutput, SplitPane, SplitPanel, SplitSize, SplitStyle,
    SplitSurface, Table, TextEdit, Ui, Vec2,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitCase {
    Cached,
    CachedHeavy,
    Horizontal,
    Vertical,
    Nested,
    Heavy,
    Blur,
    Many,
    Viewport,
    Reorder,
}
impl SplitCase {
    pub fn name(self) -> &'static str {
        match self {
            Self::Cached => "split_cached",
            Self::CachedHeavy => "split_cached_heavy",
            Self::Horizontal => "split_drag_horizontal",
            Self::Vertical => "split_drag_vertical",
            Self::Nested => "split_drag_nested",
            Self::Heavy => "split_drag_heavy",
            Self::Blur => "split_drag_blur",
            Self::Many => "split_many_panels",
            Self::Viewport => "split_viewport",
            Self::Reorder => "split_reorder",
        }
    }
    pub fn interactive(self) -> bool {
        !matches!(
            self,
            Self::Cached | Self::CachedHeavy | Self::Viewport | Self::Reorder
        )
    }
    fn heavy(self) -> bool {
        matches!(self, Self::Heavy | Self::CachedHeavy | Self::Blur)
    }
}
pub struct Probe {
    kind: SplitCase,
    count: usize,
    step: usize,
    output: Option<SplitOutput<()>>,
    previous: Vec<f32>,
    revision: u64,
    tessellations: u64,
    origin: Option<(Vec2, f32, f32)>,
    expected: Option<f32>,
    order: Vec<usize>,
    text: String,
    labels: Vec<String>,
    built_rows: usize,
}
impl Probe {
    pub fn new(kind: SplitCase, count: usize) -> Self {
        Self {
            kind,
            count,
            step: 0,
            output: None,
            previous: Vec::new(),
            revision: 0,
            tessellations: 0,
            origin: None,
            expected: None,
            order: (0..if kind == SplitCase::Many {
                count.max(3)
            } else {
                3
            })
                .collect(),
            text: "Immediate-mode editor".into(),
            labels: (0..count)
                .map(|i| format!("Item {i:05} · renderer and layout"))
                .collect(),
            built_rows: 0,
        }
    }
    fn axis(&self) -> Layout {
        if matches!(self.kind, SplitCase::Vertical | SplitCase::Nested) {
            Layout::Vertical
        } else {
            Layout::Horizontal
        }
    }
    pub fn input(&mut self, c: &mut Context, step: usize) {
        self.step = step;
        self.revision = c.draw_data().revision;
        self.tessellations = c.cache_stats().tessellated_elements;
        let out = self.output.as_ref().unwrap();
        self.previous = out.panels.iter().map(|p| p.size).collect();
        if self.kind == SplitCase::Reorder {
            self.order.rotate_left(1);
            return;
        }
        if !self.kind.interactive() {
            return;
        }
        if self.origin.is_none() {
            let pointer = out.boundaries[0].bounds.center();
            move_to(c, pointer);
            assert!(
                c.on_window_event(&WindowEvent::MouseInput {
                    device_id: DeviceId::dummy(),
                    state: ElementState::Pressed,
                    button: MouseButton::Left
                })
                .consumed
            );
            self.origin = Some((
                pointer,
                out.panels[0].size,
                out.panels[0].size + out.panels[1].size,
            ));
        }
        let (pointer, start, sum) = self.origin.unwrap();
        // Continuous native-format captured drag, including fractional positions
        // and an excursion outside the handle/container in the cross axis.
        let delta = (step as f32 * 0.17).sin()
            * if self.kind == SplitCase::Many {
                sum * 0.3
            } else {
                48.0
            };
        let offset = match self.axis() {
            Layout::Horizontal => vec2(delta, 900.0),
            Layout::Vertical => vec2(900.0, delta),
        };
        move_to(c, pointer + offset);
        let min = if self.kind == SplitCase::Many {
            0.0
        } else {
            24.0
        };
        self.expected = Some((start + delta).clamp(min, sum - min));
    }
    pub fn build(&mut self, c: &mut Context) {
        let kind = self.kind;
        let axis = self.axis();
        let heavy = kind.heavy();
        let panel_style = SplitSurface::default()
            .fill(Color::gray(45))
            .corner_radius(12.0)
            .padding(Padding::all(8.0))
            .border(Border::new(1.0, Color::gray(70)))
            .shadow(Shadow {
                blur_radius: 4.0,
                ..Shadow::default()
            })
            .blur(if kind == SplitCase::Blur { 12.0 } else { 0.0 });
        let style = SplitStyle::default().panel(panel_style).handle(
            SplitHandleStyle::default()
                .kind(SplitHandle::Grip)
                .thickness(3.0),
        );
        let panels: Vec<_> = self
            .order
            .iter()
            .map(|&i| {
                let p = SplitPanel::new(i).default_size(SplitSize::Weight(1.0));
                if kind == SplitCase::Many {
                    p
                } else {
                    p.min_size(24.0)
                }
            })
            .collect();
        self.built_rows = 0;
        c.run(|c| {
            Root::new().padding(Padding::all(8.0)).show(c, |ui| {
                let size = vec2(
                    (ui.available_width() - 40.0).min(1100.0),
                    (ui.available_height() - 40.0).min(680.0),
                );
                let size = if kind == SplitCase::Viewport {
                    size - Vec2::splat((self.step % 2) as f32 * 40.375)
                } else {
                    size
                };
                let build = |ui: &mut Ui<'_>, this: &mut Self| {
                    let out = SplitPane::new("split-bench", axis)
                        .size(size)
                        .style(style)
                        .gap(if kind == SplitCase::Many { 0.0 } else { 10.0 })
                        .panels(panels)
                        .show(ui, |split| {
                            for &i in &this.order.clone() {
                                split.panel(i, |ui| {
                                    if heavy {
                                        this.heavy_content(ui, i);
                                    } else if kind != SplitCase::Many {
                                        ScrollArea::vertical().show(ui, |ui| {
                                            for label in &this.labels {
                                                ui.label(label);
                                            }
                                        });
                                    }
                                });
                            }
                        });
                    this.output = Some(out);
                };
                if kind == SplitCase::Nested {
                    ui.split_horizontal(
                        "outer",
                        [SplitPanel::new("nested"), SplitPanel::new("side")],
                        |split| {
                            split.panel("nested", |ui| build(ui, self));
                            split.panel("side", |ui| {
                                ui.label("Untouched sibling");
                            });
                        },
                    );
                } else {
                    build(ui, self);
                }
            })
        });
    }
    fn heavy_content(&mut self, ui: &mut Ui<'_>, panel: usize) {
        match panel {
            0 => {
                ui.add(TextEdit::new(&mut self.text).width(ui.available_width().max(1.0)));
                ScrollArea::vertical().show(ui, |ui| {
                    for label in &self.labels {
                        ui.add(Button::new(label).padding(Padding::symmetric(5.0, 3.0)));
                    }
                });
            }
            1 => {
                Table::new("records")
                    .max_height(ui.available_height())
                    .columns([
                        Column::fixed("id", 48.0).title("ID"),
                        Column::remainder("value").title("Value"),
                    ])
                    .show_rows(ui, 30.0, self.count * 10, |body, i| {
                        self.built_rows += 1;
                        body.row(i, |row| {
                            row.cell(|ui| {
                                ui.label(i.to_string());
                            });
                            row.cell(|ui| {
                                ui.button("Run");
                            });
                        });
                    });
            }
            _ => {
                ui.add(
                    Image::new(include_bytes!("../../../assets/images/landscape.jpg").as_slice())
                        .size(vec2(ui.available_width(), 180.0)),
                );
                ScrollArea::vertical().show(ui, |ui| {
                    for label in &self.labels {
                        ui.label(label);
                    }
                });
            }
        }
    }
    pub fn verify(&self, c: &Context) {
        let out = self.output.as_ref().unwrap();
        assert_eq!(out.panels.len(), self.order.len());
        assert!(out
            .panels
            .iter()
            .all(|p| p.size.is_finite() && p.size >= 0.0));
        for pair in out.panels.windows(2) {
            match self.axis() {
                Layout::Horizontal => assert!(pair[0].bounds.max.x <= pair[1].bounds.min.x),
                Layout::Vertical => assert!(pair[0].bounds.max.y <= pair[1].bounds.min.y),
            }
        }
        if matches!(self.kind, SplitCase::Cached | SplitCase::CachedHeavy)
            && !self.previous.is_empty()
        {
            assert_eq!(
                c.draw_data().revision,
                self.revision,
                "cached split rebuilt frame geometry"
            );
            assert_eq!(
                c.cache_stats().tessellated_elements,
                self.tessellations,
                "cached split retessellated"
            );
        }
        if let Some(expected) = self.expected {
            assert!(
                (out.panels[0].size - expected).abs() < 0.05,
                "resize did not follow pointer"
            );
            let sum = self.origin.unwrap().2;
            assert!((out.panels[0].size + out.panels[1].size - sum).abs() < 0.05);
            assert!(out
                .panels
                .iter()
                .skip(2)
                .zip(self.previous.iter().skip(2))
                .all(|(p, old)| (p.size - old).abs() < 0.05));
            assert!(c.input().primary_down, "continuous capture lost");
        }
        if self.kind.heavy() {
            assert!(
                self.built_rows > 0 && self.built_rows < 28,
                "table virtualization changed"
            );
        }
        assert!(!c.draw_data().vertices.is_empty());
    }
}
fn move_to(c: &mut Context, point: Vec2) {
    let scale = c.scale_factor() as f64;
    c.on_window_event(&WindowEvent::CursorMoved {
        device_id: DeviceId::dummy(),
        position: PhysicalPosition::new(point.x as f64 * scale, point.y as f64 * scale),
    });
}
