use std::path::PathBuf;

use anyhow::{Result, Context, Error};
use imgui_app::imgui;
use imgui::{StyleVar, Ui, WindowFlags};
use libks::editions::is_ks_dir;

use crate::{SharedState, Task, config, widgets::{FileSelection, error_display, file_select}};

pub struct State {
    error: Option<Error>,
    selected_path: Option<FileSelection>,
    is_ks_dir: bool,
}

impl State {
    pub fn new(error: Option<Error>) -> Self {
        Self {
            error,
            selected_path: None,
            is_ks_dir: false,
        }
    }
}

pub fn build_ui(shared: &mut SharedState, state: &mut State, ui: &Ui) -> Task {
    let avail_width = ui.content_region_avail()[0];
    let min_width = f32::min(avail_width, 600.0);
    let window_width = f32::max(avail_width * (2.0/3.0), min_width);
    let window_x = (avail_width - window_width) / 2.0;
    ui.set_cursor_pos([window_x, 0.0]);
    let _padding = ui.push_style_var(StyleVar::WindowPadding([12.0, 12.0]));
    let _window = ui.child_window("##Startup")
        .size([window_width, 0.0])
        .movable(false)
        .flags(WindowFlags::ALWAYS_USE_WINDOW_PADDING)
        .begin();
    
    match state.error {
        Some(ref err) => {
            error_display(ui, err);
        }
        None => {
            ui.text_wrapped("KS could not be located automatically. Please move this program into your KS directory \
                (or a subdirectory, such as 3rd Party Tools) or locate Knytt Stories.exe manually.");
            ui.new_line();
            
            {
                let _disabled = ui.begin_disabled(!state.is_ks_dir);
                if ui.button("Continue")
                    && let Some(FileSelection { path, .. }) = state.selected_path.take()
                {
                    shared.config.ks_path.replace(path.clone());
                    let _ = config::save_config(&shared.config);
                    shared.worlds_path = path.join("Worlds");
                    return Task::SelectLevel;
                }
            }
            
            ui.same_line();
            if file_select(ui, &mut state.selected_path)
                && let Some(FileSelection { path, .. }) = state.selected_path.as_ref()
            {
                state.is_ks_dir = is_ks_dir(&path);
            }
            
            ui.new_line();
            if state.selected_path.is_some() && !state.is_ks_dir {
                ui.text("Hey, that's not a KS directory! You can't fool me.");
            }
        }
    };
    
    Task::None
}

pub fn try_find_ks() -> Result<Option<PathBuf>> {
    let mut maybe_ks_dir = std::env::current_dir()
        .context("Failed to get current directory")?;
        
    while !is_ks_dir(&maybe_ks_dir) {
        if !maybe_ks_dir.pop() {
            // Reached root without finding KS
            return Ok(None);
        }
    }
    
    Ok(Some(maybe_ks_dir.to_owned()))
}
