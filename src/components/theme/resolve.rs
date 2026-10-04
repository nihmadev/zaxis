use super::*;
use crate::{Border, CornerRadius, Padding, Style, Vec2};

impl Theme {
    pub(super) fn resolve_style(&self) -> Style {
        let mut s = Style::default();
        let p = self.palette;
        let m = self.metrics;
        let d = self.density.factor();
        let height = m.control_height * d;
        let radius = CornerRadius::all(m.corner_radius);
        let border = Border::new(m.border_width, p.border);
        let padding = |p: Padding| Padding {
            left: p.left * d,
            right: p.right * d,
            top: p.top * d,
            bottom: p.bottom * d,
        };
        s.background = p.background;
        s.window_fill = p.surface;
        s.title_fill = p.surface;
        s.text_color = p.foreground;
        s.muted_text = p.muted;
        s.disabled_text = p.disabled;
        s.accent = p.accent;
        s.on_accent = p.on_accent;
        s.selected_fill = p.selected;
        s.selected_text = p.on_selected;
        s.success = p.success;
        s.on_success = p.on_success;
        s.warning = p.warning;
        s.on_warning = p.on_warning;
        s.error = p.error;
        s.on_error = p.on_error;
        s.border = border;
        s.focus_border = Border::new(m.border_width.max(1.5), p.focus);
        s.hover_style = crate::HoverStyle {
            fill: Some(crate::HoverFill::Theme),
            ..crate::HoverStyle::NONE
        };
        s.button_fill = p.surface_control;
        s.button_hovered = p.hover;
        s.button_pressed = p.pressed;
        // Fields are wells (darker than the panel); buttons are raised.
        s.text_edit_fill = p.background;
        s.text_edit_hovered = p.surface_raised;
        s.text_edit_placeholder = p.muted;
        s.text_edit_selection =
            crate::Color::rgba(p.accent.0[0], p.accent.0[1], p.accent.0[2], 110);
        s.text_edit.caret = Some(p.foreground);
        s.text_edit.selection_foreground = Some(p.on_selected);
        // Text areas follow the Radix/shadcn proportions: 12x8 padding, 64px minimum height.
        s.text_edit.area_padding = Some(padding(Padding::symmetric(12.0, 8.0)));
        s.text_edit.area_min_height = Some(64.0 * d);
        s.rounding = radius;
        s.text_edit_rounding = radius;
        s.font_size = self.typography.body;
        s.typography = self.typography;
        s.text_edit_font_size = self.typography.body;
        s.spacing = m.spacing * d;
        s.window_padding = padding(m.padding);
        s.button_padding = padding(s.button_padding);
        s.text_edit_padding = padding(s.text_edit_padding);
        s.control_height = height;
        s.text_edit_height = height;
        s.title_height = height + 2.0;
        s.blur_radius = super::super::blur::normalize_radius(m.blur);
        s.blur_opacity = m.blur_opacity;
        s.opacity = m.opacity;
        s.elevation = m.elevation;
        s.button.surface.idle.rounding = Some(radius);
        s.checkbox.body.idle.rounding = Some(CornerRadius::all((m.corner_radius - 2.0).max(2.0)));
        s.checkbox.size = Some(16.0 * d);
        s.checkbox.gap = Some(8.0 * d);
        s.switch.height = Some(20.0 * d);
        s.switch.width = Some(36.0 * d);
        s.switch.gap = Some(8.0 * d);
        s.slider.height = Some(height);
        s.slider.track_height = Some(6.0 * d);
        s.slider.thumb_radius = Some(8.0 * d);
        // Radix-style handles: white thumb with a soft drop shadow, no outline.
        s.slider.track.idle = SurfaceStyle::fill(p.hover);
        s.slider.thumb.idle = SurfaceStyle {
            border: Some(Border::NONE),
            shadow: Some(handle_shadow()),
            ..SurfaceStyle::fill(crate::Color::WHITE)
        };
        s.slider.thumb.hover = SurfaceStyle::fill(crate::Color::gray(240));
        s.slider.thumb.pressed = SurfaceStyle::fill(crate::Color::gray(225));
        s.switch.track.idle = SurfaceStyle {
            border: Some(Border::NONE),
            ..SurfaceStyle::fill(p.hover)
        };
        s.switch.track.selected = SurfaceStyle::fill(p.accent);
        s.switch.thumb.idle = SurfaceStyle {
            border: Some(Border::NONE),
            shadow: Some(handle_shadow()),
            ..SurfaceStyle::fill(crate::Color::WHITE)
        };
        s.switch.thumb.selected = SurfaceStyle::fill(crate::Color::WHITE);
        s.switch.thumb_inset = Some(2.0 * d);
        s.popup.surface.rounding = Some(radius);
        s.popup.surface.fill = Some(crate::Gradient::new(p.surface_raised, p.surface_raised));
        s.popup.surface.shadow = Some(floating_shadow(&p, 8.0, 24.0, 110));
        s.window.body.shadow = Some(floating_shadow(&p, 10.0, 32.0, 90));
        s.window.title_font_size = Some(self.typography.body);
        s.popup.padding = Some(Padding::all(4.0 * d));
        s.popup.gap = Some(4.0 * d);
        s.popup.spacing = Some(2.0 * d);
        let light = p.surface.0[0] > 128;
        s.modal.overlay = Some(crate::Color::rgba(0, 0, 0, if light { 102 } else { 153 }));
        s.modal.overlay_blur = Some(0.0);
        s.modal.surface = SurfaceStyle {
            border: Some(border),
            rounding: Some(CornerRadius::all(m.corner_radius + 3.0)),
            shadow: Some(floating_shadow(&p, 12.0, 48.0, 130)),
            ..SurfaceStyle::fill(p.surface_raised)
        };
        s.modal.padding = Some(Padding::all(20.0 * d));
        s.modal.gap = Some(16.0 * d);
        s.modal.spacing = Some(8.0 * d);
        s.modal.min_width = Some(280.0);
        s.modal.max_width = Some(520.0);
        s.modal.margin = Some(16.0 * d);
        s.modal.enter_scale = Some(0.97);
        s.modal.enter_offset = Some(5.0);
        s.modal.title = Some(TypographyRole::Heading);
        s.modal.description = Some(TypographyRole::Body);
        s.modal.close_size = Some(height - 4.0 * d);
        s.card.surface = SurfaceStyle {
            border: Some(border),
            rounding: Some(CornerRadius::all(m.corner_radius + 3.0)),
            shadow: Some(crate::Shadow {
                color: crate::Color::rgba(0, 0, 0, 60),
                offset: Vec2::new(0.0, 1.0),
                blur_radius: 3.0,
                spread: 0.0,
            }),
            ..SurfaceStyle::fill(p.surface_raised)
        };
        s.card.padding = Some(Padding::all(16.0 * d));
        s.color_picker.rounding = Some(radius);
        s.color_picker.row_height = Some(height);
        s.color_picker.gap = Some(10.0 * d);
        s.color_picker.field_gap = Some(5.0 * d);
        s.color_picker.palette_height = Some(112.0 * d);
        s.color_picker.hue_height = Some(14.0 * d);
        s.number.fill = p.background;
        s.number.hovered = p.surface_raised;
        s.number.invalid = p.error;
        s.number.height = height;
        s.number.padding = padding(s.number.padding);
        s.number.rounding = radius;
        s.number.font_size = self.typography.body;
        s.scroll.spacing = 8.0 * d;
        s.scroll.padding = padding(s.scroll.padding);
        s.scroll.bar_width = 4.0 * d;
        s.scroll.bar_margin *= d;
        s.scroll.thumb_color = crate::Color::rgba(p.muted.0[0], p.muted.0[1], p.muted.0[2], 110);
        s.scroll.thumb_hovered = p.muted;
        s.scroll.hint_color = crate::Color::rgba(0, 0, 0, 16);
        s.grid.fill = p.surface_raised;
        s.grid.rounding = m.corner_radius;
        s.grid.padding = padding(s.grid.padding);
        s.grid.cell_padding = padding(s.grid.cell_padding);
        s.grid.spacing *= d;
        s.grid.component_spacing *= d;
        s.table.fill = p.surface;
        s.table.header_fill = p.surface;
        s.table.striped = false;
        s.table.alternate_fill = p.surface;
        s.table.hovered_fill = p.surface_raised;
        s.table.selected_fill = p.selected;
        s.table.row.selected.foreground = Some(p.on_selected);
        s.table.text_color = p.foreground;
        s.table.border = border;
        s.table.separator_color = p.border;
        s.table.separator_width = m.border_width;
        s.table.rounding = radius;
        s.table.padding = padding(s.table.padding);
        s.table.cell_padding = padding(s.table.cell_padding);
        s.table.header_height = height;
        s.table.row_min_height = height;
        s.table.component_spacing *= d;
        s.table.font_size = self.typography.body;
        s.table.scroll = crate::ScrollStyle {
            padding: Padding::default(),
            spacing: 0.0,
            ..s.scroll
        };
        let c = &mut s.combo_box;
        c.trigger_fill = p.background;
        c.popup_fill = p.surface_raised;
        c.text = p.foreground;
        c.muted_text = p.muted;
        c.disabled_text = p.disabled;
        c.option.selected.foreground = Some(p.on_selected);
        c.check_color = p.foreground;
        c.active_fill = p.accent;
        c.border = border;
        c.hover_border = Border::new(m.border_width, p.focus);
        c.popup_border = border;
        c.rounding = radius;
        c.trigger_height = height;
        c.row_height = height - 4.0 * d;
        c.label_height *= d;
        c.label_gap *= d;
        c.row_gap *= d;
        c.popup_gap *= d;
        c.trigger_padding = padding(c.trigger_padding);
        c.popup_padding = padding(c.popup_padding);
        c.font_size = self.typography.body;
        c.label_font_size = self.typography.body;
        s.split.handle.idle = p.border;
        s.split.handle.hover = p.accent;
        s.split.handle.pressed = p.accent;
        s.split.handle.focus = p.focus;
        s.split.gap *= d;
        s.split.spacing *= d;
        s.collapsing.header.height = Some(height);
        s.collapsing.header.padding = Some(Padding::symmetric(6.0 * d, 2.0 * d));
        s.collapsing.header.chevron_size = Some(12.0 * d);
        s.collapsing.header.icon_size = Some(16.0 * d);
        s.collapsing.header.icon_gap = Some(6.0 * d);
        s.collapsing.header.font_size = Some(self.typography.body);
        s.collapsing.header.surface.idle.fill =
            Some(crate::Gradient::new(p.surface_raised, p.surface_raised));
        s.collapsing.header.surface.hover = SurfaceStyle::fill(p.surface_control);
        s.collapsing.header.surface.pressed.fill = Some(crate::Gradient::new(p.pressed, p.pressed));
        s.collapsing.content_padding = Some(Padding {
            left: 18.0 * d,
            ..Padding::default()
        });
        s.tree.row = s.collapsing.header.clone();
        s.tree.row.surface.idle.fill = Some(crate::Gradient::new(
            crate::Color::TRANSPARENT,
            crate::Color::TRANSPARENT,
        ));
        s.tree.row.surface.hover = SurfaceStyle::fill(p.surface_raised);
        s.tree.row.surface.selected = SurfaceStyle::fill(p.selected);
        s.tree.row.surface.selected.foreground = Some(p.on_selected);
        s.tree.indent = Some(18.0 * d);
        s.progress.size = Some(Vec2::new(160.0, 4.0 * d));
        // Explicit values apply after all palette/density derivation.
        self.overrides.apply(&mut s);
        s
    }
}

/// Soft, wide drop shadow for floating layers; fades out on light palettes.
fn floating_shadow(p: &Palette, offset: f32, blur: f32, alpha: u8) -> crate::Shadow {
    let light = p.surface.0[0] > 128;
    crate::Shadow {
        color: crate::Color::rgba(0, 0, 0, if light { alpha / 3 } else { alpha }),
        offset: Vec2::new(0.0, offset),
        blur_radius: blur,
        spread: 0.0,
    }
}

fn handle_shadow() -> crate::Shadow {
    crate::Shadow {
        color: crate::Color::rgba(0, 0, 0, 90),
        offset: Vec2::new(0.0, 1.0),
        blur_radius: 2.5,
        spread: 0.0,
    }
}
