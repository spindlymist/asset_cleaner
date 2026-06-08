use std::{borrow::Cow, fs::{self, File}, path::{Path, PathBuf}};

use anyhow::Result;
use chardetng::EncodingDetector;
use imgui_app::{dear_imgui_rs, Fonts, ImguiCursorExt, Textures};
use dear_imgui_rs::{ChildFlags, FontId, StyleColor, StyleVar, TableFlags, TableOptions, TableSizingPolicy, Ui, WindowFlags};
use kira::{AudioManager, AudioManagerSettings, Tween, sound::{FromFileError, PlaybackState, streaming::{StreamingSoundData, StreamingSoundHandle}}};
use symphonia::{core::{formats::FormatReader, io::MediaSourceStream, meta::StandardTag, units::Timestamp}, default::formats::OggReader};

use crate::{SharedState, Task, analysis::UnusedAssets, widgets::{ChecklistItem, checklist_box, error_display, progress_button_with_size, text_centered}};

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
const N_TABS: usize = Tab::Ignored as usize + 1;

pub struct State {
    #[allow(dead_code)]
    pub level_dir: PathBuf,
    pub level_name: String,
    pub level_icon: Option<u64>,
    pub tilesets: Vec<AssetListItem>,
    pub gradients: Vec<AssetListItem>,
    pub songs: Vec<AssetListItem>,
    pub ambiance: Vec<AssetListItem>,
    pub sounds: Vec<AssetListItem>,
    pub custom_objects: Vec<AssetListItem>,
    pub cutscenes: Vec<AssetListItem>,
    pub misc: Vec<AssetListItem>,
    pub ignored: Vec<AssetListItem>,
    pub selected_indices: [Option<usize>; N_TABS],
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
            stream_handle: None,
            duration: 0.0,
        };
        
        let level_icon = textures.create_texture_from_file(&level_dir.join("Icon.png"))
            .map(Option::Some)
            .unwrap_or(None);
        
        Self {
            level_name: level_dir.file_name().unwrap().to_string_lossy().into_owned(),
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
            selected_indices: [Some(0); N_TABS],
            prev_tab: Tab::Tilesets,
            audio,
        }
    }
}

pub struct AudioState {
    pub audio_manager: AudioManager,
    pub stream_handle: Option<StreamingSoundHandle<FromFileError>>,
    pub duration: f64,
}

impl AudioState {
    fn stop(&mut self, tween: Tween) {
        if let Some(mut handle) = self.stream_handle.take() {
            handle.stop(tween);
        }
    }
    
    fn is_playing(&self) -> bool {
        self.stream_handle
            .as_ref()
            .is_some_and(|handle| handle.state() == PlaybackState::Playing)
    }
}

static FADE_OUT_SLOW: Tween = Tween {
    start_time: kira::StartTime::Immediate,
    duration: std::time::Duration::from_millis(300),
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
    pub error: Option<anyhow::Error>,
    pub texture_id: Option<u64>,
    pub text: Option<String>,
    pub meta: Option<Metadata>,
    pub is_checked: bool,
    pub kind: AssetKind,
}

#[derive(Debug, Default)]
pub struct Metadata {
    artist: String,
    title: String,
    duration: String,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum AssetKind {
    Image,
    Audio,
    Text,
    Other,
}

impl ChecklistItem for AssetListItem {
    fn label(&self) -> &str {
        &self.display_name
    }
    
    fn label_and_is_checked_mut(&mut self) -> (&str, &mut bool) {
        (&self.display_name, &mut self.is_checked)
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
                    .into_owned()
            }
            else {
                rel_path.to_string_lossy().into_owned()
            };
        
        Self {
            abs_path: level_dir.join(&rel_path),
            display_name: file_name,
            rel_path,
            error: None,
            texture_id: None,
            text: None,
            meta: None,
            is_checked: false,
            kind,
        }
    }
}

