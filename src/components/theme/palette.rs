//! Palette-only interpolation. Layout and font metrics always use the target.
use super::*;
use crate::*;
trait PaletteBlend: Clone {
    fn blend(&self, to: &Self, t: f32) -> Self;
}
impl PaletteBlend for Color {
    fn blend(&self, to: &Self, t: f32) -> Self {
        self.interpolate(to, t)
    }
}
impl PaletteBlend for Gradient {
    fn blend(&self, to: &Self, t: f32) -> Self {
        self.interpolate(to, t)
    }
}
impl PaletteBlend for Border {
    fn blend(&self, to: &Self, t: f32) -> Self {
        Self {
            color: self.color.interpolate(&to.color, t),
            ..*to
        }
    }
}
impl PaletteBlend for Shadow {
    fn blend(&self, to: &Self, t: f32) -> Self {
        Self {
            color: self.color.interpolate(&to.color, t),
            ..*to
        }
    }
}
impl<T: PaletteBlend> PaletteBlend for Option<T> {
    fn blend(&self, to: &Self, t: f32) -> Self {
        match (self, to) {
            (Some(a), Some(b)) => Some(a.blend(b, t)),
            _ => to.clone(),
        }
    }
}
impl PaletteBlend for HoverFill {
    fn blend(&self, to: &Self, t: f32) -> Self {
        match (self, to) {
            (Self::Solid(a), Self::Solid(b)) => Self::Solid(a.interpolate(b, t)),
            (Self::Gradient(a), Self::Gradient(b)) => Self::Gradient(a.interpolate(b, t)),
            _ => *to,
        }
    }
}
macro_rules! blend {
 ($ty:ty; $($field:ident),* )=>{impl PaletteBlend for $ty {
 fn blend(&self,to:&Self,t:f32)->Self {let mut v=to.clone(); $(v.$field=self.$field.blend(&to.$field,t);)* v}
 }};
}
blend!(Style; accent,on_accent,disabled_text,selected_fill,selected_text,success,on_success,warning,on_warning,error,on_error,elevation,button,checkbox,switch,slider,text_edit,window,title_bar,popup,modal,card,color_picker,text,separator,loader,progress,split,number,grid,table,combo_box,scroll,drag,background,window_fill,title_fill,text_color,muted_text,border,button_fill,button_hovered,hover_style,button_pressed,text_edit_fill,text_edit_hovered,text_edit_placeholder,text_edit_selection,focus_border);
blend!(SurfaceStyle; fill,foreground,border,shadow);
blend!(ControlStyle; idle,hover,pressed,disabled,selected,focus,success,warning,error);
blend!(ButtonStyle; surface);
blend!(CheckboxStyle; body,indicator);
blend!(SwitchStyle; track,thumb);
blend!(SliderStyle; track,fill,thumb);
blend!(TextEditStyle; placeholder,selection,surface,caret,selection_foreground);
blend!(WindowStyle; body,title);
blend!(TitleBarStyle; surface,controls);
blend!(PopupStyle; surface);
blend!(ModalStyle; overlay,surface);
blend!(CardStyle; surface);
blend!(ColorPickerStyle; body,field);
blend!(TextStyle; color,muted);
blend!(SeparatorStyle; color);
blend!(LoaderStyle; color);
blend!(ProgressStyle; track,fill);
blend!(NumberStyle; surface, fill,hovered,invalid);
blend!(GridStyle; surface, fill);
blend!(TableStyle; row,header, border,fill,header_fill,text_color,alternate_fill,selected_fill,hovered_fill,separator_color,scroll);
blend!(ComboBoxStyle; trigger,option, trigger_fill,popup_fill,text,muted_text,disabled_text,check_color,active_fill,border,hover_border,popup_border);
blend!(ScrollStyle; thumb, thumb_color,thumb_hovered,hint_color);
blend!(SplitStyle; container,panel,handle);
blend!(SplitSurface; fill,border,shadow);
blend!(SplitHandleStyle; idle,hover,pressed,focus);
blend!(HoverStyle; fill,shadow,border,text_color);
blend!(DragStyle; target_fill,target_border,reject_border,line_color);
impl Interpolate for Style {
    fn interpolate(&self, to: &Self, t: f32) -> Self {
        self.blend(to, t)
    }
}
