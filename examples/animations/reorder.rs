use zaxis::{move_item, Button, Id, Reorder, Ui};

pub struct Items(pub Vec<u32>);
impl Default for Items {
    fn default() -> Self {
        Self((1..=6).collect())
    }
}

pub fn show(ui: &mut Ui<'_>, items: &mut Items) {
    let order = &mut items.0;
    ui.horizontal(|ui| {
        if ui.button("Reverse").clicked() {
            order.reverse();
        }
        if ui.button("Rotate").clicked() && !order.is_empty() {
            order.rotate_left(1);
        }
        if ui.button("Reset").clicked() {
            *order = (1..=6).collect();
        }
    });
    let mut moved = None;
    Reorder::new("reorder").show(ui, order.iter().map(Id::new), |rows| {
        for (index, number) in order.iter().enumerate() {
            rows.item(Id::new(number), |ui| {
                ui.horizontal(|ui| {
                    if ui.add(Button::new("Up").enabled(index > 0)).clicked() {
                        moved = Some((index, index - 1));
                    }
                    if ui
                        .add(Button::new("Down").enabled(index + 1 < order.len()))
                        .clicked()
                    {
                        moved = Some((index, index + 1));
                    }
                    ui.label(format!("Item {number}"));
                });
            });
        }
    });
    if let Some((from, to)) = moved {
        move_item(order, from, to);
    }
}