pub fn build_ui(_shared: &mut SharedState, state: &mut State, ui: &Ui, textures: &mut Textures, fonts: &Fonts) -> Task {
    ui.move_cursor_down(8.0);
    
    // Level icon
    if let Some(image) = state.level_icon.as_ref()
        .and_then(|id| textures.get_texture(*id))
    {
        const ICON_WIDTH: f32 = 30.0;
        const ICON_HEIGHT: f32 = 30.0;
        
        let mut cursor_pos = ui.cursor_pos();
        let [text_width, text_height] = ui.current_font().calc_text_size(ui.current_font_size(), f32::MAX, 0.0, &state.level_name);
        let [spacing_x, _] = unsafe { ui.style().item_spacing() };
        let [avail_width, _] = ui.content_region_avail();
        let x_offset = f32::round((avail_width - text_width) / 2.0) - spacing_x - ICON_WIDTH;
        ui.move_cursor_right(x_offset);
        
        let top_left = ui.cursor_screen_pos();
        let bottom_right = [top_left[0] + ICON_WIDTH, top_left[1] + ICON_HEIGHT];
        ui.get_window_draw_list()
            .add_rect(top_left, bottom_right, [235.0, 235.0, 235.0, 1.0])
            .filled(true)
            .build();
        
        dear_imgui_rs::Image::new(ui, image.texture_data.as_ref(), [ICON_WIDTH, ICON_HEIGHT])
            .build();
        
        let y_offset = f32::round((ICON_HEIGHT - text_height) / 2.0);
        cursor_pos[1] += y_offset;
        ui.set_cursor_pos(cursor_pos);
    }
    
    // Back button
    {
        let _button_color = ui.push_style_color(StyleColor::Button, [0.0, 0.0, 0.0, 0.0]);
        
        let frame_padding = unsafe { ui.style().frame_padding() };
        let cursor_pos = ui.cursor_pos();
        let width_text = ui.current_font().calc_text_size(ui.current_font_size(), f32::MAX, 0.0, "Go Back")[0];
        let width_padding = frame_padding[0] * 2.0;
        let width_avail = ui.content_region_avail()[0];
        ui.move_cursor([width_avail - 8.0 - width_text - width_padding, -frame_padding[1]]);
        
        if ui.button("Go Back") {
            for texture_id in state.tilesets.iter_mut()
                .chain(state.gradients.iter_mut())
                .chain(state.custom_objects.iter_mut())
                .chain(state.cutscenes.iter_mut())
                .chain(state.misc.iter_mut())
                .filter_map(|asset| asset.texture_id.take())
            {
                textures.delete_texture(texture_id);
            }
            return Task::SelectLevel;
        }
        ui.set_cursor_pos(cursor_pos);
    }
    
    // Level title
    text_centered(ui, &state.level_name);
    
    ui.move_cursor_down(8.0);
    
    // Tabs
    let Some(_tab_bar) = ui.tab_bar("##Tabs") else { return Task::None };
    if let Some(_tab_item) = ui.tab_item(format!("Tilesets ({})###Tilesets", state.tilesets.len())) {
        build_tab(ui, state, textures, fonts, Tab::Tilesets);
    }
    if let Some(_tab_item) = ui.tab_item(format!("Gradients ({})###Gradients", state.gradients.len())) {
        build_tab(ui, state, textures, fonts, Tab::Gradients);
    }
    if let Some(_tab_item) = ui.tab_item(format!("Songs ({})###Songs", state.songs.len())) {
        build_tab(ui, state, textures, fonts, Tab::Songs);
    }
    if let Some(_tab_item) = ui.tab_item(format!("Ambiance ({})###Ambiance", state.ambiance.len())) {
        build_tab(ui, state, textures, fonts, Tab::Ambiance);
    }
    if let Some(_tab_item) = ui.tab_item(format!("Sounds ({})###Sounds", state.sounds.len())) {
        build_tab(ui, state, textures, fonts, Tab::Sounds);
    }
    if let Some(_tab_item) = ui.tab_item(format!("Custom Objects ({})###Custom Objects", state.custom_objects.len())) {
        build_tab(ui, state, textures, fonts, Tab::CustomObjects);
    }
    if let Some(_tab_item) = ui.tab_item(format!("Cutscenes ({})###Cutscenes", state.cutscenes.len())) {
        build_tab(ui, state, textures, fonts, Tab::Cutscenes);
    }
    if let Some(_tab_item) = ui.tab_item(format!("Misc ({})###Misc", state.misc.len())) {
        build_tab(ui, state, textures, fonts, Tab::Misc);
    }
    if let Some(_tab_item) = ui.tab_item(format!("Ignored ({})###Ignored", state.ignored.len())) {
        build_tab(ui, state, textures, fonts, Tab::Ignored);
    }
    
    Task::None
}

