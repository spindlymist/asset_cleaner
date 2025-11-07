use imgui::{MouseButton, Ui};

pub fn list_box<Label, Item, Size>(
    ui: &Ui,
    label: Label,
    current_index: &mut Option<usize>,
    list_items: &[Item],
    indices: &[usize],
    size: Size
) -> bool
where
    Label: AsRef<str>,
    Item: AsRef<str>,
    Size: Into<mint::Vector2<f32>>
{
    let mut double_click = false;
    let mut found_index = false;
    
    ui.child_window(label)
        .size(size)
        .menu_bar(false)
        .movable(false)
        .scrollable(true)
        .build(|| {
            for item_idx in indices {
                let item = &list_items[*item_idx];
                let is_selected = *current_index == Some(*item_idx);
                if ui.selectable_config(item)
                    .allow_double_click(true)
                    .selected(is_selected)
                    .build()
                {
                    if ui.is_mouse_double_clicked(MouseButton::Left) {
                        double_click = true;
                    }
                    *current_index = Some(*item_idx);
                    found_index = true;
                }
                found_index |= is_selected;
            }
        });
        
    if !found_index {
        *current_index = None;
    }
    
    double_click
}

pub fn checklist_box<Label, Size>(
    ui: &Ui,
    label: Label,
    current_index: &mut Option<usize>,
    items: &mut [impl ChecklistItem],
    size: Size
) -> bool
where
    Label: AsRef<str>,
    Size: Into<mint::Vector2<f32>>,
{
    let mut double_click = false;
    if current_index.is_some_and(|i| i >= items.len()) {
        *current_index = None;
    }
    
    ui.child_window(label)
        .size(size)
        .menu_bar(false)
        .movable(false)
        .scrollable(true)
        .build(|| {
            let _frame_padding = ui.push_style_var(imgui::StyleVar::FramePadding([0.0, 0.0]));
            
            let [content_width, _] = ui.content_region_avail();
            let [inner_spacing, _] = unsafe { ui.style().item_inner_spacing };
            let frame_height = ui.frame_height();
            let selectable_x = frame_height + inner_spacing + 1.0;
            let selectable_width = content_width - selectable_x;
            let selectable_height = ui.text_line_height();
            
            for (i, item) in items.iter_mut().enumerate() {     
                let (label, checked) = item.label_and_checked();
                          
                let cursor_pos_initial = ui.cursor_pos();
                ui.set_cursor_pos([cursor_pos_initial[0] + selectable_x, cursor_pos_initial[1]]);
                if ui.selectable_config(format!("##{}", label))
                    .size([selectable_width, selectable_height])
                    .allow_double_click(true)
                    .selected(*current_index == Some(i))
                    .build()
                {
                    if ui.is_mouse_double_clicked(MouseButton::Left) {
                        double_click = true;
                    }
                    *current_index = Some(i);
                }
                
                ui.set_cursor_pos(cursor_pos_initial);
                ui.checkbox(label, checked);
            }
        });
    
    double_click
}

pub trait ChecklistItem {
    fn label_and_checked(&mut self) -> (&str, &mut bool);
}
