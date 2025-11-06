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

use std::{error::Error, path::{Path, PathBuf}};

use anyhow::Result;
use imgui::{Condition, StyleColor, StyleVar, Ui};
use sdl2::video::Window;

use support::{platform_init, imgui_init, renderer_init, run, Textures};
use screens::*;
use analysis::UnusedAssets;

struct App {
    shared: SharedState,
    screen: Screen,
}

struct SharedState {
    worlds_path: PathBuf,
}

enum Screen {
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
    
    let mut app = app_init()?;
    run(imgui, |ui, window, textures| build_ui(&mut app, ui, window, textures));

    Ok(())
}

fn app_init() -> Result<App> {
    let Some(ks_path) = try_find_ks()? else {
        anyhow::bail!("KS not found");
    };
    let worlds_path = ks_path.join("Worlds");
    
    let screen_state = level_select::State::new(&worlds_path);
    let screen = Screen::LevelSelect(screen_state);
    let shared = SharedState {
        worlds_path,
    };
    
    Ok(App {
        shared,
        screen,
    })
}

fn build_ui(app: &mut App, ui: &Ui, window: &Window, textures: &mut Textures) {
    let (width, height) = window.size();
    let _style_window_padding = ui.push_style_var(StyleVar::WindowPadding([0.0, 0.0]));
    
    let mut task = Task::None;
    ui.window("Main")
        .position([0.0, 0.0], Condition::Always)
        .size([width as f32, height as f32], Condition::Always)
        .title_bar(false)
        .movable(false)
        .resizable(false)
        .build(|| {
            task = match &mut app.screen {
                Screen::LevelSelect(state) => {
                    level_select::build_ui(&mut app.shared, state, ui)
                }
                Screen::Progress(state) => {
                    progress::build_ui(&mut app.shared, state, ui)
                }
                Screen::LevelDetails(state) => {
                    level_details::build_ui(&mut app.shared, state, ui, textures)
                }
            }
        });
    
    match task {
        Task::SelectLevel => {
            let ks_path = try_find_ks().unwrap().unwrap(); // FIXME
            let worlds_path = ks_path.join("Worlds");
            let screen_state = level_select::State::new(&worlds_path);
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
    
    let mut maybe_ks_dir = std::env::current_dir()?;
    
    while !is_ks_dir(&maybe_ks_dir) {
        if !maybe_ks_dir.pop() {
            // Reached root without finding KS
            return Ok(None);
        }
    }
    
    return Ok(Some(maybe_ks_dir.to_owned()));
}