fn build_tab(ui: &Ui, state: &mut State, textures: &mut Textures, fonts: &Fonts, tab: Tab) {
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
    let selected_index = &mut state.selected_indices[tab as usize];
    
    let cursor_initial_screen_pos = ui.cursor_screen_pos();
    let full_width = ui.content_region_avail()[0];
    let group_left = ui.begin_group();
    
    if ui.button("Check all") {
        assets.iter_mut().for_each(|item| item.is_checked = true);
    }
    
    ui.same_line();
    if ui.button("Check none") {
        assets.iter_mut().for_each(|item| item.is_checked = false);
    }
    
    ui.same_line();
    {
        let n_checked = assets.iter().filter(|item| item.is_checked).count();
        let _disabled = ui.begin_disabled_with_cond(n_checked == 0);
        if ui.button(format!("Delete checked ({})", n_checked)) {
            ui.open_popup(tab.modal_id_multiple());
        }
        if modal_delete_multiple(ui, tab, assets, textures, n_checked, selected_index)
            && selected_index.is_none()
        {
            state.audio.stop(FADE_OUT_FAST);
        }
    }
    
    const PREVIEW_MIN_WIDTH: f32 = 600.0 + 20.0;
    let item_spacing_x = unsafe { ui.style().item_spacing()[0] };
    let buttons_width = ui.item_rect_max()[0] - cursor_initial_screen_pos[0];
    let left_half_width = f32::floor((full_width - item_spacing_x) * 0.5);
    let right_half_width = full_width - left_half_width;
    let checklist_width =
        if right_half_width >= PREVIEW_MIN_WIDTH {
            left_half_width
        }
        else {
            buttons_width
        };

    let prev_index = selected_index.clone();
    checklist_box(ui, tab.checklist_id(), selected_index, assets, [checklist_width, -1.0]);
    if *selected_index != prev_index {
        state.audio.stop(FADE_OUT_SLOW);
    }
    
    group_left.end();
    ui.same_line();
    let _group_right = ui.begin_group();
    
    {
        let _disabled = ui.begin_disabled_with_cond(selected_index.is_none());
        
        if ui.button("Show in Explorer") {
            let _ = std::process::Command::new("explorer.exe")
                .arg("/select,")
                .arg(assets[selected_index.unwrap()].abs_path.as_os_str())
                .output();
        }

        ui.same_line();
        if ui.button("Delete this") {
            ui.open_popup(tab.modal_id_single());
        }
        if modal_delete_single(ui, tab, assets, textures, selected_index)
            && selected_index.is_none()
        {
            state.audio.stop(FADE_OUT_FAST);
        }
    }
    
    {
        let _bg_color = ui.push_style_color(StyleColor::ChildBg, [0.1, 0.1, 0.1, 1.0]);
        let _padding = ui.push_style_var(StyleVar::WindowPadding([10.0, 10.0]));
        ui.child_window("Preview")
            .size([-1.0, -1.0])
            .flags(WindowFlags::NO_MOVE | WindowFlags::NO_TITLE_BAR | WindowFlags::HORIZONTAL_SCROLLBAR)
            .child_flags(ChildFlags::ALWAYS_USE_WINDOW_PADDING)
        .build(ui, || {
            if let Some(item) = selected_index
                .as_ref()
                .and_then(|i| assets.get_mut(*i))
            {
                match item.kind {
                    AssetKind::Image => build_preview_image(ui, textures, item),
                    AssetKind::Audio => build_preview_audio(ui, item, &mut state.audio),
                    AssetKind::Text => build_preview_text(ui, item, fonts.mono),
                    AssetKind::Other => ui.text("No preview available.")
                }
            }
        });
    }
}

fn build_preview_image(ui: &Ui, textures: &mut Textures, item: &mut AssetListItem) {
    let should_load = match &item.error {
        Some(err) => {
            ui.text("An error occurred while loading the preview:");
            error_display(ui, err);
            ui.button("Retry")
        }
        None => item.texture_id.is_none()
    };
    
    if should_load {
        match textures.create_texture_from_file(&item.abs_path) {
            Ok(val) => {
                item.texture_id = Some(val);
                item.error = None;
            }
            Err(err) => item.error = Some(err.into())
        }
        return; // Don't render this frame to prevent flash
    }
    
    if let Some(image) = item.texture_id.as_ref().and_then(|id| textures.get_texture(*id)) {
        if image.is_downscaled {
            ui.text("Preview is downscaled.");
        }
        dear_imgui_rs::Image::new(ui, image.texture_data.as_ref(), [
                image.texture_data.width() as f32,
                image.texture_data.height() as f32
            ])
            .build();
    }
}

