//#![windows_subsystem = "windows"]

mod support;
mod widgets;
mod analysis;
mod screens {
    pub mod progress;
    pub mod level_details;
    pub mod level_select;
}
mod imgui_ext;
mod png_decoder;

use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use imgui::{Condition, StyleVar, Ui};
use sdl2::video::Window;

use support::{platform_init, imgui_init, renderer_init, run, Textures};
use screens::*;
use analysis::UnusedAssets;

use crate::widgets::error_display;

struct App {
    shared: SharedState,
    screen: Screen,
}

struct SharedState {
    worlds_path: PathBuf,
}

enum Screen {
    StartupFailed(anyhow::Error),
    LevelSelect(level_select::State),
    Progress(progress::State),
    LevelDetails(level_details::State),
}

enum Task {
    None,
    SelectLevel,
    OpenLevel(PathBuf),
    ShowDetails(PathBuf, UnusedAssets),
}

fn main() -> Result<()> {
    env_logger::init();

    let platform = platform_init("Title", (1024, 768))?;
    let renderer = renderer_init(&platform.window, platform.window.size())?;
    let imgui = imgui_init(platform, renderer);
    
    let mut app = init_app();
    run(imgui, |ui, window, textures| {
        let (width, height) = window.size();
        let _style_window_padding = ui.push_style_var(StyleVar::WindowPadding([0.0, 0.0]));
        ui.window("Main")
            .position([0.0, 0.0], Condition::Always)
            .size([width as f32, height as f32], Condition::Always)
            .title_bar(false)
            .movable(false)
            .resizable(false)
            .build(|| build_ui(&mut app, ui, window, textures));
    });

    Ok(())
}

fn init_app() -> App {
    let ks_path;
    let startup_error;
    match try_find_ks() {
        Ok(Some(val)) => {
            ks_path = val;
            startup_error = None;
        }
        Ok(None) => {
            ks_path = PathBuf::from(".");
            startup_error = Some(anyhow!(
                "This program must be placed in your KS directory, \
                or in a subdirectory such as 3rd Party Tools."));
        }
        Err(err) => {
            ks_path = PathBuf::from(".");
            startup_error = Some(err);
        }
    };

    let worlds_path = ks_path.join("Worlds");
    let screen = match startup_error {
        Some(err) => {
            Screen::StartupFailed(err)
        }
        None => {
            let screen_state = level_select::State::new(&worlds_path);
            Screen::LevelSelect(screen_state)
        }
    };
    let shared = SharedState {
        worlds_path,
    };
    
    App {
        shared,
        screen,
    }
}

fn build_ui(app: &mut App, ui: &Ui, _window: &Window, textures: &mut Textures) {
    let task = match &mut app.screen {
        Screen::StartupFailed(err) => {
            ui.text("Startup failed:");
            error_display(ui, err);
            Task::None
        }
        Screen::LevelSelect(state) => {
            level_select::build_ui(&mut app.shared, state, ui)
        }
        Screen::Progress(state) => {
            progress::build_ui(&mut app.shared, state, ui)
        }
        Screen::LevelDetails(state) => {
            level_details::build_ui(&mut app.shared, state, ui, textures)
        }
    };
    
    match task {
        Task::SelectLevel => {
            let screen_state = level_select::State::new(&app.shared.worlds_path);
            app.screen = Screen::LevelSelect(screen_state);
        }
        Task::OpenLevel(path) => {
            let screen_state = progress::State::new(path);
            app.screen = Screen::Progress(screen_state);
        }
        Task::ShowDetails(path, unused_assets) => {
            let screen_state = level_details::State::new(path, unused_assets, textures);
            app.screen = Screen::LevelDetails(screen_state);
        }
        Task::None => {}
    }
}

fn try_find_ks() -> Result<Option<PathBuf>> {
    use libks::editions::is_ks_dir;
    
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
