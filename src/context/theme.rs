use super::Context;
use crate::{Id, Interpolate, Style, Theme, Tween, TweenOptions};

impl Context {
    /// Installed theme source; None after direct set_style or on a new Context.
    pub fn theme(&self) -> Option<&Theme> {
        self.theme.as_ref()
    }
    /// Resolves only when tokens/overrides change. Native background follows Context.
    /// Input, focus, editing history, popups and scroll state are preserved.
    pub fn set_theme(&mut self, theme: Theme) {
        if self.theme.as_ref() == Some(&theme) {
            return;
        }
        let style = theme.resolve();
        self.set_style(style);
        self.theme = Some(theme);
    }
    /// Explicit palette transition; geometry/font metrics take their new values now.
    /// Uses the normal scheduler, and snaps when reduced_motion is enabled.
    pub fn set_theme_animated(&mut self, theme: Theme, options: TweenOptions) {
        if self.theme.as_ref() == Some(&theme) {
            return;
        }
        assert!(
            options.repeat == crate::Repeat::Once && !options.auto_reverse,
            "theme transition must finish at its target"
        );
        let from = self.style.clone();
        self.set_theme(theme);
        let to = self.style.clone();
        if to.motion.reduced_motion || from == to {
            return;
        }
        self.restart_animation(
            Id::new("theme-palette"),
            Tween::with_options(from.interpolate(&to, 0.0), to.clone(), options),
        );
        self.palette_transition = Some(());
        self.sample_theme_palette();
    }
    pub(super) fn sample_theme_palette(&mut self) {
        if self.palette_transition.is_none() {
            return;
        }
        let id = Id::new("theme-palette");
        let pass = self.animation_pass(true);
        if let Some(value) = self.animations.read::<Style>(id, pass) {
            let completed = value.completed();
            self.style = value.value;
            if completed {
                self.palette_transition = None;
                self.style_revision = self.style_revision.wrapping_add(1);
                self.animations.remove(id);
            }
        }
    }
}
