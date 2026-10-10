//! One set of commands behind a menu bar, a context menu, a toolbar and the keyboard.
//! `Ctrl+K Ctrl+C` is a chord that works while the text has focus.
#[path = "actions/commands.rs"]
mod commands;
#[path = "actions/model.rs"]
mod model;
#[path = "actions/shortcuts.rs"]
mod shortcuts;

use commands::Cmd;
use model::Notes;
use zaxis::{
    App, Button, Context, ContextMenuItem, Frame, InputEvent, KeyInput, MenuBar, MenuItem, Root,
    TextEdit, Ui,
};

struct Demo {
    notes: Notes,
    smoke: Option<u32>,
}

impl Demo {
    fn menus() -> Vec<MenuItem> {
        let action = |cmd: Cmd| MenuItem::action(cmd);
        vec![
            MenuItem::submenu(
                "File",
                [
                    action(Cmd::New),
                    action(Cmd::Save),
                    action(Cmd::Revert),
                    MenuItem::separator(),
                    action(Cmd::Quit),
                ],
            ),
            MenuItem::submenu("Edit", [action(Cmd::Clear)]),
            MenuItem::submenu(
                "View",
                [
                    action(Cmd::Wrap),
                    action(Cmd::Statistics),
                    MenuItem::separator(),
                    action(Cmd::ZoomIn),
                    action(Cmd::ZoomOut),
                    action(Cmd::ZoomReset),
                ],
            ),
            MenuItem::submenu("Tools", [action(Cmd::Shortcuts)]),
        ]
    }

    fn context_menu() -> Vec<ContextMenuItem> {
        vec![
            ContextMenuItem::action(Cmd::Save),
            ContextMenuItem::action(Cmd::Clear),
            ContextMenuItem::separator(),
            ContextMenuItem::action(Cmd::Wrap),
            ContextMenuItem::action(Cmd::ZoomIn),
            ContextMenuItem::action(Cmd::ZoomOut),
        ]
    }

    fn ui(&mut self, ui: &mut Ui<'_>) {
        ui.horizontal(|ui| {
            MenuBar::new("menu", &Self::menus()).show(ui);
        });
        ui.horizontal(|ui| {
            for cmd in [
                Cmd::New,
                Cmd::Save,
                Cmd::Revert,
                Cmd::Wrap,
                Cmd::ZoomOut,
                Cmd::ZoomIn,
            ] {
                ui.add(Button::action(cmd));
            }
        });
        let height = (ui.available_height() - 30.0).max(80.0);
        let (wrap, size) = (self.notes.wrap, self.notes.font_size());
        let editor = ui.add(
            TextEdit::new(&mut self.notes.text)
                .id_source("editor")
                .multiline()
                .wrap(wrap)
                .font_size(size)
                .height(height),
        );
        // Keys that belong to the editor only, such as the chord that clears it.
        if editor.has_focus {
            ui.actions().set_context("editor");
        }
        ui.context_menu(editor, &Self::context_menu());
        ui.horizontal(|ui| {
            if self.notes.statistics {
                let text = &self.notes.text;
                ui.label(format!(
                    "{} lines   {} words   {} characters",
                    text.lines().count().max(1),
                    text.split_whitespace().count(),
                    text.chars().count()
                ));
            }
            ui.spacer();
            if let Some(pending) = ui.actions().pending_chord() {
                ui.label(pending.text);
            }
        });
    }

    /// Drives the example without a person: every path to a command, then a round trip of
    /// the saved keymap. Panics when a step does not do what the interface promises.
    fn smoke(&mut self, context: &mut Context, step: u32) {
        let press = |context: &mut Context, mods, code, letter: &str| {
            use zaxis::winit::{
                event::ElementState,
                keyboard::{Key, PhysicalKey},
            };
            context.on_input(InputEvent::Modifiers(mods));
            for state in [ElementState::Pressed, ElementState::Released] {
                context.on_input(InputEvent::Key(KeyInput {
                    physical: PhysicalKey::Code(code),
                    logical: Key::Character(letter.into()),
                    state,
                    repeat: false,
                    text: None,
                }));
            }
            context.on_input(InputEvent::Modifiers(Default::default()));
        };
        use zaxis::winit::keyboard::{KeyCode, ModifiersState};
        let notes = &mut self.notes;
        match step {
            1 => notes.text = "one two\nthree".into(),
            2 => assert!(context.actions().trigger(Cmd::Wrap)),
            3 => {
                assert!(!notes.wrap, "the menu, button and key share Wrap");
                press(context, ModifiersState::CONTROL, KeyCode::KeyS, "s");
            }
            4 => {
                assert_eq!(notes.saved, notes.text, "Ctrl+S saved");
                notes.text.push('!');
                press(context, ModifiersState::CONTROL, KeyCode::KeyK, "k");
                press(context, ModifiersState::CONTROL, KeyCode::KeyC, "c");
            }
            5 => {
                // No editor focus in this run: the chord is bound to the editor context.
                assert!(notes.text.ends_with('!'), "Clear needs the editor context");
                let mut actions = context.actions();
                assert!(actions
                    .keymap_mut()
                    .rebind(Cmd::Save, Some("mod+shift+s".parse().unwrap()))
                    .is_empty());
                let text = actions.keymap().save();
                let clean = actions.keymap_mut().rebind(Cmd::Save, None);
                assert!(clean.is_empty());
                assert!(actions.keymap_mut().load(&text).is_empty());
                assert_eq!(actions.keymap().save(), text, "the keymap round-trips");
            }
            _ => {
                assert!(
                    context.diagnostics().is_empty(),
                    "{:?}",
                    context.diagnostics()
                );
                notes.quit = true;
            }
        }
    }
}

impl App for Demo {
    fn update(&mut self, context: &mut Context, frame: &mut Frame<'_>) {
        if context.action_registry().is_empty() {
            context.set_actions(commands::registry());
        }
        self.notes.publish(context);
        Root::new().show(context, |ui| self.ui(ui));
        shortcuts::show(context, &mut self.notes.shortcuts_open);
        self.notes.run(context);
        if let Some(step) = &mut self.smoke {
            *step += 1;
            let step = *step;
            self.smoke(context, step);
            context.request_repaint();
        }
        if self.notes.quit {
            frame.close();
        }
    }
}

fn main() -> Result<(), zaxis::RunError> {
    zaxis::run_with_options(
        Demo {
            notes: Notes::new(),
            smoke: std::env::args()
                .any(|arg| arg == "--smoke-test")
                .then_some(0),
        },
        zaxis::RunOptions {
            window_attributes: zaxis::winit::window::Window::default_attributes()
                .with_title("zaxis — Actions")
                .with_inner_size(zaxis::winit::dpi::LogicalSize::new(760.0, 520.0)),
            ..Default::default()
        },
    )
}
