use std::time::Duration;
use zaxis::{
    vec2, App, Badge, Border, Button, Carousel, CarouselIndicator, Color, Context, Frame,
    ImageSource, Root, SegmentOption, Shape, Text, Theme, Ui,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum View {
    Cards,
    Photos,
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Axis {
    Horizontal,
    Vertical,
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Scheme {
    Dark,
    Light,
}

struct Shipment {
    place: &'static str,
    transit: &'static str,
    hub: &'static str,
    to: &'static str,
    from: &'static str,
    item: &'static str,
    following: bool,
}

const SIZE: (u32, u32) = (640, 360);
const TITLES: [(&str, &str); 6] = [
    ("Dawn Ridge", "#071"),
    ("Salt Flats", "#072"),
    ("Cedar Hollow", "#073"),
    ("Ember Coast", "#074"),
    ("Glass Lake", "#075"),
    ("Night Pass", "#076"),
];

/// A layered landscape: sky gradient, a sun and three ranges of hills.
fn landscape(seed: u32) -> ImageSource {
    let (w, h) = SIZE;
    let hue = |i: f32| {
        let t = (seed as f32 * 0.9 + i) * 1.1;
        [
            0.5 + 0.5 * t.sin(),
            0.5 + 0.5 * (t + 2.1).sin(),
            0.5 + 0.5 * (t + 4.2).sin(),
        ]
    };
    let (sky, sun, far, mid, near) = (hue(0.0), hue(1.3), hue(2.1), hue(3.4), hue(4.6));
    let mut pixels = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let (u, v) = (x as f32 / w as f32, y as f32 / h as f32);
            let mut c = [0.0_f32; 3];
            for k in 0..3 {
                c[k] = sky[k] * (1.0 - v * 0.6) + 0.25 * v;
            }
            let (dx, dy) = (u - 0.68, (v - 0.34) * 1.6);
            let glow = (1.0 - (dx * dx + dy * dy).sqrt() * 3.2).clamp(0.0, 1.0);
            for k in 0..3 {
                c[k] += sun[k] * glow * glow;
            }
            let ridge = |base: f32, amp: f32, freq: f32, phase: f32| {
                base + amp * ((u * freq + phase + seed as f32).sin() + 0.5 * (u * freq * 2.3).sin())
            };
            for (line, tint, shade) in [
                (ridge(0.58, 0.06, 6.0, 0.0), far, 0.55),
                (ridge(0.70, 0.07, 9.0, 1.7), mid, 0.4),
                (ridge(0.84, 0.05, 13.0, 3.1), near, 0.25),
            ] {
                if v > line {
                    for k in 0..3 {
                        c[k] = tint[k] * shade;
                    }
                }
            }
            pixels.extend(c.map(|v| (v.clamp(0.0, 1.0) * 255.0) as u8));
            pixels.push(255);
        }
    }
    ImageSource::rgba([w, h], pixels)
}

struct Gallery {
    view: View,
    scheme: Scheme,
    axis: Axis,
    card: usize,
    photo: usize,
    shipments: Vec<Shipment>,
    scenes: Vec<ImageSource>,
    smoke: bool,
    passes: usize,
}

impl Gallery {
    fn new(smoke: bool) -> Self {
        let shipment = |place, transit, hub, to, from, item| Shipment {
            place,
            transit,
            hub,
            to,
            from,
            item,
            following: false,
        };
        let mut scenes: Vec<_> = (1..TITLES.len() as u32).map(landscape).collect();
        scenes.insert(0, ImageSource::path("assets/images/landscape.jpg"));
        Self {
            view: View::Cards,
            scheme: Scheme::Light,
            axis: Axis::Horizontal,
            card: 0,
            photo: 0,
            shipments: vec![
                shipment(
                    "Sending by courier",
                    "Kalimantan Sorting",
                    "Bandung Hub",
                    "Bandung",
                    "Kalimantan",
                    "Batagor Bandung",
                ),
                shipment(
                    "Out for delivery",
                    "Surabaya Depot",
                    "Jakarta Hub",
                    "Jakarta",
                    "Surabaya",
                    "Kopi Tubruk",
                ),
                shipment(
                    "Ready for pickup",
                    "Medan Sorting",
                    "Makassar Hub",
                    "Makassar",
                    "Medan",
                    "Kain Songket",
                ),
                shipment(
                    "Handed to courier",
                    "Denpasar Depot",
                    "Yogyakarta Hub",
                    "Yogyakarta",
                    "Denpasar",
                    "Batik Tulis",
                ),
            ],
            scenes,
            smoke,
            passes: 0,
        }
    }
}

fn timeline(ui: &mut Ui<'_>, s: &Shipment) {
    let accent = ui.style().accent;
    let pale = ui.style().muted_text.with_opacity(0.45);
    ui.vertical(|ui| {
        for (i, (caption, value)) in [
            ("Current location", s.place),
            ("In transit", s.transit),
            ("Processed", s.hub),
        ]
        .into_iter()
        .enumerate()
        {
            ui.with_height(52.0, |ui| {
                ui.horizontal(|ui| {
                    let marker = ui.allocate_space(vec2(30.0, 30.0));
                    if i < 2 {
                        let from = marker.center() + vec2(0.0, 12.0);
                        ui.paint(Shape::Line {
                            start: from,
                            end: from + vec2(0.0, 40.0),
                            width: 2.0,
                            color: pale,
                        });
                    }
                    let (radius, fill) = if i == 0 { (14.0, accent) } else { (7.0, pale) };
                    ui.paint(Shape::Circle {
                        center: marker.center(),
                        radius,
                        fill,
                        border: Border::NONE,
                    });
                    if i == 0 {
                        ui.paint(Shape::Circle {
                            center: marker.center(),
                            radius: 4.5,
                            fill: Color::WHITE,
                            border: Border::NONE,
                        });
                    }
                    ui.vertical(|ui| {
                        ui.add(Text::new(caption).muted().size(12.0));
                        ui.add(Text::new(value).weight(zaxis::FontWeight::SEMIBOLD));
                    });
                });
            });
        }
    });
}

fn field(ui: &mut Ui<'_>, caption: &str, value: &str) {
    ui.vertical(|ui| {
        ui.add(Text::new(caption).muted().size(12.0));
        ui.add(
            Text::new(value)
                .size(18.0)
                .weight(zaxis::FontWeight::SEMIBOLD),
        );
    });
}

fn shipment(ui: &mut Ui<'_>, s: &mut Shipment) {
    let green = ui.style().success;
    ui.horizontal(|ui| {
        timeline(ui, s);
        ui.add_space(20.0);
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                field(ui, "Destination", s.to);
                ui.add_space(28.0);
                field(ui, "From", s.from);
            });
            ui.add_space(6.0);
            field(ui, "Item", s.item);
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                let label = if s.following {
                    "Following"
                } else {
                    "On its way!"
                };
                ui.add(
                    Badge::new(label)
                        .fill(green.with_opacity(0.18))
                        .text_color(green)
                        .height(26.0)
                        .font_size(13.0),
                );
                ui.add_space(8.0);
                let action = if s.following { "Unfollow" } else { "Follow" };
                if ui.add(Button::new(action)).clicked() {
                    s.following = !s.following;
                }
            });
        });
    });
}

