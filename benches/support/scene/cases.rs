use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Case {
    Access(crate::access::AccessCase),
    Actions(crate::actions::ActionsCase),
    Keys(crate::keys::KeysCase),
    Dock(crate::dock::DockCase),
    Dnd(crate::dnd::DndCase),
    Disclosure(crate::disclosure::DisclosureCase),
    ListBox(crate::list_box::ListCase),
    Carousel(crate::carousel::CarouselCase),
    PanZoom(crate::pan_zoom::PanZoomCase),
    Images(crate::images::ImageCase),
    Material(crate::material::MaterialCase),
    Modal(crate::modal::ModalCase),
    Popup(crate::popup::PopupCase),
    TextArea(crate::text_area::TextAreaCase),
    Number(crate::number::NumberCase),
    Split(split::SplitCase),
    Cold,
    Cached,
    Repaint,
    Schedule,
    Dynamic,
    Button,
    Followup,
    Checkbox,
    Slider,
    Hover,
    Press,
    Drag,
    Resize,
    Tabs,
    Disabled,
    Cancel,
    FocusLoss,
    Wheel,
    ScrollSettings,
    ScrollRows,
    ScrollNested,
    ScrollMiddle,
    ScrollCached,
    ComboClosed,
    ComboOpen,
    ComboToggle,
    ComboKeys,
    ComboFilter,
    ComboUpdates,
    ComboScroll,
    Ime,
    Theme,
    Dpi,
    Lifecycle,
    Text,
    WrappedText,
    TextWeights,
    Shapes,
    Blur0,
    Blur8,
    Blur24,
    Blur64,
    BlurDynamic,
    BlurStack,
    ControlBlur,
    ProtocolCached,
    ProtocolGeometry,
    ProtocolTexture,
    KeyboardButton,
    KeyboardCheckbox,
    KeyboardSlider,
    KeyboardTab,
    ColorPickerClosed,
    ColorPickerInternal,
    ColorPickerFloating,
    ColorPickerPalette,
    ColorPickerHue,
    ColorPickerInput,
    ColorPickerDrag,
    ColorPickerFloatingDrag,
    ColorPickerParentDrag,
    BlurWindowDrag,
    ColorPickerInternalBlurDrag,
    ColorPickerFloatingBlurDrag,
}

