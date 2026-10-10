use super::commands::Cmd;
use zaxis::Context;

pub const ZOOM_STEPS: std::ops::RangeInclusive<i32> = -4..=12;
const BASE_SIZE: f32 = 15.0;

#[derive(Default)]
pub struct Notes {
    pub text: String,
    pub saved: String,
    pub wrap: bool,
    pub zoom: i32,
    pub statistics: bool,
    pub shortcuts_open: bool,
    pub quit: bool,
}

impl Notes {
    pub fn new() -> Self {
        Self {
            wrap: true,
            statistics: true,
            ..Default::default()
        }
    }

    pub fn dirty(&self) -> bool {
        self.text != self.saved
    }

    pub fn font_size(&self) -> f32 {
        BASE_SIZE + self.zoom as f32
    }

    /// What the application knows about its commands this pass. Set it before the
    /// widgets that show the commands are built.
    pub fn publish(&self, context: &mut Context) {
        let mut actions = context.actions();
        actions.set_enabled(Cmd::Save, self.dirty());
        actions.set_enabled(Cmd::Revert, self.dirty());
        actions.set_enabled(Cmd::Clear, !self.text.is_empty());
        actions.set_enabled(Cmd::ZoomIn, self.zoom < *ZOOM_STEPS.end());
        actions.set_enabled(Cmd::ZoomOut, self.zoom > *ZOOM_STEPS.start());
        actions.set_enabled(Cmd::ZoomReset, self.zoom != 0);
        actions.set_checked(Cmd::Wrap, self.wrap);
        actions.set_checked(Cmd::Statistics, self.statistics);
    }

    /// Run every command that was asked for since the last pass, from wherever.
    pub fn run(&mut self, context: &mut Context) {
        for cmd in Cmd::ALL {
            if context.actions().triggered(cmd) {
                self.apply(cmd);
            }
        }
    }

    fn apply(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::New => {
                self.text.clear();
                self.saved.clear();
            }
            Cmd::Save => self.saved = self.text.clone(),
            Cmd::Revert => self.text = self.saved.clone(),
            Cmd::Clear => self.text.clear(),
            Cmd::Wrap => self.wrap = !self.wrap,
            Cmd::ZoomIn => self.zoom = (self.zoom + 1).min(*ZOOM_STEPS.end()),
            Cmd::ZoomOut => self.zoom = (self.zoom - 1).max(*ZOOM_STEPS.start()),
            Cmd::ZoomReset => self.zoom = 0,
            Cmd::Statistics => self.statistics = !self.statistics,
            Cmd::Shortcuts => self.shortcuts_open = true,
            Cmd::Quit => self.quit = true,
        }
    }
}
