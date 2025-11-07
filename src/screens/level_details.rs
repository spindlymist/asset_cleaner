use std::{fs, path::{Path, PathBuf}};

use imgui::{MouseButton, StyleColor, StyleVar, Ui, WindowFlags};
use kira::{AudioManager, AudioManagerSettings, Tween, sound::{FromFileError, PlaybackState, streaming::{StreamingSoundData, StreamingSoundHandle}}};

use crate::{SharedState, Task, analysis::UnusedAssets, imgui_ext::ImguiExt, support::{Image, Textures}, widgets::{ChecklistItem, checklist_box}};

pub struct State {
    pub level_dir: PathBuf,
    pub level_name: String,
    pub level_icon: Option<Image>,
    pub tilesets: Vec<AssetListItem>,
    pub gradients: Vec<AssetListItem>,
    pub songs: Vec<AssetListItem>,
    pub ambiance: Vec<AssetListItem>,
    pub sounds: Vec<AssetListItem>,
    pub custom_objects: Vec<AssetListItem>,
    pub cutscenes: Vec<AssetListItem>,
    pub misc: Vec<AssetListItem>,
    pub ignored: Vec<AssetListItem>,
    pub selected_index: Option<usize>,
    pub prev_tab: Tab,
    pub audio: AudioState,
}

impl State {
    pub fn new(level_dir: PathBuf, unused_assets: UnusedAssets, textures: &mut Textures) -> Self {
        let mut tilesets: Vec<_> = unused_assets.tilesets.into_iter()
            .map(|path| AssetListItem::new(path, &level_dir, true))
            .collect();
        tilesets.sort_by(compare_list_items);
        let mut gradients: Vec<_> = unused_assets.gradients.into_iter()
            .map(|path| AssetListItem::new(path, &level_dir, true))
            .collect();
        gradients.sort_by(compare_list_items);
        let mut songs: Vec<_> = unused_assets.songs.into_iter()
            .map(|path| AssetListItem::new(path, &level_dir, true))
            .collect();
        songs.sort_by(compare_list_items);
        let mut ambiance: Vec<_> = unused_assets.ambiance.into_iter()
            .map(|path| AssetListItem::new(path, &level_dir, true))
            .collect();
        ambiance.sort_by(compare_list_items);
        let mut sounds: Vec<_> = unused_assets.sounds.into_iter()
            .map(|path| AssetListItem::new(path, &level_dir, true))
            .collect();
        sounds.sort_by(compare_list_items);
        let mut custom_objects: Vec<_> = unused_assets.custom_objects.into_iter()
            .map(|path| AssetListItem::new(path, &level_dir, true))
            .collect();
        custom_objects.sort_by(compare_list_items);
        let mut cutscenes: Vec<_> = unused_assets.cutscenes.into_iter()
            .map(|path| AssetListItem::new(path, &level_dir, false))
            .collect();
        cutscenes.sort_by(compare_list_items);
        let mut misc: Vec<_> = unused_assets.misc.into_iter()
            .map(|path| AssetListItem::new(path, &level_dir, false))
            .collect();
        misc.sort_by(compare_list_items);
        let mut ignored: Vec<_> = unused_assets.ignored.into_iter()
            .map(|path| AssetListItem::new(path, &level_dir, false))
            .collect();
        ignored.sort_by(compare_list_items);
        
        let audio_settings = AudioManagerSettings::default();
        let audio_manager = AudioManager::new(audio_settings)
            .expect("Create audio manager");
        let audio = AudioState {
            audio_manager,
            playing_index: None,
            playing_handle: None,
            duration: 0.0,
        };
        
        let level_icon = textures.create_texture_from_file(&level_dir.join("Icon.png"))
            .map(Option::Some)
            .unwrap_or(None);
        
        Self {
            level_name: level_dir.file_name().unwrap().to_string_lossy().to_string(),
            level_dir,
            level_icon,
            tilesets,
            gradients,
            songs,
            ambiance,
            sounds,
            custom_objects,
            cutscenes,
            misc,
            ignored,
            selected_index: None,
            prev_tab: Tab::Tilesets,
            audio,
        }
    }
}

