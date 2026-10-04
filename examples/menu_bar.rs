use zaxis::{App, Context, Frame, Id, MenuBar, MenuItem, Root, TextEdit};

struct Demo {
    text: String,
    word_wrap: bool,
    status_bar: bool,
    last: String,
}

impl Demo {
    fn menus(&self) -> Vec<MenuItem> {
        vec![
            MenuItem::submenu(
                "File",
                [
                    MenuItem::new("new", "New").shortcut("Ctrl+N"),
                    MenuItem::submenu(
                        "Open recent",
                        [
                            MenuItem::new("recent-1", "notes.txt"),
                            MenuItem::new("recent-2", "todo.md"),
                        ],
                    ),
                    MenuItem::separator(),
                    MenuItem::new("clear", "Clear").enabled(!self.text.is_empty()),
                ],
            ),
            MenuItem::submenu(
                "View",
                [
                    MenuItem::new("wrap", "Word wrap").checked(self.word_wrap),
                    MenuItem::new("status", "Status bar").checked(self.status_bar),
                ],
            ),
            MenuItem::submenu("Help", [MenuItem::new("about", "About")]),
        ]
    }

    fn handle(&mut self, id: Id) {
        let name = |s: &str| Id::new(s) == id;
        if name("new") || name("clear") {
            self.text.clear();
        } else if name("wrap") {
            self.word_wrap = !self.word_wrap;
        } else if name("status") {
            self.status_bar = !self.status_bar;
        } else if name("recent-1") {
            self.text = "notes.txt".into();
        } else if name("recent-2") {
            self.text = "todo.md".into();
        }
        self.last = format!("{id:?}");
    }
}

impl App for Demo {
    fn update(&mut self, context: &mut Context, _frame: &mut Frame<'_>) {
        let items = self.menus();
        let mut selected = None;
        Root::new().show(context, |ui| {
            ui.horizontal(|ui| {
                selected = MenuBar::new("compact", &items)
                    .compact(true)
                    .show(ui)
                    .selected;
                selected = MenuBar::new("bar", &items).show(ui).selected.or(selected);
            });
            ui.add(
                TextEdit::new(&mut self.text)
                    .multiline()
                    .wrap(self.word_wrap),
            );
            if self.status_bar {
                ui.muted(format!("{} chars   last: {}", self.text.len(), self.last));
            }
        });
        if let Some(id) = selected {
            self.handle(id);
        }
    }
}

fn main() -> Result<(), zaxis::RunError> {
    zaxis::run_with_options(
        Demo {
            text: String::new(),
            word_wrap: true,
            status_bar: true,
            last: String::new(),
        },
        zaxis::RunOptions {
            window_attributes: zaxis::winit::window::Window::default_attributes()
                .with_title("zaxis — MenuBar")
                .with_inner_size(zaxis::winit::dpi::LogicalSize::new(640.0, 420.0)),
            ..Default::default()
        },
    )
}
