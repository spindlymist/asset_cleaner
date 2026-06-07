#![windows_subsystem = "windows"]

mod widgets;
mod analysis;
mod config;
mod screens {
    pub mod startup;
    pub mod progress;
    pub mod level_details;
    pub mod level_select;
}
mod convert;

use std::path::PathBuf;

use anyhow::Result;
use imgui_app::{imgui, platform_init, imgui_init, renderer_init, run};
use imgui::{Condition, StyleVar, Ui};

use screens::*;
use analysis::UnusedAssets;

use crate::config::AppConfig;

struct App {
    shared: SharedState,
    screen: Screen,
}

struct SharedState {
    config: AppConfig,
    worlds_path: PathBuf,
}

enum Screen {
    Startup(startup::State),
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

    let platform = platform_init("Asset Cleaner", (1024, 768))?;
    let renderer = renderer_init(&platform.window, platform.window.size())?;
    let imgui = imgui_init(platform, renderer);
    
    let mut app = init_app();
    run(imgui, |ui, ex| {
        let (width, height) = ex.window.size();
        let _style_window_padding = ui.push_style_var(StyleVar::WindowPadding([0.0, 0.0]));
        ui.window("Main")
            .position([0.0, 0.0], Condition::Always)
            .size([width as f32, height as f32], Condition::Always)
            .title_bar(false)
            .movable(false)
            .resizable(false)
            .build(|| build_ui(&mut app, ui, ex));
    });

    Ok(())
}

fn init_app() -> App {
    let shared: SharedState;
    let screen: Screen;
    
    let config = config::load_config().unwrap_or_default();
    if let Some(ref ks_path) = config.ks_path
        && libks::editions::is_ks_dir(ks_path)
    {
        let worlds_path = ks_path.join("Worlds");
        shared = SharedState {
            config,
            worlds_path,
        };
        let screen_state = level_select::State::new(&shared.worlds_path);
        screen = Screen::LevelSelect(screen_state);
    }
    else {
        match startup::try_find_ks() {
            Ok(Some(ks_path)) => {
                shared = SharedState {
                    config,
                    worlds_path: ks_path.join("Worlds"),
                };
                let screen_state = level_select::State::new(&shared.worlds_path);
                screen = Screen::LevelSelect(screen_state);
            }
            Ok(None) => {
                shared = SharedState {
                    config,
                    worlds_path: PathBuf::new(),
                };
                let screen_state = startup::State::new(None);
                screen = Screen::Startup(screen_state);
            }
            Err(err) => {
                shared = SharedState {
                    config,
                    worlds_path: PathBuf::new(),
                };
                let screen_state = startup::State::new(Some(err));
                screen = Screen::Startup(screen_state);
            }
        }
    }
    
    App {
        shared,
        screen,
    }
}

fn build_ui(app: &mut App, ui: &Ui, ex: imgui_app::Extras) {
    let _rounding = ui.push_style_var(StyleVar::FrameRounding(2.0));
    
    let task = match &mut app.screen {
        Screen::Startup(state) => {
            startup::build_ui(&mut app.shared, state, ui)
        }
        Screen::LevelSelect(state) => {
            level_select::build_ui(&mut app.shared, state, ui)
        }
        Screen::Progress(state) => {
            progress::build_ui(&mut app.shared, state, ui)
        }
        Screen::LevelDetails(state) => {
            level_details::build_ui(&mut app.shared, state, ui, ex.textures, ex.fonts)
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
            let screen_state = level_details::State::new(path, unused_assets, ex.textures);
            app.screen = Screen::LevelDetails(screen_state);
        }
        Task::None => {}
    }
}