pub struct AudioState {
    pub audio_manager: AudioManager,
    pub playing_index: Option<usize>,
    pub playing_handle: Option<StreamingSoundHandle<FromFileError>>,
    pub duration: f64,
}

impl AudioState {
    fn stop(&mut self, tween: Tween) {
        if let Some(mut handle) = self.playing_handle.take() {
            handle.stop(tween);
            self.playing_index = None;
        }
    }
    
    fn is_playing(&self) -> bool {
        self.playing_handle
            .as_ref()
            .is_some_and(|handle| handle.state() == PlaybackState::Playing)
    }
}

static FADE_OUT_SLOW: Tween = Tween {
    start_time: kira::StartTime::Immediate,
    duration: std::time::Duration::from_millis(400),
    easing: kira::Easing::Linear,
};

static FADE_OUT_FAST: Tween = Tween {
    start_time: kira::StartTime::Immediate,
    duration: std::time::Duration::from_millis(10),
    easing: kira::Easing::Linear,
};

pub struct AssetListItem {
    pub abs_path: PathBuf,
    pub rel_path: PathBuf,
    pub display_name: String,
    pub image: Option<Image>,
    pub text: Option<String>,
    pub selected: bool,
    pub kind: AssetKind,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum AssetKind {
    Image,
    Audio,
    Text,
    Other,
}

impl ChecklistItem for AssetListItem {
    fn label_and_checked(&mut self) -> (&str, &mut bool) {
        (&self.display_name, &mut self.selected)
    }
}

impl AssetListItem {
    fn new(rel_path: PathBuf, level_dir: &Path, skip_top_level: bool) -> Self {
        let extension = rel_path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase());
        let kind = match extension.as_ref().map(|ext| ext.as_str()) {
            Some("png") => AssetKind::Image,
            Some("ogg") => AssetKind::Audio,
            Some("txt" | "ini" | "html" | "lua") => AssetKind::Text,
            _ => AssetKind::Other,
        };
        
        let file_name =
            if skip_top_level {
                rel_path.iter()
                    .skip(1)
                    .collect::<PathBuf>()
                    .to_string_lossy()
                    .to_string()
            }
            else {
                rel_path.to_string_lossy()
                    .to_string()
            };
        
