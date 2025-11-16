use std::path::PathBuf;

use imgui::{Key, MouseButton, StyleColor, Ui, WindowFlags};

pub fn filtered_list_box<Label, Item, Size>(
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
    let mut show_selected = false;
    
    if ui.is_key_pressed(Key::DownArrow)
        && let Some(item_index) = current_index.as_mut()
        && let Some(list_index) = indices.iter()
            .position(|i| i == item_index)
        && let Some(new_index) = indices.get(list_index + 1)
    {
        *item_index = *new_index;
        show_selected = true;
    }
    else if ui.is_key_pressed(Key::UpArrow)
        && let Some(item_index) = current_index.as_mut()
        && let Some(list_index) = indices.iter()
            .position(|i| i == item_index)
        && list_index > 0
        && let Some(new_index) = indices.get(list_index - 1)
    {
        *item_index = *new_index;
        show_selected = true;
    }
    else if ui.is_key_pressed(Key::Home)
        && let Some(item_index) = current_index.as_mut()
        && let Some(new_index) = indices.first()
    {
        *item_index = *new_index;
        show_selected = true;
    }
    else if ui.is_key_pressed(Key::End)
        && let Some(item_index) = current_index.as_mut()
        && let Some(new_index) = indices.last()
    {
        *item_index = *new_index;
        show_selected = true;
    }
    
    let _window = ui.child_window(label)
        .size(size)
        .movable(false)
        .scrollable(true)
        .flags(WindowFlags::ALWAYS_USE_WINDOW_PADDING)
        .begin();
    
    for item_idx in indices {
        let item = &list_items[*item_idx];
        let is_selected = *current_index == Some(*item_idx);
        
        if is_selected && show_selected {
            ui.set_scroll_here_y();
        }
        
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
    let mut show_selected = false;
    
    if current_index.is_some_and(|i| i >= items.len()) {
        *current_index = None;
    }
    
    if ui.is_key_pressed(Key::DownArrow)
        && let Some(current_index) = current_index.as_mut()
        && *current_index < items.len() - 1
    {
        *current_index += 1;
        show_selected = true;
    }
    else if ui.is_key_pressed(Key::UpArrow)
        && let Some(current_index) = current_index.as_mut()
        && *current_index > 0
    {
        *current_index -= 1;
        show_selected = true;
    }
    else if ui.is_key_pressed(Key::Home)
        && let Some(current_index) = current_index.as_mut()
        && !items.is_empty()
    {
        *current_index = 0;
        show_selected = true;
    }
    else if ui.is_key_pressed(Key::End)
        && let Some(current_index) = current_index.as_mut()
        && !items.is_empty()
    {
        *current_index = items.len() - 1;
        show_selected = true;
    }
    
    let _window = ui.child_window(label)
        .size(size)
        .movable(false)
        .scrollable(true)
        .flags(WindowFlags::ALWAYS_USE_WINDOW_PADDING | WindowFlags::HORIZONTAL_SCROLLBAR)
        .begin();
    
    let _frame_padding = ui.push_style_var(imgui::StyleVar::FramePadding([0.0, 0.0]));
    
    let max_item_width = items.iter_mut()
        .map(|item| {
            ui.calc_text_size(item.label())[0]
        })
        .fold(f32::NEG_INFINITY, f32::max);
    
    let content_width = ui.content_region_avail()[0];
    let inner_spacing_x = unsafe { ui.style().item_inner_spacing[0] };
    let frame_height = ui.frame_height();
    let selectable_x = frame_height + inner_spacing_x + 2.0;
    let selectable_width = f32::max(max_item_width, content_width - selectable_x);
    let selectable_height = ui.text_line_height();
    
    for (i, item) in items.iter_mut().enumerate() {
        let (label, checked) = item.label_and_is_checked_mut();
        let is_selected = *current_index == Some(i);
        
        if is_selected && show_selected {
            ui.set_scroll_here_y();
        }

        let cursor_pos_initial = ui.cursor_pos();
        ui.set_cursor_pos([cursor_pos_initial[0] + selectable_x, cursor_pos_initial[1]]);
        if ui.selectable_config(format!("##{}", label))
            .size([selectable_width, selectable_height])
            .allow_double_click(true)
            .selected(is_selected)
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
    
    double_click
}

pub trait ChecklistItem {
    fn label(&self) -> &str;
    fn label_and_is_checked_mut(&mut self) -> (&str, &mut bool);
}

pub fn error_display(ui: &Ui, err: &anyhow::Error) {
    ui.bullet();
    ui.text_wrapped(format!("{err}"));
    for cause in err.chain().skip(1) {
        ui.bullet();
        ui.text_wrapped(format!("caused by: {cause}"));
    }
}

pub fn progress_button_with_size<Label>(ui: &Ui, label: Label, size: [f32; 2], progress: f32) -> bool
where
    Label: AsRef<str>,
{
    let top_left = ui.cursor_screen_pos();
    let button_bottom_right = [
        top_left[0] + size[0],
        top_left[1] + size[1]
    ];
    let progress_bottom_right = [
        top_left[0] + progress * size[0],
        top_left[1] + size[1]
    ];

    let is_button_hovered = ui.is_mouse_hovering_rect(top_left, button_bottom_right);
    let is_button_active = is_button_hovered && ui.is_mouse_down(MouseButton::Left);

    let style_color = 
        if is_button_active {
            StyleColor::ButtonActive
        }
        else if is_button_hovered {
            StyleColor::ButtonHovered
        }
        else {
            StyleColor::Button
        };
    let color_progress = ui.style_color(style_color);
    let color_button = [
        color_progress[0],
        color_progress[1],
        color_progress[2],
        color_progress[3] * 0.5
    ];

    ui.get_window_draw_list()
        .add_rect(top_left, progress_bottom_right, color_progress)
        .filled(true)
        .rounding(2.0)
        .build();

    let _button_color = ui.push_style_color(style_color, color_button);
    ui.button_with_size(label, size)
}

pub struct FileSelection {
    pub path: PathBuf,
    pub display: String,
}

pub fn file_select(ui: &Ui, selection: &mut Option<FileSelection>) -> bool {
    let mut did_selection_change = false;
    
    if ui.button("Browse") {
        let dialog = rfd::FileDialog::new();
        if let Some(mut path) = dialog.pick_file() {
            path.pop();
            selection.replace(FileSelection {
                display: path.to_string_lossy().into_owned(),
                path,
            });
            did_selection_change = true;
        }
    }
    
    ui.same_line();
    match selection.as_ref() {
        Some(FileSelection { display, .. }) => {
            ui.text(display);
        }
        None => {
            ui.text("Please select a file.");
        }
    }
    
    did_selection_change
}
