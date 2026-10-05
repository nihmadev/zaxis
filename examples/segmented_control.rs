use zaxis::{
    icons, App, Card, Context, Frame, Root, ScrollArea, SegmentOption, SegmentWidth,
    SegmentedControl, SegmentedSize, SegmentedVariant, Theme, Ui,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Scheme {
    Dark,
    Light,
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum View {
    List,
    Board,
    Table,
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Filter {
    All,
    Open,
    Archived,
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Spacing {
    Tight,
    Normal,
    Loose,
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Sort {
    Name,
    Size,
    Date,
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Page {
    Overview,
    Activity,
    Settings,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Media {
    Songs,
    Albums,
    Podcasts,
}

/// name, size, day, open
const ROWS: [(&str, u32, u32, bool); 6] = [
    ("Quarterly plan", 48, 12, true),
    ("Design review", 12, 3, true),
    ("Release notes", 5, 21, false),
    ("Roadmap", 31, 8, true),
    ("Budget", 22, 17, false),
    ("Interviews", 9, 29, true),
];

struct Gallery {
    scheme: Scheme,
    view: View,
    filter: Filter,
    spacing: Spacing,
    sort: Sort,
    page: Page,
    media: Media,
    locked: View,
    smoke: bool,
    passes: usize,
}

impl Gallery {
    fn rows(&self) -> Vec<(&'static str, u32, u32, bool)> {
        let mut rows: Vec<_> = ROWS
            .into_iter()
            .filter(|row| match self.filter {
                Filter::All => true,
                Filter::Open => row.3,
                Filter::Archived => !row.3,
            })
            .collect();
        match self.sort {
            Sort::Name => rows.sort_by_key(|row| row.0),
            Sort::Size => rows.sort_by_key(|row| std::cmp::Reverse(row.1)),
            Sort::Date => rows.sort_by_key(|row| row.2),
        }
        rows
    }

    fn list(&self, ui: &mut Ui<'_>) {
        let rows = self.rows();
        let gap = match self.spacing {
            Spacing::Tight => 0.0,
            Spacing::Normal => 6.0,
            Spacing::Loose => 14.0,
        };
        match self.view {
            View::List => {
                for row in &rows {
                    ui.label(row.0);
                    ui.add_space(gap);
                }
            }
            View::Board => {
                ui.horizontal(|ui| {
                    for open in [true, false] {
                        ui.vertical(|ui| {
                            ui.muted(if open { "Open" } else { "Archived" });
                            for row in rows.iter().filter(|row| row.3 == open) {
                                ui.label(row.0);
                                ui.add_space(gap);
                            }
                        });
                        ui.add_space(24.0);
                    }
                });
            }
            View::Table => {
                for row in &rows {
                    ui.horizontal(|ui| {
                        ui.label(format!("{:<18}{:>4} KB   May {}", row.0, row.1, row.2));
                    });
                    ui.add_space(gap);
                }
            }
        }
    }

    fn content(&mut self, ui: &mut Ui<'_>) {
        ui.segmented(
            &mut self.scheme,
            [
                SegmentOption::new(Scheme::Dark, "Dark").icon(&icons::MOON),
                SegmentOption::new(Scheme::Light, "Light").icon(&icons::SUN),
            ],
        );
        ui.add_space(12.0);
        Card::new("view").show(ui, |ui| {
            ui.add(
                SegmentedControl::new(
                    &mut self.view,
                    [
                        SegmentOption::new(View::List, "List").icon(&icons::LIST),
                        SegmentOption::new(View::Board, "Board").icon(&icons::SQUARE_KANBAN),
                        SegmentOption::new(View::Table, "Table").icon(&icons::TABLE),
                    ],
                )
                .width(SegmentWidth::Fill),
            );
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.add(SegmentedControl::new(
                    &mut self.filter,
                    [
                        (Filter::All, "All"),
                        (Filter::Open, "Open"),
                        (Filter::Archived, "Archived"),
                    ],
                ));
                ui.add_space(8.0);
                ui.add(
                    SegmentedControl::new(
                        &mut self.spacing,
                        [
                            SegmentOption::new(Spacing::Tight, "Tight").tooltip("Tight rows"),
                            SegmentOption::new(Spacing::Normal, "Normal"),
                            SegmentOption::new(Spacing::Loose, "Loose").enabled(true),
                        ],
                    )
                    .size(SegmentedSize::Small),
                );
            });
            ui.add_space(12.0);
            self.list(ui);
        });
        ui.add_space(12.0);
        Card::new("media").show(ui, |ui| {
            ui.add(
                SegmentedControl::new(
                    &mut self.media,
                    [
                        (Media::Songs, "Songs"),
                        (Media::Albums, "Albums"),
                        (Media::Podcasts, "Podcasts"),
                    ],
                )
                .variant(SegmentedVariant::Outlined),
            );
            ui.add_space(8.0);
            let items: &[&str] = match self.media {
                Media::Songs => &["Morning light", "Night drive", "Paper boats"],
                Media::Albums => &["Low tide", "Signal", "Understory"],
                Media::Podcasts => &["Field notes", "Slow code", "After hours"],
            };
            for item in items {
                ui.label(*item);
            }
        });
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.add(
                    SegmentedControl::new(
                        &mut self.sort,
                        [
                            (Sort::Name, "Name"),
                            (Sort::Size, "Size"),
                            (Sort::Date, "Date"),
                        ],
                    )
                    .vertical()
                    .size(SegmentedSize::Large),
                );
            });
            ui.add_space(16.0);
            Card::new("narrow").width(210.0).show(ui, |ui| {
                ui.add(
                    SegmentedControl::new(
                        &mut self.page,
                        [
                            SegmentOption::new(Page::Overview, "Overview")
                                .icon(&icons::LAYOUT_DASHBOARD),
                            SegmentOption::new(Page::Activity, "Activity")
                                .icon(&icons::CHART_COLUMN),
                            SegmentOption::new(Page::Settings, "Settings").icon(&icons::SETTINGS),
                        ],
                    )
                    .width(SegmentWidth::Fill),
                );
                ui.add_space(8.0);
                ui.label(match self.page {
                    Page::Overview => "Overview",
                    Page::Activity => "Activity",
                    Page::Settings => "Settings",
                });
            });
            ui.add_space(16.0);
            ui.vertical(|ui| {
                ui.add(
                    SegmentedControl::new(
                        &mut self.locked,
                        [
                            SegmentOption::new(View::List, "List").icon(&icons::LIST),
                            SegmentOption::new(View::Board, "Board").icon(&icons::SQUARE_KANBAN),
                        ],
                    )
                    .enabled(false),
                );
                ui.add_space(8.0);
                let mut mixed = self.view;
                ui.add(SegmentedControl::new(
                    &mut mixed,
                    [
                        SegmentOption::new(View::List, "List"),
                        SegmentOption::new(View::Board, "Board").enabled(false),
                        SegmentOption::new(View::Table, "Table"),
                    ],
                ));
                self.view = mixed;
            });
        });
    }
}

impl App for Gallery {
    fn update(&mut self, c: &mut Context, frame: &mut Frame<'_>) {
        c.set_theme(match self.scheme {
            Scheme::Dark => Theme::dark(),
            Scheme::Light => Theme::light(),
        });
        Root::new().show(c, |ui| {
            ScrollArea::vertical().show(ui, |ui| self.content(ui));
        });
        if self.smoke {
            self.passes += 1;
            if self.passes >= 3 {
                frame.close();
            } else {
                c.request_repaint();
            }
        }
    }
}

fn main() -> Result<(), zaxis::RunError> {
    zaxis::run_with_options(
        Gallery {
            scheme: Scheme::Dark,
            view: View::List,
            filter: Filter::All,
            spacing: Spacing::Normal,
            sort: Sort::Name,
            page: Page::Overview,
            media: Media::Songs,
            locked: View::Board,
            smoke: std::env::args().any(|a| a == "--smoke-test"),
            passes: 0,
        },
        zaxis::RunOptions {
            window_attributes: zaxis::winit::window::Window::default_attributes()
                .with_title("zaxis — Segmented control")
                .with_inner_size(zaxis::winit::dpi::LogicalSize::new(720.0, 640.0)),
            ..Default::default()
        },
    )
}