impl Case {
    pub const ALL: [Self; 183] = catalog::ALL;
    pub fn name(self) -> &'static str {
        match self {
            Self::Access(case) => case.name(),
            Self::Actions(case) => case.name(),
            Self::Keys(case) => case.name(),
            Self::Dock(case) => case.name(),
            Self::Dnd(case) => case.name(),
            Self::Disclosure(case) => case.name(),
            Self::ListBox(case) => case.name(),
            Self::Carousel(case) => case.name(),
            Self::PanZoom(case) => case.name(),
            Self::Images(case) => case.name(),
            Self::Material(case) => case.name(),
            Self::Modal(case) => case.name(),
            Self::Popup(case) => case.name(),
            Self::TextArea(case) => case.name(),
            Self::Number(case) => case.name(),
            Self::Split(case) => case.name(),
            Self::Cold => "cold_ui",
            Self::Cached => "cached_ui",
            Self::Repaint => "explicit_repaint",
            Self::Schedule => "repaint_deadlines",
            Self::Dynamic => "dynamic_widgets",
            Self::Button => "button_click",
            Self::Followup => "model_followup",
            Self::Checkbox => "checkbox_click",
            Self::Slider => "slider_drag",
            Self::Hover => "hover",
            Self::Press => "press_release",
            Self::Drag => "window_drag",
            Self::Resize => "window_resize",
            Self::Tabs => "tab_switch",
            Self::Disabled => "disabled_click",
            Self::Cancel => "release_outside",
            Self::FocusLoss => "focus_loss",
            Self::Wheel => "wheel_input",
            Self::ScrollSettings => "scroll_settings",
            Self::ScrollRows => "scroll_virtual_rows",
            Self::ScrollNested => "scroll_nested",
            Self::ScrollMiddle => "scroll_middle",
            Self::ScrollCached => "scroll_cached",
            Self::ComboClosed => "combo_closed_cached",
            Self::ComboOpen => "combo_open_cached",
            Self::ComboToggle => "combo_toggle_animation",
            Self::ComboKeys => "combo_keyboard",
            Self::ComboFilter => "combo_filter",
            Self::ComboUpdates => "combo_options_update",
            Self::ComboScroll => "combo_scroll",
            Self::Ime => "ime_input",
            Self::Theme => "theme_change",
            Self::Dpi => "dpi_change",
            Self::Lifecycle => "object_lifecycle",
            Self::Text => "text_dynamic",
            Self::WrappedText => "text_wrapped",
            Self::TextWeights => "text_weights",
            Self::Shapes => "vector_shapes",
            Self::Blur0 => "blur_0",
            Self::Blur8 => "blur_8",
            Self::Blur24 => "blur_24",
            Self::Blur64 => "blur_64",
            Self::BlurDynamic => "blur_radius_change",
            Self::BlurStack => "blur_stack",
            Self::ControlBlur => "control_blur",
            Self::ProtocolCached => "protocol_cached",
            Self::ProtocolGeometry => "protocol_geometry",
            Self::ProtocolTexture => "protocol_texture",
            Self::KeyboardButton => "keyboard_button",
            Self::KeyboardCheckbox => "keyboard_checkbox",
            Self::KeyboardSlider => "keyboard_slider",
            Self::KeyboardTab => "keyboard_tab",
            Self::ColorPickerClosed => "color_picker_closed",
            Self::ColorPickerInternal => "color_picker_internal_cached",
            Self::ColorPickerFloating => "color_picker_floating_cached",
            Self::ColorPickerPalette => "color_picker_palette_drag",
            Self::ColorPickerHue => "color_picker_hue_drag",
            Self::ColorPickerInput => "color_picker_rgb_hex_input",
            Self::ColorPickerDrag => "color_picker_internal_window_drag",
            Self::ColorPickerFloatingDrag => "color_picker_floating_window_drag",
            Self::ColorPickerParentDrag => "color_picker_parent_window_drag",
            Self::BlurWindowDrag => "blur_window_drag",
            Self::ColorPickerInternalBlurDrag => "color_picker_internal_blur_window_drag",
            Self::ColorPickerFloatingBlurDrag => "color_picker_floating_blur_window_drag",
        }
    }
    pub fn scrolling(self) -> bool {
        matches!(
            self,
            Self::ScrollSettings
                | Self::ScrollRows
                | Self::ScrollNested
                | Self::ScrollMiddle
                | Self::ScrollCached
        )
    }
    pub fn interactive(self) -> bool {
        matches!(self, Self::PanZoom(c) if c.interactive())
            || matches!(self, Self::Dock(c) if c.interactive())
            || matches!(self, Self::Access(c) if c.interactive())
            || matches!(self, Self::Dnd(c) if c.interactive())
            || matches!(self, Self::Split(c) if c.interactive())
            || matches!(self, Self::Disclosure(c) if c.interactive())
            || matches!(self, Self::ListBox(c) if c.interactive())
            || matches!(self, Self::Carousel(c) if c.interactive())
            || matches!(self, Self::Material(c) if c.interactive())
            || matches!(self, Self::Modal(c) if c.interactive())
            || matches!(self, Self::Popup(c) if c.interactive())
            || matches!(self, Self::TextArea(c) if c.interactive())
            || matches!(self, Self::Number(c) if c.interactive())
            || matches!(self, Self::Actions(c) if c.interactive())
            || matches!(self, Self::Keys(c) if c.interactive())
            || self.combo()
            || matches!(
                self,
                Self::Button
                    | Self::Followup
                    | Self::Checkbox
                    | Self::Slider
                    | Self::Hover
                    | Self::Press
                    | Self::Drag
                    | Self::Resize
                    | Self::Tabs
                    | Self::Disabled
                    | Self::Cancel
                    | Self::FocusLoss
                    | Self::KeyboardButton
                    | Self::KeyboardCheckbox
                    | Self::KeyboardSlider
                    | Self::KeyboardTab
                    | Self::ColorPickerInternal
                    | Self::ColorPickerFloating
                    | Self::ColorPickerPalette
                    | Self::ColorPickerHue
                    | Self::ColorPickerInput
                    | Self::ColorPickerDrag
                    | Self::ColorPickerFloatingDrag
                    | Self::ColorPickerParentDrag
                    | Self::BlurWindowDrag
                    | Self::ColorPickerInternalBlurDrag
                    | Self::ColorPickerFloatingBlurDrag
            )
    }
    pub fn combo(self) -> bool {
        matches!(
            self,
            Self::ComboClosed
                | Self::ComboOpen
                | Self::ComboToggle
                | Self::ComboKeys
                | Self::ComboFilter
                | Self::ComboUpdates
                | Self::ComboScroll
        )
    }
    pub(super) fn color_picker_probe(self) -> bool {
        matches!(
            self,
            Self::ColorPickerInternal
                | Self::ColorPickerFloating
                | Self::ColorPickerPalette
                | Self::ColorPickerHue
                | Self::ColorPickerInput
                | Self::ColorPickerDrag
                | Self::ColorPickerFloatingDrag
                | Self::ColorPickerParentDrag
                | Self::ColorPickerInternalBlurDrag
                | Self::ColorPickerFloatingBlurDrag
        )
    }
    pub(super) fn floating_picker(self) -> bool {
        matches!(
            self,
            Self::ColorPickerFloating
                | Self::ColorPickerFloatingDrag
                | Self::ColorPickerParentDrag
                | Self::ColorPickerFloatingBlurDrag
        )
    }
    pub fn protocol(self) -> bool {
        matches!(
            self,
            Self::ProtocolCached | Self::ProtocolGeometry | Self::ProtocolTexture
        )
    }
    pub fn radius(self, step: usize) -> f32 {
        match self {
            Self::Blur8 => 8.0,
            Self::Blur24 => 24.0,
            Self::Blur64 => 64.0,
            Self::BlurDynamic => {
                if step.is_multiple_of(2) {
                    8.0
                } else {
                    24.0
                }
            }
            _ => 0.0,
        }
    }
}

#[path = "cases/catalog.rs"]
mod catalog;
