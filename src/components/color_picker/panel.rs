//! Where the editor is shown: inline below the row (revealed with the expand motion; the
//! only form inside a modal), or in a floating window of its own with a close button.

use super::{
    color::FIELDS,
    editor::{self, Editor},
    fields::field_id,
    visible_label, ColorPickerState, HitAction, Paint, Shape,
};
use crate::{
    accessibility::Scope, components::appearance::Appearance, AccessRole, Border, Color,
    CornerRadius, Id, Padding, Rect, Response, Ui, Vec2, Window,
};
use winit::keyboard::KeyCode;

/// The editor of one picker and the room it takes.
pub(super) struct Panel<'s> {
    pub editor: Editor<'s>,
    pub label: &'s str,
    /// Width of the row; the inline editor takes it.
    pub width: f32,
    /// Width set on the picker, for the floating window.
    pub requested_width: Option<f32>,
    pub height: f32,
}

/// Escape closes the editor unless a channel field has focus: there it cancels the field's
/// draft first.
fn field_focused(ui: &Ui<'_>, id: Id) -> bool {
    (0..FIELDS).any(|field| ui.context.has_focus(field_id(id, field)))
}

/// Reveal the editor below `row`; returns the height it takes in this pass.
pub(super) fn inline(
    ui: &mut Ui<'_>,
    panel: &Panel<'_>,
    row: Rect,
    (state, color): (&mut ColorPickerState, &mut Color),
    response: &mut Response,
) -> f32 {
    let id = panel.editor.id;
    let open = state.open;
    let editor = Editor {
        enabled: panel.editor.enabled && open,
        ..panel.editor
    };
    let style = editor.style;
    let reveal_id = ui.scope.with(("reveal", Id::new(id)));
    if open && ui.context.visuals.effect_value(reveal_id).is_none() {
        // Preserve the existing first-appearance snap for default_open.
        ui.context
            .transition(reveal_id, panel.height, style.motion.expand.clone());
    }
    ui.layout.cursor.y = row.max.y;
    let gap = style.color_picker.gap.unwrap_or(10.0);
    ui.reveal(id, open, |ui| {
        let origin = ui.layout.cursor + Vec2::new(0.0, gap);
        let group = if open {
            ui.a11y_begin(id.with("editor"), AccessRole::Group, |node| {
                node.label(visible_label(panel.label));
            })
        } else {
            Scope::NONE
        };
        editor::show(ui, &editor, (origin, panel.width), (state, color), response);
        ui.a11y_end(group, None);
        ui.allocate_space(Vec2::new(panel.width, panel.height));
    });
    let height = ui.context.visuals.effect_value(reveal_id).unwrap_or(0.0);
    if ui.context.input().keys_pressed.contains(&KeyCode::Escape)
        && response.has_focus
        && !field_focused(ui, id)
    {
        state.open = false;
        ui.context.request_repaint();
    }
    height
}

/// Show the editor in its own window below `row`, closing it from its close button or
/// Escape while it is the front window.
pub(super) fn floating(
    ui: &mut Ui<'_>,
    panel: &Panel<'_>,
    row: Rect,
    (state, color): (&mut ColorPickerState, &mut Color),
    response: &mut Response,
) {
    let editor = &panel.editor;
    let (id, style) = (editor.id, editor.style);
    let window_id = id.with("floating");
    let size = Vec2::new(
        panel
            .requested_width
            .or(style.color_picker.width)
            .unwrap_or(300.0)
            .max(260.0)
            + 24.0,
        style.title_height.max(24.0) + panel.height + 4.0,
    );
    let viewport = ui.context.viewport().size();
    let position = Vec2::new(row.min.x, row.max.y + 8.0)
        .min((viewport - size).max(Vec2::ZERO))
        .max(Vec2::ZERO);
    Window::new(visible_label(panel.label))
        .id(window_id)
        .default_position(position)
        .default_size(size)
        .min_size(size)
        .resizable(false)
        .padding(Padding::all(12.0))
        .effective_style(style.clone())
        .show(ui.context, |window| {
            let origin = window.layout.cursor;
            let width = window.available_width();
            editor::show(window, editor, (origin, width), (state, color), response);
            let closed = close_button(window, editor, window_id);
            let escape = window
                .context
                .input()
                .keys_pressed
                .contains(&KeyCode::Escape)
                && !field_focused(window, id)
                && window.context.front_window() == Some(window_id);
            if closed || escape {
                state.commit(color);
                state.open = false;
                window.context.request_repaint();
            }
        });
    if response.clicked() {
        ui.context.raise_window(window_id);
    }
}

/// The window's corner close button. Returns whether it was clicked.
fn close_button(ui: &mut Ui<'_>, editor: &Editor<'_>, window_id: Id) -> bool {
    let (id, style) = (editor.id, editor.style);
    let window = ui.context.windows[&window_id].rect;
    let clip = window.intersect(ui.context.viewport());
    let close = Rect::from_min_size(
        Vec2::new(window.max.x - 30.0, window.min.y + 6.0),
        Vec2::splat(24.0),
    );
    let close_id = id.with("close");
    let response = ui.response(close_id, close, true);
    ui.context.register_hit(crate::context::HitRegion {
        id: close_id,
        window: window_id,
        rect: close,
        clip,
        action: HitAction::Activate,
    });
    ui.context.a11y_leaf(
        window_id,
        close_id,
        AccessRole::Button,
        (close, clip),
        |node| {
            node.label("Close").clicks(close_id);
        },
    );
    let preset = editor.hover.or(ui.hover_style).unwrap_or(style.hover_style);
    let ring = if response.focus_visible {
        style.focus_border
    } else {
        Border::NONE
    };
    let hover = ui.animate_hover(
        response,
        preset,
        Appearance::new(Color::TRANSPARENT, ring, style.text_color),
        style.button_hovered,
    );
    let rounding = style
        .color_picker
        .rounding
        .unwrap_or(CornerRadius::all(4.0));
    let mut paint = Vec::new();
    hover.paint_shadow(close, rounding, &mut paint);
    hover.paint_body(close, rounding, style, 0.0, &mut paint);
    for direction in [-1.0, 1.0] {
        paint.push(Paint::Shape(Shape::Line {
            start: close.center() + Vec2::new(-4.0, -4.0 * direction),
            end: close.center() + Vec2::new(4.0, 4.0 * direction),
            width: 1.5,
            color: style.text_color,
        }));
    }
    ui.context
        .paint(close_id.with("body"), window_id, clip, paint);
    response.clicked()
}
