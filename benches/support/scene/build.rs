use super::*;
impl Scene {
    pub fn build(&mut self) {
        self.build_once();
        if self.case == Case::Followup && self.context.needs_repaint() {
            self.build_once();
        }
    }

    fn build_once(&mut self) {
        if let Some(probe) = &mut self.dnd {
            probe.build(&mut self.context);
            return;
        }
        if let Some(probe) = &mut self.access {
            probe.build(&mut self.context);
            return;
        }
        if let Some(probe) = &mut self.carousel {
            probe.build(&mut self.context);
            return;
        }
        if let Some(probe) = &mut self.list_box {
            probe.build(&mut self.context);
            return;
        }
        if let Some(probe) = &mut self.disclosure {
            probe.build(&mut self.context);
            return;
        }
        if let Some(probe) = &mut self.modal {
            probe.build(&mut self.context);
            return;
        }
        if let Some(probe) = &mut self.text_area {
            probe.build(&mut self.context);
            return;
        }
        if let Some(split) = &mut self.split {
            split.build(&mut self.context);
            return;
        }
        if let Some(images) = &mut self.images {
            images.build(&mut self.context);
            return;
        }
        if let Some(combo) = &mut self.combo {
            combo.build(&mut self.context, self.case, self.step);
            return;
        }
        if self.case.scrolling() {
            self.scroll.build(
                &mut self.context,
                self.case,
                self.step,
                &self.labels,
                &mut self.checks,
                &mut self.values,
            );
            return;
        }
        if self.case.protocol() {
            match self.case {
                Case::ProtocolGeometry => {
                    self.protocol.vertices[0].position[0] = (self.step % 2) as f32;
                    self.protocol.revision += 1;
                }
                Case::ProtocolTexture => {
                    Arc::make_mut(&mut self.protocol.textures[0].pixels)[0] =
                        (self.step % 256) as u8;
                    self.protocol.textures[0].revision += 1;
                }
                _ => {}
            }
            black_box(&self.protocol);
            return;
        }
        let Self {
            context,
            case,
            count,
            step,
            targets,
            checked,
            clicks,
            value,
            selected,
            labels,
            checks,
            values,
            colors,
            color,
            model_label,
            ..
        } = self;
        *targets = Targets::default();
        context.run(|context| {
            let bounds = context.viewport();
            // Detailed backdrop makes blur cost and paint ordering representative.
            for i in 0..32 {
                let p = vec2((i % 8) as f32 * bounds.size().x / 8.0, (i / 8) as f32 * bounds.size().y / 4.0);
                context.paint_background(Shape::rect(Rect::from_min_size(p, bounds.size() / vec2(8.0, 4.0)),
                    Color::rgb(30 + (i * 7) as u8, 55, 90)).corner_radius(12.0));
            }
            let columns = ((*count as f32 * bounds.size().x / bounds.size().y).sqrt().ceil() as usize).clamp(1, 64);
            let rows = count.div_ceil(columns);
            let width = bounds.size().x / columns as f32;
            let visible_count = if *case == Case::Lifecycle && *step % 2 == 0 { *count / 2 } else { *count };
            for column in 0..columns {
                Window::new(format!("Load##{column}"))
                    .default_position(vec2(column as f32 * width, 0.0))
                    .default_size(vec2(width, bounds.size().y))
                    .min_size(vec2(64.0, 64.0)).padding(Padding::all(3.0))
                    .draggable(false).resizable(false).blur(case.radius(*step))
                    .show(context, |ui| {
                        for row in 0..rows {
                            let i = column * rows + row;
                            if i >= visible_count { break; }
                            ui.push_id(i, |ui| {
                                match case {
                                    Case::ColorPickerClosed => {
                                        ui.add(ColorPicker::new(&mut colors[i], &labels[i]).width(width - 6.0));
                                    }
                                    Case::Text | Case::WrappedText => {
                                        let caption = format!("Row {i}: frame {step:06} · text wrapping and glyph cache λ");
                                        ui.add(Text::new(caption).size(12.0).wrap(*case == Case::WrappedText));
                                    }
                                    Case::TextWeights => {
                                        // Four weights in one frame: four faces share the layout cache and atlas.
                                        let weight = [FontWeight::REGULAR, FontWeight::MEDIUM, FontWeight::SEMIBOLD, FontWeight::BOLD][i % 4];
                                        let caption = format!("Row {i}: frame {step:06} · text wrapping and glyph cache λ");
                                        ui.add(Text::new(caption).size(12.0).weight(weight).wrap(false));
                                    }
                                    Case::Shapes => {
                                        let rect = ui.allocate_space(vec2(width - 6.0, 24.0));
                                        let color = Color::rgb(70, 100 + (*step % 2) as u8 * 80, 160);
                                        match i % 3 {
                                            0 => ui.paint(Shape::rect(rect, color).corner_radius(CornerRadius {
                                                top_left: 2.0, top_right: 7.0, bottom_right: 12.0, bottom_left: 4.0,
                                            }).border(Border::new(1.0, Color::WHITE))),
                                            1 => ui.paint(Shape::Circle { center: rect.center(), radius: 10.0,
                                                fill: color, border: Border::new(2.0, Color::WHITE) }),
                                            _ => ui.paint(Shape::Line { start: rect.min, end: rect.max, width: 2.0, color }),
                                        }
                                    }
                                    _ => {
                                        if *case == Case::Dynamic {
                                            checks[i] = *step % 2 == 0;
                                            values[i] = (*step % 100) as f32 / 100.0;
                                        }
                                        let blur = if *case == Case::ControlBlur { 8.0 } else { 0.0 };
                                        match i % 4 {
                                            0 => {
                                                let caption = if *case == Case::Dynamic { format!("{i}:{step}") } else { labels[i].clone() };
                                                ui.add(Button::new(caption).blur(blur).selected(*case == Case::Dynamic && *step % 2 == 0));
                                            }
                                            1 => { ui.add(Checkbox::new(&mut checks[i], &labels[i]).size(14.0).blur(blur)); }
                                            2 => { ui.add(Slider::new(&mut values[i], 0.0..=1.0).step(0.01).status(SliderStatus::Success).blur(blur)); }
                                            _ => {
                                                let caption = if *case == Case::Dynamic { format!("{i}:{step}") } else { labels[i].clone() };
                                                ui.add(Text::new(caption).muted().wrap(false));
                                            }
                                        }
                                    }
                                }
                            });
                        }
                        ui.add(Separator::new().thickness(1.0).spacing(0.0).inset(2.0));
                    });
            }
            if *case == Case::BlurStack {
                Window::new("Layered glass").default_position(vec2(40.0, 40.0))
                    .default_size(bounds.size() - vec2(80.0, 80.0)).blur(8.0).show(context, |ui| {
                        let rect = ui.clip_rect();
                        for i in 0..8 {
                            Blur::new(rect.shrink(i as f32 * 4.0)).radius(8.0 + i as f32 * 4.0)
                                .corner_radius(12.0).tint(Color::rgba(80, 130, 180, 12)).show(ui);
                        }
                        ui.label("Eight overlapping filters with foreground text");
                    });
            }
            if case.interactive() {
                Window::new("Probe").default_position(vec2(32.0, 32.0))
                    .default_size(vec2(420.0, 360.0)).padding(Padding::all(8.0))
                    .show(context, |ui| {
                        // Content bounds include the resize reserve of 12 logical px.
                        let clip = ui.clip_rect();
                        targets.window = Rect::from_min_max(
                            clip.min - vec2(8.0, ui.style().title_height + 8.0),
                            clip.max + vec2(12.0, 12.0),
                        );
                        if case.color_picker_probe() {
                            let response = ui.add(ColorPicker::new(color, "Probe color")
                                .width(300.0).default_open(true)
                                .picker_type(if case.floating_picker() { ColorPickerType::Floating } else { ColorPickerType::Internal }));
                            targets.color_picker = Some(response);
                            targets.button = Some(ui.button("React"));
                            if !case.floating_picker() {
                                targets.palette = Rect::from_min_size(response.rect.min + vec2(0.0, 34.0), vec2(300.0, 112.0));
                            }
                            return;
                        }
                        *model_label = format!("Model: {clicks} / {checked} / {value:.2}");
                        ui.label(model_label.clone());
                        let button = ui.button("React");
                        if button.clicked() { *clicks += 1; }
                        targets.button = Some(button);
                        targets.checkbox = Some(ui.checkbox(checked, "Toggle"));
                        targets.slider = Some(ui.add(Slider::new(value, 0.0..=1.0).step(0.01)));
                        targets.tabs = ui.tab_bar(selected, [(0, "First"), (1, "Second")]);
                        targets.disabled = Some(ui.add_enabled_ui(false, |ui| {
                            ui.add_enabled_ui(true, |ui| ui.button("Disabled group"))
                        }));
                        ui.horizontal(|ui| {
                            ui.selectable(false, "Selection");
                            ui.add(Separator::vertical(20.0));
                            ui.muted("Nested row");
                        });
                    });
            }
        });
        if self.case.color_picker_probe() {
            if self.case.floating_picker() {
                // Identify the popup's content clip from public draw data, so the probe
                // observes its actual retained position after a title-bar drag.
                let clip = self
                    .context
                    .draw_data()
                    .commands
                    .iter()
                    .map(|command| command.clip_rect)
                    .find(|clip| (clip.size() - vec2(300.0, 176.0)).length() < 0.01)
                    .expect("open color picker popup was not rendered");
                self.targets.palette = Rect::from_min_size(clip.min, vec2(300.0, 112.0));
                self.targets.floating = Rect::from_min_max(
                    clip.min - vec2(12.0, self.context.style().title_height + 12.0),
                    clip.max + Vec2::splat(12.0),
                );
            }
            let origin = self.targets.palette.min;
            self.targets.hue = Rect::from_min_size(origin + vec2(0.0, 122.0), vec2(300.0, 14.0));
            self.targets.fields = std::array::from_fn(|field| {
                Rect::from_min_size(origin + vec2(field as f32 * 75.0, 150.0), vec2(70.0, 26.0))
            });
        }
        black_box(self.context.draw_data());
    }
}
