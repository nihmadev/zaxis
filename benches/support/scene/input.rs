use super::*;
impl Scene {
    /// Inject native-format events through the same public dispatcher as a host.
    pub fn input(&mut self, step: usize) {
        self.step = step;
        self.previous_button = self.targets.button;
        self.previous_window = self.targets.window;
        self.previous_palette = self.targets.palette;
        self.previous_floating = self.targets.floating;
        self.previous_color = self.color;
        self.previous_checked = self.checked;
        self.previous_selected = self.selected;
        self.previous_clicks = self.clicks;
        self.previous_revision = self.draw_data().revision;
        self.previous_tessellations = self.context.cache_stats().tessellated_elements;
        self.input_verified = true;
        if let Some(probe) = &mut self.dnd {
            probe.input(&mut self.context, step);
            return;
        }
        if let Some(probe) = &mut self.access {
            probe.input(&mut self.context, step);
            return;
        }
        if let Some(probe) = &mut self.carousel {
            probe.input(&mut self.context, step);
            return;
        }
        if let Some(probe) = &mut self.list_box {
            probe.input(&mut self.context, step);
            return;
        }
        if let Some(probe) = &mut self.disclosure {
            probe.input(&mut self.context, step);
            return;
        }
        if let Some(probe) = &mut self.modal {
            probe.input(&mut self.context, step);
            return;
        }
        if let Some(probe) = &mut self.text_area {
            probe.input(&mut self.context, step);
            return;
        }
        if let Some(probe) = &mut self.number {
            probe.input(&mut self.context, step);
            return;
        }
        if let Some(split) = &mut self.split {
            split.input(&mut self.context, step);
            return;
        }
        if let Some(images) = &mut self.images {
            images.input(&mut self.context, step);
            return;
        }
        if let Some(combo) = &mut self.combo {
            combo.input(&mut self.context, self.case, step);
            return;
        }
        if self.case.scrolling() {
            self.scroll.input(&mut self.context, self.case, step);
            return;
        }
        let forward = step.is_multiple_of(2);
        match self.case {
            Case::Cold => {
                self.context = Context::new();
                configure(&mut self.context, self.size, self.scale);
            }
            Case::Repaint => self.context.request_repaint(),
            Case::Schedule => {
                self.context.request_repaint_after(Duration::from_secs(10));
                let first = self.context.next_repaint().unwrap();
                self.context.request_repaint_after(Duration::from_secs(20));
                self.input_verified &= self.context.next_repaint() == Some(first);
                self.context.request_repaint_after(Duration::from_secs(5));
                self.input_verified &= self.context.next_repaint().unwrap() <= first;
            }
            Case::Button | Case::Followup | Case::Checkbox | Case::Disabled | Case::Tabs => {
                let response = match self.case {
                    Case::Button | Case::Followup => self.targets.button.unwrap(),
                    Case::Checkbox => self.targets.checkbox.unwrap(),
                    Case::Disabled => self.targets.disabled.unwrap(),
                    _ => self.targets.tabs[1 - self.selected],
                };
                self.move_to(response.rect.center());
                self.mouse(ElementState::Pressed);
                self.mouse(ElementState::Released);
            }
            Case::Hover => {
                let target = if forward {
                    self.targets.button.unwrap().rect.center()
                } else {
                    vec2(1.0, 1.0)
                };
                self.move_to(target);
            }
            Case::Press => {
                self.move_to(self.targets.button.unwrap().rect.center());
                self.mouse(if forward {
                    ElementState::Pressed
                } else {
                    ElementState::Released
                });
            }
            Case::Slider => {
                let rect = self.targets.slider.unwrap().rect;
                self.move_to(rect.center());
                self.mouse(ElementState::Pressed);
                self.move_to(vec2(
                    if forward {
                        rect.max.x + 30.0
                    } else {
                        rect.min.x - 30.0
                    },
                    rect.center().y,
                ));
                self.mouse(ElementState::Released);
            }
            Case::Drag
            | Case::Resize
            | Case::ColorPickerDrag
            | Case::ColorPickerFloatingDrag
            | Case::ColorPickerParentDrag
            | Case::BlurWindowDrag
            | Case::ColorPickerInternalBlurDrag
            | Case::ColorPickerFloatingBlurDrag => {
                let rect = if matches!(
                    self.case,
                    Case::ColorPickerFloatingDrag | Case::ColorPickerFloatingBlurDrag
                ) {
                    self.targets.floating
                } else {
                    self.targets.window
                };
                let point = if self.case != Case::Resize {
                    rect.min + vec2(30.0, 12.0)
                } else {
                    rect.max - vec2(5.0, 5.0)
                };
                self.move_to(point);
                self.mouse(ElementState::Pressed);
                self.move_to(
                    point
                        + if forward {
                            vec2(3.0, 2.0)
                        } else {
                            vec2(-3.0, -2.0)
                        },
                );
                self.mouse(ElementState::Released);
            }
            Case::ColorPickerPalette | Case::ColorPickerHue => {
                let rect = if self.case == Case::ColorPickerPalette {
                    self.targets.palette
                } else {
                    self.targets.hue
                };
                let t = if forward {
                    vec2(0.8, 0.2)
                } else {
                    vec2(0.2, 0.8)
                };
                self.move_to(rect.center());
                self.input_verified &= self.mouse(ElementState::Pressed);
                self.move_to(rect.min + rect.size() * t);
                self.input_verified &= self.mouse(ElementState::Released);
            }
            Case::ColorPickerInput => {
                let field = if forward { 3 } else { 0 };
                self.move_to(self.targets.fields[field].center());
                self.input_verified &= self.mouse(ElementState::Pressed);
                self.input_verified &= self.mouse(ElementState::Released);
                self.input_verified &= self
                    .context
                    .on_text_event(if forward { "#336699" } else { "222" })
                    .consumed;
                self.input_verified &= self
                    .context
                    .on_key_event(KeyCode::Enter, ElementState::Pressed, false)
                    .consumed;
                self.context
                    .on_key_event(KeyCode::Enter, ElementState::Released, false);
            }
            Case::Cancel | Case::FocusLoss => {
                self.move_to(self.targets.button.unwrap().rect.center());
                self.mouse(ElementState::Pressed);
                if self.case == Case::FocusLoss {
                    self.context.on_window_event(&WindowEvent::Focused(false));
                } else {
                    self.move_to(vec2(1.0, 1.0));
                }
                self.mouse(ElementState::Released);
            }
            Case::Wheel => {
                self.context.on_window_event(&WindowEvent::MouseWheel {
                    device_id: DeviceId::dummy(),
                    delta: MouseScrollDelta::LineDelta(0.0, 1.0),
                    phase: TouchPhase::Moved,
                });
                self.input_verified = self.context.input().scroll_delta.y > 0.0;
            }
            Case::Ime => {
                self.context
                    .on_window_event(&WindowEvent::Ime(Ime::Commit("Benchmark λ".into())));
                self.input_verified = self.context.input().text == "Benchmark λ";
            }
            Case::KeyboardButton
            | Case::KeyboardCheckbox
            | Case::KeyboardSlider
            | Case::KeyboardTab => {
                // A cancelled pointer press focuses the desired widget without activating it.
                let target = match self.case {
                    Case::KeyboardCheckbox => self.targets.checkbox.unwrap(),
                    Case::KeyboardSlider => self.targets.slider.unwrap(),
                    _ => self.targets.button.unwrap(),
                };
                self.move_to(target.rect.center());
                self.mouse(ElementState::Pressed);
                self.move_to(vec2(1.0, 1.0));
                self.mouse(ElementState::Released);
                let code = match self.case {
                    Case::KeyboardButton => {
                        if forward {
                            KeyCode::Enter
                        } else {
                            KeyCode::Space
                        }
                    }
                    Case::KeyboardCheckbox => KeyCode::Space,
                    Case::KeyboardSlider => {
                        if forward {
                            KeyCode::End
                        } else {
                            KeyCode::Home
                        }
                    }
                    _ => KeyCode::Tab,
                };
                self.input_verified = self
                    .context
                    .on_key_event(code, ElementState::Pressed, false)
                    .consumed;
                self.context
                    .on_key_event(code, ElementState::Released, false);
            }
            Case::Theme => {
                let mut style = self.context.style().clone();
                style.button_fill = if forward {
                    Color::rgb(64, 76, 92)
                } else {
                    Color::gray(66)
                };
                self.context.set_style(style);
            }
            Case::Dpi => self.context.set_viewport(
                self.size,
                if forward {
                    self.scale * 2.0
                } else {
                    self.scale
                },
            ),
            _ => {}
        }
    }
}