impl App for Gallery {
    fn update(&mut self, c: &mut Context, frame: &mut Frame<'_>) {
        c.set_theme(match self.scheme {
            Scheme::Dark => Theme::dark(),
            Scheme::Light => Theme::light(),
        });
        Root::new().show(c, |ui| {
            ui.horizontal(|ui| {
                ui.segmented(
                    &mut self.view,
                    [
                        SegmentOption::new(View::Cards, "Cards"),
                        SegmentOption::new(View::Photos, "Photos"),
                    ],
                );
                ui.add_space(12.0);
                ui.segmented(
                    &mut self.axis,
                    [
                        SegmentOption::new(Axis::Horizontal, "Horizontal"),
                        SegmentOption::new(Axis::Vertical, "Vertical"),
                    ],
                );
                ui.add_space(12.0);
                ui.segmented(
                    &mut self.scheme,
                    [
                        SegmentOption::new(Scheme::Dark, "Dark"),
                        SegmentOption::new(Scheme::Light, "Light"),
                    ],
                );
            });
            ui.add_space(16.0);
            match self.view {
                View::Cards => {
                    let shipments = &mut self.shipments;
                    let vertical = self.axis == Axis::Vertical;
                    let cards = Carousel::new("shipments");
                    let cards = if vertical { cards.vertical() } else { cards };
                    cards
                        .pages(shipments.len())
                        .size(vec2(640.0, 330.0))
                        .layers(3)
                        .show(ui, &mut self.card, |ui, i| shipment(ui, &mut shipments[i]));
                }
                View::Photos => {
                    let scenes = &self.scenes;
                    let vertical = self.axis == Axis::Vertical;
                    let photos = Carousel::new("photos");
                    let photos = if vertical { photos.vertical() } else { photos };
                    photos
                        .images()
                        .pages(scenes.len())
                        .size(if vertical {
                            vec2(560.0, 440.0)
                        } else {
                            vec2(820.0, 400.0)
                        })
                        .looping(true)
                        .arrows(true)
                        .indicator(CarouselIndicator::Dashes)
                        .autoplay(Duration::from_secs(6))
                        .show_images_with(
                            ui,
                            &mut self.photo,
                            |i| scenes[i].clone(),
                            |ui, i| {
                                let (title, tag) = TITLES[i];
                                ui.add_space(110.0);
                                ui.vertical_aligned(zaxis::Align::Center, |ui| {
                                    ui.add(
                                        Text::new(title)
                                            .size(34.0)
                                            .weight(zaxis::FontWeight::BOLD)
                                            .color(Color::WHITE),
                                    );
                                    ui.add(
                                        Text::new(tag)
                                            .size(24.0)
                                            .color(Color::rgba(255, 255, 255, 215)),
                                    );
                                });
                            },
                        );
                }
            }
        });
        if self.smoke {
            self.passes += 1;
            match self.passes {
                3 => self.card = 2,
                6 => self.view = View::Photos,
                _ if self.passes > 8 && c.image_state(&self.scenes[1]).is_ready() => {
                    assert!(c.diagnostics().is_empty(), "{:?}", c.diagnostics());
                    println!("carousel smoke passed");
                    frame.close();
                }
                _ if self.passes > 600 => panic!("photos never became ready"),
                _ => {}
            }
            c.request_repaint();
        }
    }
}

fn main() -> Result<(), zaxis::RunError> {
    zaxis::run_with_options(
        Gallery::new(std::env::args().any(|a| a == "--smoke-test")),
        zaxis::RunOptions {
            window_attributes: zaxis::winit::window::Window::default_attributes()
                .with_title("zaxis — Carousel")
                .with_inner_size(zaxis::winit::dpi::LogicalSize::new(900.0, 560.0)),
            ..Default::default()
        },
    )
}
