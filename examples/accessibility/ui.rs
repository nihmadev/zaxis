use crate::model::{Model, Page, Plan};
use zaxis::{
    vec2, Button, Column, ComboBox, ComboBoxOption, Context, Dialog, DialogAction, Field, Id,
    Image, ListBox, ListEntry, ListMode, MenuBar, MenuItem, RadioGroup, Root, ScrollArea, Slider,
    Table, TextEdit, Toast, TreeView, Ui, Validation, Widget,
};

pub fn show(context: &mut Context, model: &mut Model) {
    let saved = model.saved;
    Root::new().show(context, |ui| {
        menu(ui, model);
        ui.tab_bar(
            &mut model.page,
            [(Page::Profile, "Profile"), (Page::Data, "Data")],
        );
        ScrollArea::vertical()
            .id_source(model.page)
            .show(ui, |ui| match model.page {
                Page::Profile => profile(ui, model),
                Page::Data => data(ui, model),
            });
        about(ui, model);
    });
    if model.saved != saved {
        // A toast is a live region: a screen reader speaks it without moving focus.
        context.toast(Toast::new("Profile saved").content(model.name.clone()));
    }
}

fn menu(ui: &mut Ui<'_>, model: &mut Model) {
    let items = [
        MenuItem::submenu(
            "File",
            [
                MenuItem::new("save", "Save").shortcut("Ctrl+S"),
                MenuItem::separator(),
                MenuItem::new("autosave", "Autosave").checked(model.autosave),
            ],
        ),
        MenuItem::submenu("Help", [MenuItem::new("about", "About")]),
    ];
    let selected = MenuBar::new("menu", &items).show(ui).selected;
    if selected == Some(Id::new("save")) {
        model.saved += 1;
    } else if selected == Some(Id::new("autosave")) {
        model.autosave = !model.autosave;
    } else if selected == Some(Id::new("about")) {
        model.about = true;
    }
}

fn profile(ui: &mut Ui<'_>, model: &mut Model) {
    ui.horizontal(|ui| {
        ui.add(
            Image::new(&model.logo)
                .size(vec2(40.0, 40.0))
                .alt("zaxis logo"),
        );
        ui.heading("Profile");
        // An icon alone says nothing to a screen reader: the button needs a name.
        let refresh = Button::new("##refresh").icon(&model.logo);
        if ui.add(refresh.accessible_label("Refresh")).clicked() {
            model.refreshed += 1;
        }
    });
    Field::new("Name").show(ui, |ui| {
        ui.add(TextEdit::new(&mut model.name).id_source("name"));
    });
    let problem = model
        .email_problem()
        .map_or(Validation::ok(), Validation::error);
    Field::new("Email")
        .hint("Used for receipts")
        .validation(problem)
        .show(ui, |ui| {
            ui.add(TextEdit::new(&mut model.email).id_source("email"));
        });
    Field::new("Notes").show(ui, |ui| {
        ui.add(
            TextEdit::new(&mut model.notes)
                .multiline()
                .rows(4.0)
                .id_source("notes"),
        );
    });
    ui.checkbox(&mut model.subscribe, "Subscribe to updates");
    ui.add(RadioGroup::new(
        &mut model.plan,
        [
            (Plan::Free, "Free"),
            (Plan::Pro, "Pro"),
            (Plan::Team, "Team"),
        ],
    ));
    ui.add(
        Slider::new(&mut model.volume, 0.0..=100.0)
            .step(5.0)
            .text("Volume"),
    );
    let regions = [
        ComboBoxOption::new("eu", "europe", "Europe"),
        ComboBoxOption::new("us", "america", "America"),
        ComboBoxOption::new("asia", "asia", "Asia"),
    ];
    Field::new("Region").show(ui, |ui| {
        ui.add(
            ComboBox::new(&mut model.region, &regions)
                .id_source("region")
                .width(220.0),
        );
    });
    ui.horizontal(|ui| {
        if ui.button("Save").clicked() {
            model.saved += 1;
        }
        ui.hyperlink_to("Documentation", "https://nihmadev.github.io/zaxis/");
    });
}

fn data(ui: &mut Ui<'_>, model: &mut Model) {
    ui.heading("Files");
    let files = &model.files;
    ListBox::new("files")
        .mode(ListMode::Multiple)
        .selection(&mut model.selected_files)
        .row_height(28.0)
        .max_height(120.0)
        .show_rows(
            ui,
            files.len(),
            |i| ListEntry::item(i, &files[i]),
            |ui, row| {
                ui.label(row.text);
            },
        );
    ui.heading("Project");
    TreeView::new("project")
        .open(&mut model.open_folders)
        .selected(&mut model.selected_node)
        .max_height(140.0)
        .show(ui, &model.projects);
    ui.heading("Members");
    let members = &mut model.members;
    Table::new("members")
        .columns([
            Column::remainder("name").min_width(140.0).title("Name"),
            Column::fixed("role", 110.0).title("Role"),
            Column::fixed("active", 110.0).title("Active"),
        ])
        .max_height(150.0)
        .show_rows(ui, 32.0, members.len(), |body, index| {
            let member = &mut members[index];
            body.row(member.id, |row| {
                row.cell(|ui| {
                    ui.label(member.name);
                });
                row.cell(|ui| {
                    ui.label(member.role);
                });
                row.cell(|ui| {
                    ui.checkbox(&mut member.active, "Active");
                });
            });
        });
}

fn about(ui: &mut Ui<'_>, model: &mut Model) {
    Dialog::new("about", "About")
        .action(DialogAction::new("Close").primary())
        .show(ui, &mut model.about, |ui| {
            ui.label("An accessible form built with zaxis.");
        });
}