fn build_preview_audio(ui: &Ui, item: &mut AssetListItem, audio_state: &mut AudioState) {    
    if item.meta.is_none() {
        item.meta = Some(metadata_from_file(&item.abs_path).unwrap_or_default());
    }
    
    if let Some(meta) = &item.meta {
        let options = TableOptions::new()
            .flags(TableFlags::BORDERS)
            .sizing_policy(TableSizingPolicy::FixedFit);
        if let Some(_table) = ui.begin_table_with_flags("##Metadata", 2, options) {
            ui.table_next_row();
            ui.table_next_column();
            ui.text("Artist");
            ui.table_next_column();
            ui.text(&meta.artist);
            
            ui.table_next_row();
            ui.table_next_column();
            ui.text("Title");
            ui.table_next_column();
            ui.text(&meta.title);
            
            ui.table_next_row();
            ui.table_next_column();
            ui.text("Duration");
            ui.table_next_column();
            ui.text(&meta.duration);
        }
        ui.move_cursor_down(4.0);
    }
    
    let play_button_size = [168.0, 48.0];
    if audio_state.is_playing() {
        let t = audio_state.stream_handle
            .as_ref()
            .map(|handle| handle.position() / audio_state.duration)
            .unwrap_or(1.0) as f32;
        if progress_button_with_size(ui, "Stop", play_button_size, t) {
            audio_state.stop(FADE_OUT_FAST);
        }
    }
    else if ui.button_with_size("Play", play_button_size) {
        audio_state.stop(FADE_OUT_FAST);
        match StreamingSoundData::from_file(&item.abs_path) {
            Ok(data) => {
                let duration = data.duration().as_secs_f64();
                match audio_state.audio_manager.play(data) {
                    Ok(handle) => {
                        audio_state.stream_handle = Some(handle);
                        audio_state.duration = duration;
                        item.error = None;
                    }
                    Err(err) => {
                        item.error = Some(anyhow::Error::new(err).context("Failed to play sound."));
                    }
                };
            }
            Err(err) => {
                item.error = Some(anyhow::Error::new(err).context("Failed to load sound."));
            }
        };
    }
    
    if let Some(ref err) = item.error {
        ui.text("An error occurred while loading the preview:");
        error_display(ui, err);
    }
}

fn build_preview_text(ui: &Ui, item: &mut AssetListItem, font: FontId) {
    let should_load = match item.error {
        Some(ref err) => {
            ui.text("An error occurred while loading the preview:");
            error_display(ui, err);
            ui.button("Retry")
        }
        None => item.text.is_none()
    };
    
    if should_load {
        match fs::read(&item.abs_path) {
            Ok(bytes) => {
                let ext = item.abs_path.extension().map(|ext| ext.to_ascii_lowercase());
                let ext = ext.as_ref()
                    .and_then(|ext| ext.to_str())
                    .unwrap_or("");
                
                let encoding = match ext {
                    "ini" => encoding_rs::WINDOWS_1252,
                    _ => {
                        let mut detector = EncodingDetector::new(chardetng::Iso2022JpDetection::Allow);
                        detector.feed(&bytes, true);
                        detector.guess(None, chardetng::Utf8Detection::Allow)
                    }
                };
                let text = match encoding.decode_with_bom_removal(&bytes) {
                    (Cow::Owned(bytes), _) => { bytes },
                    (Cow::Borrowed(_), _) => unsafe { String::from_utf8_unchecked(bytes) }
                };
                
                item.text = Some(text);
                item.error = None;
            }
            Err(err) => {
                item.error = Some(err.into())
            }
        };
    }
    
    if let Some(text) = &item.text {
        let _font = ui.push_font(font);
        ui.text(text);
    }
}

fn modal_delete_single(
    ui: &Ui,
    tab: Tab,
    assets: &mut Vec<AssetListItem>,
    textures: &mut Textures,
    selected_index: &mut Option<usize>,
) -> bool {
    let mut did_delete_file = false;
    
    let _modal_padding = ui.push_style_var(StyleVar::WindowPadding([10.0, 10.0]));
    
    if let Some(_modal) = ui.begin_modal_popup_config(tab.modal_id_single())
        .flags(WindowFlags::NO_MOVE | WindowFlags::NO_RESIZE | WindowFlags::ALWAYS_AUTO_RESIZE)
        .begin()
    {
        let Some(i) = selected_index else {
            ui.close_current_popup();
            return false;
        };
        let item = &assets[*i];
        
        let rel_path = item.rel_path.to_string_lossy();
        ui.text(format!("Are you sure you want to delete {rel_path}?"));
        
        if ui.button("Yes, destroy it!") {
            if trash::delete(&item.abs_path).is_ok() {
                if let Some(texture_id) = &item.texture_id {
                    textures.delete_texture(*texture_id);
                }
                assets.remove(*i);
                selected_index.take();
                did_delete_file = true;
            }
            ui.close_current_popup();
        }
        
        ui.same_line();
        if ui.button("No, don't do it!") {
            ui.close_current_popup();
        }
    }
    
    did_delete_file
}

