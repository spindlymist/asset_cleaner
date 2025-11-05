use std::{fs, path::Path};

use imgui::Ui;

use crate::{SharedState, Task, widgets::list_box};

pub struct State {
    pub error: Option<anyhow::Error>,
    pub levels: Vec<LevelListItem>,
    pub filter: String,
    pub selected_level_index: Option<usize>,
}

impl State {
    pub fn new(worlds: &Path) -> Self {
        let (levels, error) = match list_levels(&worlds) {
            Ok(levels) => (levels, None),
            Err(err) => (Vec::new(), Some(err.into())),
        };
        
        Self {
            error,
            levels,
            selected_level_index: None,
            filter: String::from(""),
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
    if let Some(error) = state.error.as_ref() {
        ui.text(format!("Failed to list levels: {error}"));
        return Task::None;
    }
    
    let mut open_level;
    
    let width = {
        let [width, _] = ui.content_region_avail();
        width - 14.0
    };
    
    ui.set_next_item_width(width * 0.833);
    ui.input_text("##Level filter", &mut state.filter)
        .build();
    
    ui.same_line();
    open_level = ui.button_with_size("Open", [-1.0, 0.0]);
    
    let list_items: Vec<_> = filter_levels(&state.levels, &state.filter.to_lowercase());
    open_level |= list_box(ui, "##Level list", &mut state.selected_level_index, &list_items, [-1.0, -1.0]);
    
    if open_level {
        if let Some(index) = state.selected_level_index {
            let level = list_items[index];
            let path = shared.worlds_path.join(&level.name);
            return Task::OpenLevel(path);
        }
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

fn filter_levels<'a, 'b>(levels: &'a Vec<LevelListItem>, filter: &'b str) -> Vec<&'a LevelListItem> {
    levels.iter()
        .by_ref()
        .filter(|level| level.search_name.contains(filter))
        .collect()
}
