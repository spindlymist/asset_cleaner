use std::{fs, path::Path};

use imgui::{StyleVar, Ui, WindowFlags};

use crate::{SharedState, Task, imgui_ext::ImguiExt, widgets::{error_display, filtered_list_box}};

pub struct State {
    pub error: Option<anyhow::Error>,
    pub levels: Vec<LevelListItem>,
    pub filtered_indices: Vec<usize>,
    pub filter: String,
    pub selected_level_index: Option<usize>,
}

impl State {
    pub fn new(worlds: &Path) -> Self {
        let (levels, error) = match list_levels(worlds) {
            Ok(levels) => (levels, None),
            Err(err) => (Vec::new(), Some(err)),
        };
        let filtered_indices: Vec<_> = Vec::from_iter(0..levels.len());
        
        Self {
            error,
            levels,
            filtered_indices,
            filter: String::from(""),
            selected_level_index: Some(0),
        }
    }
}

#[derive(Debug)]
pub struct LevelListItem {
    name: String,
    search_name: String,
}

impl AsRef<str> for LevelListItem {
    fn as_ref(&self) -> &str {
        self.name.as_str()
    }
}

pub fn build_ui(shared: &mut SharedState, state: &mut State, ui: &Ui) -> Task {
    if let Some(err) = state.error.as_ref() {
        ui.text(format!("An error occurred while listing levels:"));
        error_display(ui, err);
        return Task::None;
    }
    
    let mut open_level = false;
    
    ui.set_cursor_pos([8.0, 8.0]);
    let width = ui.content_region_avail()[0] - unsafe { ui.style() }.item_spacing[0] - 8.0;
    
    
    ui.set_next_item_width(width * 0.833);
    if ui.input_text("##Level filter", &mut state.filter)
        .build()
    {
        state.filtered_indices = filter_levels(&state.levels, &state.filter.to_lowercase());
    }
    
    ui.same_line();
    open_level |= ui.button_with_size("Open", [-8.0, 0.0]);
    
    ui.move_cursor_down(4.0);
    let _list_padding = ui.push_style_var(StyleVar::WindowPadding([12.0, 0.0]));
    open_level |= filtered_list_box(ui,
        "##Level list",
        &mut state.selected_level_index,
        &state.levels,
        &state.filtered_indices,
        [-1.0, -1.0]);
    
    if open_level
        && let Some(index) = state.selected_level_index
    {
        let level = &state.levels[index];
        let path = shared.worlds_path.join(&level.name);
        return Task::OpenLevel(path);
    }
    
    Task::None
}

fn list_levels(path: &Path) -> anyhow::Result<Vec<LevelListItem>> {
    let mut levels = Vec::new();
    
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        
        let level_path = entry.path();
        let level_name = entry.file_name().to_string_lossy().to_string();
        
        let map_path = level_path.join("Map.bin");
        let ini_path = level_path.join("World.ini");
        if !fs::exists(&map_path)? || !fs::exists(&ini_path)? {
            continue;
        }
        
        let level = LevelListItem {
            search_name: level_name.to_lowercase(),
            name: level_name,
        };
        levels.push(level);
    }
    
    Ok(levels)
}

fn filter_levels(levels: &[LevelListItem], filter: &str) -> Vec<usize> {
    levels.iter()
        .enumerate()
        .filter(|(_, level)| level.search_name.contains(filter))
        .map(|(i, _)| i)
        .collect()
}
