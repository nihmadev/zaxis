use super::*;
impl Scene {
    /// Assertions run outside measured intervals; every interaction must actually work.
    pub fn verify(&self) {
        if let Some(probe) = &self.dnd {
            probe.verify(&self.context);
            return;
        }
        if let Some(probe) = &self.disclosure {
            probe.verify(&self.context);
            return;
        }
        if let Some(probe) = &self.modal {
            probe.verify(&self.context);
            return;
        }
        if let Some(probe) = &self.text_area {
            probe.verify(&self.context);
            return;
        }
        if let Some(split) = &self.split {
            split.verify(&self.context);
            return;
        }
        if let Some(images) = &self.images {
            images.verify(&self.context);
            return;
        }
        if let Some(combo) = &self.combo {
            combo.verify(&self.context, self.case, self.step);
            return;
        }
        if self.case.scrolling() {
            self.scroll.verify(&self.context, self.case);
        }
        assert!(
            self.input_verified,
            "input was not delivered or repaint scheduling failed"
        );
        let forward = self.step.is_multiple_of(2);
        match self.case {
            Case::Button | Case::Followup | Case::KeyboardButton => {
                assert_eq!(
                    self.clicks,
                    self.previous_clicks + 1,
                    "button did not react"
                );
                if self.case == Case::Followup {
                    assert_eq!(
                        self.model_label,
                        format!(
                            "Model: {} / {} / {:.2}",
                            self.clicks, self.checked, self.value
                        )
                    );
                    assert!(
                        !self.context.needs_repaint(),
                        "follow-up model frame did not settle"
                    );
                }
            }
            Case::Checkbox | Case::KeyboardCheckbox => {
                assert_ne!(
                    self.checked, self.previous_checked,
                    "checkbox did not toggle"
                );
                assert!(self.targets.checkbox.unwrap().changed());
            }
            Case::Slider | Case::KeyboardSlider => {
                assert_eq!(
                    self.value,
                    if forward { 1.0 } else { 0.0 },
                    "slider lost capture outside its bounds"
                );
                assert!(self.targets.slider.unwrap().changed());
            }
            Case::Hover => assert_eq!(self.targets.button.unwrap().hovered, forward),
            Case::Press => {
                assert_eq!(self.targets.button.unwrap().pressed, forward);
                assert_eq!(self.targets.button.unwrap().clicked(), !forward);
            }
            Case::Drag
            | Case::ColorPickerDrag
            | Case::ColorPickerParentDrag
            | Case::BlurWindowDrag
            | Case::ColorPickerInternalBlurDrag => {
                let delta = if forward {
                    vec2(3.0, 2.0)
                } else {
                    vec2(-3.0, -2.0)
                };
                assert!(
                    (self.targets.button.unwrap().rect.min
                        - self.previous_button.unwrap().rect.min
                        - delta)
                        .length()
                        < 0.01,
                    "title-bar drag did not move children"
                );
                if matches!(
                    self.case,
                    Case::ColorPickerDrag | Case::ColorPickerInternalBlurDrag
                ) {
                    assert!(
                        (self.targets.palette.min - self.previous_palette.min - delta).length()
                            < 0.01,
                        "internal color picker did not move with its parent"
                    );
                } else if self.case == Case::ColorPickerParentDrag {
                    assert_eq!(
                        self.targets.floating, self.previous_floating,
                        "independent popup moved with its parent"
                    );
                }
                assert_eq!(
                    self.color, self.previous_color,
                    "window drag changed the color"
                );
            }
            Case::ColorPickerFloatingDrag | Case::ColorPickerFloatingBlurDrag => {
                let delta = if forward {
                    vec2(3.0, 2.0)
                } else {
                    vec2(-3.0, -2.0)
                };
                assert!(
                    (self.targets.floating.min - self.previous_floating.min - delta).length()
                        < 0.01,
                    "floating color picker did not move"
                );
                assert!(
                    (self.targets.palette.min - self.previous_palette.min - delta).length() < 0.01
                );
                assert_eq!(self.targets.window, self.previous_window);
                assert_eq!(self.color, self.previous_color);
            }
            Case::ColorPickerPalette | Case::ColorPickerHue | Case::ColorPickerInput => {
                let expected = match self.case {
                    Case::ColorPickerPalette if forward => Color::rgba(204, 41, 41, 73),
                    Case::ColorPickerPalette => Color::rgba(51, 41, 41, 73),
                    Case::ColorPickerHue if forward => Color::rgba(204, 0, 255, 73),
                    Case::ColorPickerHue => Color::rgba(204, 255, 0, 73),
                    Case::ColorPickerInput if forward => Color::rgba(51, 102, 153, 73),
                    _ => Color::rgba(222, 102, 153, 73),
                };
                assert_eq!(
                    self.color, expected,
                    "color picker input did not update its bound color"
                );
                assert!(self.targets.color_picker.unwrap().changed());
            }
            Case::Resize => {
                let delta = if forward {
                    vec2(3.0, 2.0)
                } else {
                    vec2(-3.0, -2.0)
                };
                assert!(
                    (self.targets.window.size() - self.previous_window.size() - delta).length()
                        < 0.01,
                    "resize grip did not resize window"
                );
            }
            Case::Tabs => {
                assert_ne!(self.selected, self.previous_selected);
                assert!(self.targets.tabs[self.selected].clicked());
            }
            Case::KeyboardTab => assert!(self.targets.checkbox.unwrap().has_focus),
            Case::Disabled => {
                let r = self.targets.disabled.unwrap();
                assert!(!r.clicked() && !r.pressed && !r.has_focus);
                assert_eq!(self.clicks, self.previous_clicks);
            }
            Case::Cancel | Case::FocusLoss => {
                assert_eq!(self.clicks, self.previous_clicks);
                assert!(
                    !self.targets.button.unwrap().pressed && !self.context.input().primary_down
                );
                if self.case == Case::FocusLoss {
                    assert!(!self.context.input().focused);
                }
            }
            Case::Cached
            | Case::Repaint
            | Case::Blur0
            | Case::Blur8
            | Case::Blur24
            | Case::Blur64
            | Case::ControlBlur
            | Case::BlurStack
            | Case::ColorPickerClosed
            | Case::ColorPickerInternal
            | Case::ColorPickerFloating => {
                assert_eq!(
                    self.draw_data().revision,
                    self.previous_revision,
                    "static geometry changed"
                );
                assert_eq!(
                    self.context.cache_stats().tessellated_elements,
                    self.previous_tessellations
                );
            }
            Case::BlurDynamic => {
                assert_eq!(
                    self.context.cache_stats().tessellated_elements,
                    self.previous_tessellations,
                    "changing blur sigma retessellated geometry"
                );
                if self.step > 0 {
                    assert_ne!(self.draw_data().revision, self.previous_revision);
                }
            }
            Case::Dynamic
            | Case::Theme
            | Case::Dpi
            | Case::Lifecycle
            | Case::Text
            | Case::WrappedText
            | Case::Shapes => {
                if self.step > 0 {
                    assert!(
                        self.draw_data().revision > self.previous_revision,
                        "changing scene reused stale geometry"
                    );
                }
            }
            Case::TextWeights => {
                if self.step > 0 {
                    assert!(
                        self.draw_data().revision > self.previous_revision,
                        "changing scene reused stale geometry"
                    );
                }
                // Four weights of a short caption fit the atlas without growing it.
                assert!(
                    self.draw_data().textures.len() <= 2,
                    "weights multiplied atlas pages"
                );
            }
            Case::Schedule => {
                // A long GPU present can cross the deadline after Context::run.
                // An expired deadline may also have been consumed at pass start.
                let deadline = self.context.next_repaint();
                let before = Instant::now();
                let repaint = self.context.needs_repaint();
                let after = Instant::now();
                match deadline {
                    Some(time) if time <= before => assert!(repaint),
                    Some(time) if time > after => assert!(!repaint),
                    None => assert!(!repaint),
                    _ => {} // Deadline crossed during the observation itself.
                }
            }
            Case::Wheel => assert_eq!(self.context.input().scroll_delta, Vec2::ZERO),
            Case::Ime => assert!(self.context.input().text.is_empty()),
            _ => {}
        }
        if !self.case.protocol() {
            assert!(!self.draw_data().vertices.is_empty());
            assert_eq!(self.draw_data().scale_factor, self.context.scale_factor());
        }
        if self.case.interactive() && !matches!(self.case, Case::Press | Case::Hover) {
            assert!(!self.context.input().primary_down);
        }
    }
}
