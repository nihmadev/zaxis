#[path = "split_pane/content.rs"]
mod content;
use zaxis::{
    vec2, App, Border, Button, Color, Context, CornerRadius, Frame, Id, Layout, Padding, Rect,
    Root, Shadow, SplitHandle, SplitHandleStyle, SplitOutput, SplitPane, SplitPanel, SplitSize,
    SplitStyle, SplitSurface, Ui,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Panel {
    Files,
    Work,
    Inspector,
    Preview,
    Editor,
    Tasks,
    Log,
}
impl Panel {
    fn title(self) -> &'static str {
        match self {
            Self::Files => "Files",
            Self::Work => "Workspace",
            Self::Inspector => "Inspector",
            Self::Preview => "Preview",
            Self::Editor => "Editor",
            Self::Tasks => "Tasks",
            Self::Log => "Log",
        }
    }
    fn spec(self) -> SplitPanel {
        let size = match self {
            Self::Files => SplitSize::Pixels(180.0),
            Self::Inspector | Self::Preview => SplitSize::Weight(1.0),
            Self::Work | Self::Editor => SplitSize::Weight(3.0),
            Self::Tasks => SplitSize::Weight(2.0),
            Self::Log => SplitSize::Pixels(110.0),
        };
        SplitPanel::new(self).default_size(size).min_size(60.0)
    }
}
struct Workspace {
    text: String,
    file: usize,
    enabled: bool,
    vertical: bool,
    glass: bool,
    zoom: f32,
    order: Vec<Panel>,
    inner_order: Vec<Panel>,
    drag: Option<(Panel, zaxis::Vec2)>,
    bounds: Vec<(Id, Rect)>,
    logs: Vec<String>,
    runs: [u32; 12],
    smoke: bool,
    passes: usize,
}
impl Workspace {
    fn header(&mut self, ui: &mut Ui<'_>, panel: Panel) {
        let response = ui.add(
            Button::new(panel.title())
                .min_size(vec2(ui.available_width(), 28.0))
                .padding(Padding::symmetric(8.0, 3.0))
                .selected(self.drag.is_some_and(|(p, _)| p == panel)),
        );
        if response.pressed && self.drag.is_none() {
            if let Some(pointer) = ui.context().input().pointer {
                self.drag = Some((panel, pointer));
            }
        }
    }
    fn drop_panel<R>(&mut self, ui: &mut Ui<'_>, output: &SplitOutput<R>, order: &mut [Panel]) {
        self.bounds
            .extend(output.panels.iter().map(|p| (p.id, p.bounds)));
        let input = ui.context().input();
        if !input.primary_released {
            return;
        }
        let Some((source, start)) = self.drag else {
            return;
        };
        let Some(pointer) = input.pointer else { return };
        if (pointer - start).length_squared() < 36.0 {
            return;
        }
        let Some(from) = order.iter().position(|p| *p == source) else {
            return;
        };
        let Some(to) = output
            .panels
            .iter()
            .position(|p| p.bounds.contains(pointer))
        else {
            return;
        };
        if from != to {
            let panel = order[from];
            if from < to {
                order[from..=to].rotate_left(1);
            } else {
                order[to..=from].rotate_right(1);
            }
            self.logs.push(format!("Moved {}", panel.title()));
            ui.context().request_repaint();
        }
    }
    fn toggle(order: &mut Vec<Panel>, panel: Panel, present: bool) {
        if present && !order.contains(&panel) {
            order.push(panel);
        }
        if !present {
            order.retain(|p| *p != panel);
        }
    }
    fn card(&self, panel: Panel, pointer: Option<zaxis::Vec2>) -> SplitSurface {
        let target = self.drag.is_some()
            && self.bounds.iter().any(|(id, bounds)| {
                *id == Id::new(panel) && pointer.is_some_and(|p| bounds.contains(p))
            });
        SplitSurface::default()
            .fill(Color::gray(45))
            .corner_radius(CornerRadius::all(12.0))
            .padding(Padding::all(8.0))
            .border(Border::new(
                if target { 2.0 } else { 1.0 },
                if target {
                    Color::rgb(130, 173, 226)
                } else {
                    Color::gray(68)
                },
            ))
            .shadow(Shadow {
                blur_radius: 4.0,
                ..Shadow::default()
            })
            .blur(if self.glass { 6.0 } else { 0.0 })
    }
    fn workspace(&mut self, ui: &mut Ui<'_>, reset: bool) {
        let mut order = self.inner_order.clone();
        let handle = SplitHandleStyle::default()
            .kind(SplitHandle::Invisible)
            .thickness(3.0)
            .hit_width(10.0)
            .length(32.0);
        let panels: Vec<_> = order
            .iter()
            .map(|p| {
                let spec = p.spec();
                if *p == Panel::Editor {
                    spec.handle(handle.kind(SplitHandle::PanelEdge))
                } else {
                    spec
                }
            })
            .collect();
        let out = SplitPane::vertical("editor-and-log")
            .gap(10.0)
            .handle(handle)
            .resizable(self.enabled)
            .reset(reset)
            .double_click_reset(true)
            .panels(panels)
            .show(ui, |split| {
                for &panel in &order {
                    split.panel(panel, |ui| {
                        self.header(ui, panel);
                        self.content(ui, panel);
                    });
                }
            });
        self.drop_panel(ui, &out, &mut order);
        self.inner_order = order;
    }
}
impl App for Workspace {
    fn update(&mut self, c: &mut Context, frame: &mut Frame<'_>) {
        Root::new().padding(Padding::all(12.0)).show(c, |ui| {
            let mut reset = false;
            ui.horizontal(|ui| {
                reset = ui.button("Reset").clicked();
                if ui.button("Swap").clicked() && self.order.len() > 1 {
                    let last = self.order.len() - 1;
                    self.order.swap(0, last);
                }
                ui.checkbox(&mut self.enabled, "Resize");
                ui.checkbox(&mut self.vertical, "Vertical");
                for panel in [Panel::Files, Panel::Inspector, Panel::Preview] {
                    let mut visible = self.order.contains(&panel);
                    if ui.checkbox(&mut visible, panel.title()).changed() {
                        Self::toggle(&mut self.order, panel, visible);
                    }
                }
            });
            if reset {
                self.order = vec![Panel::Files, Panel::Work, Panel::Inspector];
                self.inner_order = vec![Panel::Editor, Panel::Tasks, Panel::Log];
                self.drag = None;
            }
            let pointer = ui.context().input().pointer;
            let mut order = self.order.clone();
            let panels: Vec<_> = order
                .iter()
                .map(|&panel| panel.spec().surface(self.card(panel, pointer)))
                .collect();
            self.bounds.clear();
            let handle = SplitHandleStyle::default()
                .kind(SplitHandle::Grip)
                .thickness(4.0)
                .length(36.0)
                .hit_width(14.0);
            let style = SplitStyle::default().gap(14.0).handle(handle);
            let axis = if self.vertical {
                Layout::Vertical
            } else {
                Layout::Horizontal
            };
            let out = SplitPane::new("workspace", axis)
                .style(style)
                .resizable(self.enabled)
                .reset(reset)
                .double_click_reset(true)
                .panels(panels)
                .show(ui, |split| {
                    for &panel in &order {
                        split.panel(panel, |ui| {
                            self.header(ui, panel);
                            if panel == Panel::Work {
                                self.workspace(ui, reset);
                            } else {
                                self.content(ui, panel);
                            }
                        });
                    }
                });
            self.drop_panel(ui, &out, &mut order);
            self.order = order;
            let input = ui.context().input();
            if input.primary_released || !input.focused || !input.primary_down {
                self.drag = None;
            }
        });
        if self.smoke {
            self.passes += 1;
            if self.passes >= 8 {
                frame.close();
            } else {
                c.request_repaint_after(std::time::Duration::from_millis(50));
            }
        }
    }
}
fn main() -> Result<(), zaxis::RunError> {
    zaxis::run_with_options(
        Workspace {
            text: "main.rs".into(),
            file: 0,
            enabled: true,
            vertical: false,
            glass: false,
            zoom: 1.0,
            order: vec![Panel::Files, Panel::Work, Panel::Inspector],
            inner_order: vec![Panel::Editor, Panel::Tasks, Panel::Log],
            drag: None,
            bounds: Vec::new(),
            logs: vec!["Ready".into()],
            runs: [0; 12],
            smoke: std::env::args().any(|a| a == "--smoke-test"),
            passes: 0,
        },
        zaxis::RunOptions {
            presentation_mode: zaxis::PresentationMode::Immediate,
            window_attributes: zaxis::winit::window::Window::default_attributes()
                .with_title("zaxis — SplitPane")
                .with_inner_size(zaxis::winit::dpi::LogicalSize::new(1150.0, 760.0)),
            ..Default::default()
        },
    )
}