        Self {
            abs_path: level_dir.join(&rel_path),
            rel_path,
            display_name: file_name,
            image: None,
            text: None,
            selected: false,
            kind,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Tab {
    Tilesets,
    Gradients,
    Songs,
    Ambiance,
    Sounds,
    CustomObjects,
    Cutscenes,
    Misc,
    Ignored,
}

pub fn build_ui(_shared: &mut SharedState, state: &mut State, ui: &Ui, textures: &mut Textures) -> Task {
    // Level icon
    if let Some(image) = state.level_icon.as_ref() {
        let mut cursor_pos = ui.cursor_pos();
        let [text_width, text_height] = ui.calc_text_size(&state.level_name);
        let [spacing_x, _] = unsafe { ui.style().item_spacing };
        let [avail_width, _] = ui.content_region_avail();
        let x_offset = (avail_width - text_width) / 2.0 - spacing_x - 30.0;
        ui.move_cursor_right(x_offset);
        
        let top_left = ui.cursor_screen_pos();
        let bottom_right = [top_left[0] + 30.0, top_left[1] + 30.0];
        ui.get_window_draw_list()
            .add_rect(top_left, bottom_right, [235.0, 235.0, 235.0, 1.0])
            .filled(true)
            .build();
        
        imgui::Image::new(image.texture_id, [30.0, 30.0])
            .build(ui);
        
        let y_offset = (30.0 - text_height) / 2.0;
        cursor_pos[1] += y_offset;
        ui.set_cursor_pos(cursor_pos);
    }
    
    // Back button
    {
        let _padding = ui.push_style_var(StyleVar::FramePadding([0.0, 0.0]));
        let _button_color = ui.push_style_color(StyleColor::Button, [0.0, 0.0, 0.0, 0.0]);
        
        let cursor_pos = ui.cursor_pos();
        let [width_item, _] = ui.calc_text_size("Go Back");
        let [width_avail, _] = ui.content_region_avail();
        ui.move_cursor_right(width_avail - width_item);
        
        if ui.button("Go Back") {
            for image in state.tilesets.iter_mut()
                .chain(state.gradients.iter_mut())
                .chain(state.custom_objects.iter_mut())
                .chain(state.cutscenes.iter_mut())
                .chain(state.misc.iter_mut())
                .filter_map(|asset| asset.image.take())
            {
                textures.delete_texture(image.texture_id);
            }
            return Task::SelectLevel;
        }
        ui.set_cursor_pos(cursor_pos);
    }
    
    // Level title
    ui.text_centered(&state.level_name);
    
    // Tabs
    let Some(_tab_bar) = ui.tab_bar("##Tabs") else { return Task::None };
    if let Some(_tab_item) = ui.tab_item(format!("Tilesets ({})", state.tilesets.len())) {
        build_tab(ui, state, textures, Tab::Tilesets);
    }
    if let Some(_tab_item) = ui.tab_item(format!("Gradients ({})", state.gradients.len())) {
        build_tab(ui, state, textures, Tab::Gradients);
    }
    if let Some(_tab_item) = ui.tab_item(format!("Songs ({})", state.songs.len())) {
        build_tab(ui, state, textures, Tab::Songs);
    }
    if let Some(_tab_item) = ui.tab_item(format!("Ambiance ({})", state.ambiance.len())) {
        build_tab(ui, state, textures, Tab::Ambiance);
    }
    if let Some(_tab_item) = ui.tab_item(format!("Sounds ({})", state.sounds.len())) {
        build_tab(ui, state, textures, Tab::Sounds);
    }
    if let Some(_tab_item) = ui.tab_item(format!("Custom Objects ({})", state.custom_objects.len())) {
        build_tab(ui, state, textures, Tab::CustomObjects);
    }
    if let Some(_tab_item) = ui.tab_item(format!("Cutscenes ({})", state.cutscenes.len())) {
        build_tab(ui, state, textures, Tab::Cutscenes);
    }
    if let Some(_tab_item) = ui.tab_item(format!("Misc ({})", state.misc.len())) {
        build_tab(ui, state, textures, Tab::Misc);
    }
    if let Some(_tab_item) = ui.tab_item(format!("Ignored ({})", state.ignored.len())) {
        build_tab(ui, state, textures, Tab::Ignored);
    }
    
    Task::None
}

fn build_tab(ui: &Ui, state: &mut State, textures: &mut Textures, tab: Tab) {
    if tab != state.prev_tab {
        state.audio.stop(FADE_OUT_SLOW);
        state.prev_tab = tab;
    }
    
    let assets = match tab {
        Tab::Tilesets => &mut state.tilesets,
        Tab::Gradients => &mut state.gradients,
        Tab::Songs => &mut state.songs,
        Tab::Ambiance => &mut state.ambiance,
        Tab::Sounds => &mut state.sounds,
        Tab::CustomObjects => &mut state.custom_objects,
        Tab::Cutscenes => &mut state.cutscenes,
        Tab::Misc => &mut state.misc,
        Tab::Ignored => &mut state.ignored,
    };
    
    let [width, _] = ui.content_region_avail();
    let [item_spacing_x, _] = unsafe { ui.style() }.item_spacing;
    let preview_width = f32::max(600.0 + 20.0, width / 2.0);
    let checklist_width = width - preview_width - item_spacing_x;
    
    let _group = ui.begin_group();
    
    if ui.button("Check all") {
        assets.iter_mut().for_each(|item| item.selected = true);
    }
    
    ui.same_line();
    if ui.button("Check none") {
        assets.iter_mut().for_each(|item| item.selected = false);
    }
    
    {
        ui.same_line();
        let n_checked = assets.iter().filter(|item| item.selected).count();
        let _disabled = ui.begin_disabled(n_checked == 0);
        if ui.button(format!("Delete checked ({})", n_checked)) {
            ui.open_popup(tab.modal_id_multiple());
        }

        let modal_padding = ui.push_style_var(StyleVar::WindowPadding([10.0, 10.0]));
        ui.modal_popup_config(tab.modal_id_multiple())
            .movable(false)
            .resizable(false)
            .always_auto_resize(true)
            .build(|| {
                ui.text(format!("Are you sure you want to delete {} {}?", n_checked, tab.asset_plural()));
                
                if ui.button("Yes, destroy them all!") {
                    assets.retain(|item| {
                        if item.selected {
                            trash::delete(&item.abs_path).is_err()
                        }
                        else {
                            true
                        }
                    });
                    state.selected_index = None;
                    ui.close_current_popup();
                }
                
                ui.same_line();
                if ui.button("No, don't do it!") {
                    ui.close_current_popup()
                }
            });
        modal_padding.end();
    }
    
    checklist_box(ui, tab.checklist_id(), &mut state.selected_index, assets, [checklist_width, -1.0]);
    
    _group.end();
    ui.same_line();
    
    let _group = ui.begin_group();
    let _disabled = ui.begin_disabled(state.selected_index.is_none());
    
    if ui.button("Show in Explorer") {
        let _ = std::process::Command::new("explorer.exe")
            .arg("/select,")
            .arg(assets[state.selected_index.unwrap()].abs_path.as_os_str())
            .output();
    }

    ui.same_line();
    if ui.button("Delete this") {
        ui.open_popup(tab.modal_id_single());
    }

    let modal_padding = ui.push_style_var(StyleVar::WindowPadding([10.0, 10.0]));
    ui.modal_popup_config(tab.modal_id_single())
        .movable(false)
        .resizable(false)
        .always_auto_resize(true)
        .build(|| {
            let Some(i) = state.selected_index else {
                return ui.close_current_popup();
            };
            
            let rel_path = assets[i].rel_path.to_string_lossy();
            ui.text(format!("Are you sure you want to delete {rel_path}?"));
            
            if ui.button("Yes, destroy it!") {
                if trash::delete(&assets[i].abs_path).is_ok() {
                    assets.remove(state.selected_index.unwrap());
                    state.selected_index = None;
                }
                ui.close_current_popup();
            }
            
            ui.same_line();
            if ui.button("No, don't do it!") {
                ui.close_current_popup()
            }
        });
    modal_padding.end();

    let _bg_color = ui.push_style_color(StyleColor::ChildBg, [0.1, 0.1, 0.1, 1.0]);
    let _padding = ui.push_style_var(StyleVar::WindowPadding([10.0, 10.0]));
    ui.child_window("Preview")
        .size([-1.0, -1.0])
        .menu_bar(false)
        .movable(false)
        .scrollable(true)
        .flags(WindowFlags::HORIZONTAL_SCROLLBAR | WindowFlags::ALWAYS_USE_WINDOW_PADDING)
        .build(|| {
            let Some(i) = state.selected_index else { return };
            let item = &mut assets[i];
            match item.kind {
                AssetKind::Image => {
                    build_preview_image(ui, textures, item);
                }
                AssetKind::Audio => {
                    build_preview_audio(ui, item, i, &mut state.audio);
                }
                AssetKind::Text => {
                    build_preview_text(ui, item);
                }
                AssetKind::Other => {
                    ui.text("No preview available.");
                }
            }
        });
}

fn build_preview_image(ui: &Ui, textures: &mut Textures, item: &mut AssetListItem) {
    if item.image.is_none() {
        // FIXME error handling
        item.image = Some(textures.create_texture_from_file(&item.abs_path).unwrap());
    }
    if let Some(image) = &item.image {
        imgui::Image::new(image.texture_id, [image.width, image.height])
            .build(ui);
    }
}

fn build_preview_audio(ui: &Ui, item: &AssetListItem, index: usize, audio_state: &mut AudioState) {    
    let size = [168.0, 48.0];
    let is_playing =
        audio_state.playing_index == Some(index)
        && audio_state.is_playing();
    
    if is_playing {
        let t = audio_state.playing_handle
            .as_ref()
            .map(|handle| handle.position() / audio_state.duration)
            .unwrap_or(1.0) as f32;
        
        let top_left = ui.cursor_screen_pos();
        let button_bottom_right = [top_left[0] + size[0], top_left[1] + size[1]];
        let progress_bottom_right = [top_left[0] + t * size[0], top_left[1] + size[1]];
        
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
        let color_button = [color_progress[0], color_progress[1], color_progress[2], color_progress[3] * 0.5];
            
        ui.get_window_draw_list()
            .add_rect(top_left, progress_bottom_right, color_progress)
            .filled(true)
            .build();
        
        let _token = ui.push_style_color(style_color, color_button);
        if ui.button_with_size("Stop", size) {
            audio_state.stop(FADE_OUT_FAST);
        }
    }
    else {
        if ui.button_with_size("Play", size) {
            audio_state.stop(FADE_OUT_FAST);
            if let Ok(data) = StreamingSoundData::from_file(&item.abs_path) {
                audio_state.duration = data.duration().as_secs_f64();
                if let Ok(handle) = audio_state.audio_manager.play(data) {
                    audio_state.playing_handle = Some(handle);
                    audio_state.playing_index = Some(index);
                }
            }
        }
    }
}

fn build_preview_text(ui: &Ui, item: &mut AssetListItem) {
    if item.text.is_none()
        && let Ok(text) = fs::read_to_string(&item.abs_path)
    {
        item.text = Some(text);
    }
    if let Some(text) = item.text.as_ref() {
        let _border = ui.push_style_var(StyleVar::FrameBorderSize(1.0));
        ui.text_wrapped(text);
    }
}

impl Tab {
    fn checklist_id(&self) -> &'static str {
        match self {
            Tab::Tilesets => "##Tilesets",
            Tab::Gradients => "##Gradients",
            Tab::Songs => "##Songs",
            Tab::Ambiance => "##Ambiance",
            Tab::Sounds => "##Sounds",
            Tab::CustomObjects => "##Custom objects",
            Tab::Cutscenes => "##Cutscenes",
            Tab::Misc => "##Misc files",
            Tab::Ignored => "##Ignored files",
        }
    }
    
    fn modal_id_multiple(&self) -> &'static str {
        match self {
            Tab::Tilesets => "Delete tilesets",
            Tab::Gradients => "Delete gradients",
            Tab::Songs => "Delete songs",
            Tab::Ambiance => "Delete ambiance",
            Tab::Sounds => "Delete sounds",
            Tab::CustomObjects => "Delete custom objects",
            Tab::Cutscenes => "Delete cutscenes",
            Tab::Misc => "Delete misc files",
            Tab::Ignored => "Delete ignored files",
        }
    }
    
    fn modal_id_single(&self) -> &'static str {
        match self {
            Tab::Tilesets => "Delete one tileset",
            Tab::Gradients => "Delete one gradient",
            Tab::Songs => "Delete one song",
            Tab::Ambiance => "Delete one ambiance track",
            Tab::Sounds => "Delete one sound",
            Tab::CustomObjects => "Delete one custom object",
            Tab::Cutscenes => "Delete one cutscene",
            Tab::Misc => "Delete one misc file",
            Tab::Ignored => "Delete one ignored file",
        }
    }
    
    fn asset_plural(&self) -> &'static str {
        match self {
            Tab::Tilesets => "tilesets",
            Tab::Gradients => "gradients",
            Tab::Songs => "songs",
            Tab::Ambiance => "ambiance tracks",
            Tab::Sounds => "sounds",
            Tab::CustomObjects => "custom objects",
            Tab::Cutscenes => "cutscene files",
            Tab::Misc => "files",
            Tab::Ignored => "files",
        }
    }
}

fn compare_list_items(a: &AssetListItem, b: &AssetListItem) -> std::cmp::Ordering {
    natord::compare(&a.display_name, &b.display_name)
}