fn modal_delete_multiple(
    ui: &Ui,
    tab: Tab,
    assets: &mut Vec<AssetListItem>,
    textures: &mut Textures,
    n_checked: usize,
    selected_index: &mut Option<usize>,
) -> bool {
    let mut did_delete_files = false;
    
    let _padding = ui.push_style_var(StyleVar::WindowPadding([10.0, 10.0]));
    if let Some(_modal) = ui.begin_modal_popup_config(tab.modal_id_multiple())
        .flags(WindowFlags::NO_MOVE | WindowFlags::NO_RESIZE | WindowFlags::ALWAYS_AUTO_RESIZE)
        .begin()
    {
        if n_checked < 1 {
            ui.close_current_popup();
            return false;
        }
        
        ui.text(format!("Are you sure you want to delete {} {}?", n_checked, tab.asset_plural()));
        
        if ui.button("Yes, destroy them all!") {
            let previously_selected_path = selected_index
                .as_ref()
                .and_then(|i| assets.get(*i))
                .map(|item| item.rel_path.clone());
            
            assets.retain(|item| {
                if !item.is_checked {
                    return true;
                }
                match trash::delete(&item.abs_path) {
                    Ok(_) => {
                        did_delete_files = true;
                        if let Some(texture_id) = &item.texture_id {
                            textures.delete_texture(*texture_id);
                        }
                        false
                    }
                    Err(_) => true
                }
            });
            
            // Restore the previous selection if it wasn't deleted
            if let Some(path) = previously_selected_path
                && let Some(new_index) = assets.iter().position(|item| item.rel_path == path)
            {
                selected_index.replace(new_index);
            }
            else {
                selected_index.take();
            }
            
            ui.close_current_popup();
        }
        
        ui.same_line();
        if ui.button("No, don't do it!") {
            ui.close_current_popup();
        }
    }
    
    did_delete_files
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

fn metadata_from_file(path: &Path) -> Result<Metadata> {
    let mut artist = None;
    let mut title = None;
    let mut duration_readable = None;
    
    let file = Box::new(File::open(path)?);
    let source_stream = MediaSourceStream::new(file, Default::default());
    let mut reader = OggReader::try_new(source_stream, Default::default())?;
    
    let mut meta = reader.metadata();
    if let Some(meta) = meta.skip_to_latest() {
        for tag in &meta.media.tags {
            match &tag.std {
                Some(StandardTag::Artist(value)) => {
                    artist = Some(String::clone(value));
                }
                Some(StandardTag::TrackTitle(value)) => {
                    title = Some(String::clone(value));
                }
                _ => {}
            };
        }
    };
    
    if let Some(track) = reader.default_track(symphonia::core::formats::TrackType::Audio)
        && let Some(time_base) = &track.time_base
        && let Some(duration) = &track.duration
    {
        let timestamp = duration.timestamp_from(Timestamp::ZERO).unwrap();
        let seconds = time_base.calc_time(timestamp).unwrap()
            .as_secs_f64()
            .round() as u64;
        duration_readable = Some(human_readable_duration(seconds));
    }
    
    Ok(Metadata {
        artist: artist.unwrap_or_default(),
        title: title.unwrap_or_default(),
        duration: duration_readable.unwrap_or_default(),
    })
}

fn human_readable_duration(seconds: u64) -> String {
    const ONE_HOUR: u64 = 60 * 60;
    
    let h = seconds / ONE_HOUR;
    let m = (seconds % ONE_HOUR) / 60;
    let s = (seconds % ONE_HOUR) % 60;
    
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    }
    else {
        format!("{m}:{s:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::human_readable_duration;
    
    #[test]
    fn human_readable_duration_zero_seconds() {
        let seconds = 0;
        let expected = "0:00";
        assert_eq!(human_readable_duration(seconds), expected);
    }
    
    #[test]
    fn human_readable_duration_less_than_one_minute() {
        let seconds = 30;
        let expected = "0:30";
        assert_eq!(human_readable_duration(seconds), expected);
    }
    
    #[test]
    fn human_readable_duration_more_than_one_minute() {
        let seconds = 60 * 5 + 45;
        let expected = "5:45";
        assert_eq!(human_readable_duration(seconds), expected);
    }
    
    #[test]
    fn human_readable_duration_more_than_ten_minutes() {
        let seconds = 60 * 12 + 30;
        let expected = "12:30";
        assert_eq!(human_readable_duration(seconds), expected);
    }
    
    #[test]
    fn human_readable_duration_more_than_one_hour() {
        let seconds = 60 * 60 + 1;
        let expected = "1:00:01";
        assert_eq!(human_readable_duration(seconds), expected);
    }
}
