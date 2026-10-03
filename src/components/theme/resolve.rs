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
        s.title_fill = p.surface_raised;
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
        s.button_fill = p.surface_control;
        s.button_hovered = p.hover;
        s.button_pressed = p.pressed;
        s.text_edit_fill = p.surface_raised;
        s.text_edit_hovered = p.surface_control;
        s.text_edit_placeholder = p.muted;
        s.text_edit_selection = p.selected;
        s.text_edit.caret = Some(p.foreground);
        s.text_edit.selection_foreground = Some(p.on_selected);
        s.rounding = radius;
        s.text_edit_rounding = radius;
        s.font_size = self.typography.body;
        s.typography = self.typography;
        s.text_edit_font_size = self.typography.small;
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
        s.checkbox.body.idle.rounding = Some(radius);
        s.checkbox.size = Some(20.0 * d);
        s.checkbox.gap = Some(8.0 * d);
        s.slider.height = Some(height);
        s.slider.track_height = Some(6.0 * d);
        s.slider.thumb_radius = Some(8.0 * d);
        s.popup.surface.rounding = Some(radius);
        s.popup.padding = Some(Padding::all(4.0 * d));
        s.popup.gap = Some(4.0 * d);
        s.popup.spacing = Some(2.0 * d);
        s.color_picker.rounding = Some(radius);
        s.color_picker.row_height = Some(height);
        s.color_picker.gap = Some(10.0 * d);
        s.color_picker.field_gap = Some(5.0 * d);
        s.color_picker.palette_height = Some(112.0 * d);
        s.color_picker.hue_height = Some(14.0 * d);
        s.number.fill = p.surface_raised;
        s.number.hovered = p.surface_control;
        s.number.invalid = p.error;
        s.number.height = height;
        s.number.padding = padding(s.number.padding);
        s.number.rounding = radius;
        s.number.font_size = self.typography.small;
        s.scroll.spacing = 8.0 * d;
        s.scroll.padding = padding(s.scroll.padding);
        s.scroll.bar_width = 2.0 * d;
        s.scroll.bar_margin *= d;
        s.scroll.thumb_color = p.muted;
        s.scroll.thumb_hovered = p.accent;
        s.scroll.hint_color = crate::Color::rgba(0, 0, 0, 16);
        s.grid.fill = p.surface_raised;
        s.grid.rounding = m.corner_radius;
        s.grid.padding = padding(s.grid.padding);
        s.grid.cell_padding = padding(s.grid.cell_padding);
        s.grid.spacing *= d;
        s.grid.component_spacing *= d;
        s.table.fill = p.surface;
        s.table.header_fill = p.surface_raised;
        s.table.alternate_fill = p.surface_raised;
        s.table.hovered_fill = p.hover;
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
        s.table.font_size = self.typography.small;
        s.table.scroll = crate::ScrollStyle {
            padding: Padding::default(),
            spacing: 0.0,
            ..s.scroll
        };
        let c = &mut s.combo_box;
        c.trigger_fill = p.surface_raised;
        c.popup_fill = p.surface;
        c.text = p.foreground;
        c.muted_text = p.muted;
        c.disabled_text = p.disabled;
        c.option.selected.foreground = Some(p.on_selected);
        c.check_color = p.accent;
        c.active_fill = p.selected;
        c.border = border;
        c.hover_border = Border::new(m.border_width, p.focus);
        c.popup_border = border;
        c.rounding = radius;
        c.trigger_height = height;
        c.row_height = height;
        c.label_height *= d;
        c.label_gap *= d;
        c.row_gap *= d;
        c.popup_gap *= d;
        c.trigger_padding = padding(c.trigger_padding);
        c.popup_padding = padding(c.popup_padding);
        c.font_size = self.typography.small;
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
        s.collapsing.header.font_size = Some(self.typography.small);
        s.collapsing.header.surface.idle.fill =
            Some(crate::Gradient::new(p.surface_raised, p.surface_raised));
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
        s.tree.row.surface.selected = SurfaceStyle::fill(p.selected);
        s.tree.row.surface.selected.foreground = Some(p.on_selected);
        s.tree.indent = Some(18.0 * d);
        s.progress.size = Some(Vec2::new(160.0, 4.0 * d));
        // Explicit values apply after all palette/density derivation.
        self.overrides.apply(&mut s);
        s
    }
}
