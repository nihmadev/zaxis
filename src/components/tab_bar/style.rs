//! Tokens resolved against the current [`Style`] once per pass: builder, then
//! `Style::tabs`, then defaults derived from the style's own colors.

use super::options::{Config, TabVariant};
use crate::{
    components::{
        font_size,
        theme::{SurfaceStyle, TabsStyle},
    },
    Border, Color, FontWeight, Style, Vec2,
};

pub(super) struct Look {
    pub folder: bool,
    pub scale: f32,
    pub height: f32,
    pub pad_x: f32,
    pub icon: f32,
    pub icon_gap: f32,
    pub close_size: f32,
    pub close_glyph: f32,
    pub close_gap: f32,
    pub min_width: f32,
    pub max_width: f32,
    pub tab_radius: f32,
    pub plate_radius: f32,
    pub plate_inset: Vec2,
    pub divider_height: f32,
    pub font: f32,
    pub weight: FontWeight,
    pub weight_selected: FontWeight,
    /// Thickness of every line: one physical pixel at least.
    pub line: f32,
    pub strip_fill: Option<Color>,
    pub baseline: Color,
    pub frame: Color,
    pub page: Color,
    pub plate: Color,
    pub plate_pressed: Color,
    pub divider: Color,
    pub indicator: Color,
    pub indicator_width: f32,
    pub text_idle: Color,
    pub text_hover: Color,
    pub text_selected: Color,
    pub text_disabled: Color,
    pub close_idle: Color,
    pub close_hover: Color,
    pub close_plate: Color,
    pub close_plate_pressed: Color,
    pub drop_fill: Color,
    pub drop_border: Border,
    pub drop_radius: crate::CornerRadius,
    pub insertion: Color,
    pub insertion_width: f32,
    pub insertion_gap: f32,
}

pub(super) fn snap(value: f32, scale: f32) -> f32 {
    (value * scale).round() / scale
}

fn solid(surface: SurfaceStyle) -> Option<Color> {
    surface.fill.map(|fill| fill.start)
}

fn width_of(border: Option<Border>, fallback: Border, scale: f32) -> f32 {
    snap(border.unwrap_or(fallback).width.max(0.0), scale).max(1.0 / scale)
}

impl Look {
    pub fn resolve(style: &Style, cfg: &Config, scale: f32) -> Self {
        let mut tabs: TabsStyle = style.tabs;
        tabs.merge(cfg.style);
        let folder = cfg.variant == TabVariant::Folder;
        let finite = |v: Option<f32>, d: f32| v.filter(|v| v.is_finite()).map_or(d, |v| v.max(0.0));
        let line_border = tabs.strip.border.unwrap_or(style.border);
        let line = width_of(Some(line_border), style.border, scale);
        let weights = style.typography.weights;
        let (tab, close) = (tabs.tab, tabs.close);
        let page = solid(tab.selected).unwrap_or(style.window_fill);
        let hover_fg = tab.hover.foreground.unwrap_or(style.text_color);
        let selected_fg = tab.selected.foreground.unwrap_or(style.text_color);
        let close_plate = solid(close.hover).unwrap_or(style.button_pressed);
        let height = finite(tabs.height, style.control_height + 2.0).max(8.0);
        let accent_alpha = style.accent.with_alpha(40);
        let min_width = finite(cfg.min_width.or(tabs.min_width), 96.0);
        Self {
            folder,
            scale,
            height: snap(height, scale),
            pad_x: finite(tabs.padding_x, 12.0),
            icon: finite(tabs.icon_size, 16.0),
            icon_gap: finite(tabs.icon_gap, 6.0),
            close_size: finite(tabs.close_size, 18.0),
            close_glyph: finite(tabs.close_glyph, 10.0),
            close_gap: finite(tabs.close_gap, 8.0),
            min_width,
            max_width: finite(cfg.max_width.or(tabs.max_width), 240.0).max(min_width),
            tab_radius: finite(tabs.tab_radius, 8.0),
            plate_radius: finite(tabs.plate_radius, 6.0),
            plate_inset: tabs
                .plate_inset
                .unwrap_or(Vec2::new(2.0, 4.0))
                .max(Vec2::ZERO),
            divider_height: finite(tabs.divider_height, 16.0),
            font: font_size(tabs.font_size.unwrap_or(style.font_size)),
            weight: tabs.font_weight.unwrap_or(weights.control),
            weight_selected: tabs.font_weight.unwrap_or(weights.selected()),
            line,
            strip_fill: folder
                .then(|| solid(tabs.strip).unwrap_or(style.background))
                .filter(|c| c.0[3] > 0),
            baseline: line_border.color,
            frame: tab.selected.border.unwrap_or(style.border).color,
            page,
            plate: solid(tab.hover).unwrap_or(style.button_hovered),
            plate_pressed: solid(tab.pressed).unwrap_or(style.button_pressed),
            divider: tabs.divider.unwrap_or(style.border).color,
            indicator: solid(tabs.indicator).unwrap_or(style.accent),
            indicator_width: 2.0,
            text_idle: tab.idle.foreground.unwrap_or(style.muted_text),
            text_hover: hover_fg,
            text_selected: selected_fg,
            text_disabled: tab.disabled.foreground.unwrap_or(style.disabled_text),
            close_idle: close.idle.foreground.unwrap_or(style.muted_text),
            close_hover: close.hover.foreground.unwrap_or(style.text_color),
            close_plate,
            close_plate_pressed: solid(close.pressed).unwrap_or(style.border.color),
            drop_fill: solid(tabs.drop_area).unwrap_or(accent_alpha),
            drop_border: tabs
                .drop_area
                .border
                .unwrap_or(Border::new(1.5, style.accent)),
            drop_radius: tabs.drop_area.rounding.unwrap_or(style.rounding),
            insertion: tabs
                .insertion_color
                .or(style.drag.line_color)
                .unwrap_or(style.accent),
            insertion_width: finite(tabs.insertion_width, 2.0).max(1.0),
            insertion_gap: finite(tabs.insertion_gap, 8.0),
        }
    }

    /// Top-corner radius of the active tab, never more than half its width.
    pub fn top_radius(&self, width: f32) -> f32 {
        self.tab_radius.min(width * 0.5)
    }
}

impl Look {
    /// How far the close button's hit area reaches into the right padding of its tab.
    pub fn close_overhang(&self) -> f32 {
        ((self.close_size - self.close_glyph) * 0.5).max(0.0)
    }

    /// The width a close button takes inside its tab, without its distance from the label.
    pub fn close_reserve(&self) -> f32 {
        self.close_size - self.close_overhang()
    }
}
