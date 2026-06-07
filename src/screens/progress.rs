use std::{path::PathBuf, thread, sync::mpsc};

use anyhow::anyhow;
use imgui_app::imgui;
use imgui::Ui;

use crate::{SharedState, Task, analysis::{AnalysisStatus, UnusedAssets, list_unused_assets}, widgets::error_display};

pub struct State {
    pub path: PathBuf,
    pub status: AnalysisStatus,
    pub thread: Option<thread::JoinHandle<anyhow::Result<UnusedAssets>>>,
    pub rx: mpsc::Receiver<AnalysisStatus>,
    pub error: Option<anyhow::Error>,
}

impl State {
    pub fn new(path: PathBuf) -> Self {
        let (tx, rx) = mpsc::channel();
        let thread = {
            let path = path.clone();
            thread::spawn(move || list_unused_assets(tx, path))
        };
        
        Self {
            path,
            thread: Some(thread),
            rx,
            status: AnalysisStatus::EnumeratingFiles,
            error: None,
        }
    }
}

pub fn build_ui(_shared: &mut SharedState, state: &mut State, ui: &Ui) -> Task {
    let mut task = Task::None;
    
    while let Ok(new_status) = state.rx.try_recv() {
        state.status = new_status;
    }
    
    if state.thread
        .as_ref()
        .is_some_and(|thread| thread.is_finished())
    {
        let thread = state.thread.take().unwrap();
        match thread.join() {
            Ok(Ok(assets)) => {
                state.status = AnalysisStatus::Finished;
                task = Task::ShowDetails(state.path.clone(), assets);
            }
            Ok(Err(err)) => {
                state.status = AnalysisStatus::Aborted;
                state.error = Some(err);
            }
            Err(_) => {
                state.status = AnalysisStatus::Aborted;
                state.error = Some(anyhow!("The analysis thread panicked. :("));
            }
        }
    }
    
    match state.status {
        AnalysisStatus::EnumeratingFiles => {
            ui.text("Enumerating files...");
        }
        AnalysisStatus::LoadingMap => {
            ui.text("Loading Map.bin...");
        }
        AnalysisStatus::LoadingIni => {
            ui.text("Loading World.ini...");
        }
        AnalysisStatus::AnalyzingMap => {
            ui.text("Analyzing Map.bin...");
        }
        AnalysisStatus::CheckingCoUsage => {
            ui.text("Checking custom object usage...");
        }
        AnalysisStatus::AnalyzingIni => {
            ui.text("Analyzing World.ini...");
        }
        AnalysisStatus::Aborted => {
            if let Some(err) = state.error.as_ref() {
                ui.text(format!("An error occurred while processing the level:"));
                error_display(ui, err);
            }
            else {
                ui.text("An unknown error occurred while processing the level.");
            }
            if ui.button("Return to level select") {
                task = Task::SelectLevel;
            }
        }
        AnalysisStatus::Finished => {
            ui.text("Finished.");
        }
    }
    
    task
}
