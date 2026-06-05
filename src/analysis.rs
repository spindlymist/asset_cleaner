use std::{collections::{HashMap, HashSet}, ops::RangeBounds, path::{Path, PathBuf}, str::FromStr, sync::mpsc};

use anyhow::{Context, Result};
use libks::map_bin::LayerData;
use walkdir::WalkDir;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum AnalysisStatus {
    EnumeratingFiles,
    AnalyzingMap,
    AnalyzingIni,
    Aborted,
    Finished,
}

pub struct IndexedAssets {
    pub tilesets: [bool; 256],
    pub gradients: [bool; 256],
    pub songs: [bool; 256],
    pub ambiance: [bool; 256],
}

pub struct CustomObjectsUsed {
    pub bank_255: [bool; 256],
    pub bank_254: [bool; 256],
}

pub struct NamedAssets {
    pub cutscenes: Vec<PathBuf>,
    pub custom_objects: Vec<PathBuf>,
    pub sounds: Vec<PathBuf>,
    pub songs: Vec<PathBuf>,
    pub ambiance: Vec<PathBuf>,
    pub savegames: Vec<PathBuf>,
}

pub struct UnusedAssets {
    pub tilesets: Vec<PathBuf>,
    pub gradients: Vec<PathBuf>,
    pub songs: Vec<PathBuf>,
    pub ambiance: Vec<PathBuf>,
    pub sounds: Vec<PathBuf>,
    pub custom_objects: Vec<PathBuf>,
    pub cutscenes: Vec<PathBuf>,
    pub misc: Vec<PathBuf>,
    pub ignored: Vec<PathBuf>,
}

pub fn list_all_assets(world_path: &Path) -> Result<(HashMap<PathBuf, PathBuf>, Vec<PathBuf>)> {
    let current_dir = std::env::current_dir()
        .context("Failed to get current directory")?;
    std::env::set_current_dir(world_path)
        .context("Failed to set current directory")?;
    
    let mut assets = HashMap::new();
    let mut ignored = Vec::new();
    
    for entry in WalkDir::new(".") {
        let entry = entry.context("Failed while walking level directory")?;
        
        if !entry.file_type().is_file() {
            continue;
        }
        
        let ext = entry.path()
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let rel_path: PathBuf = entry.path().iter()
            .skip(1)
            .collect();
        
        if !matches!(ext.as_str(), "bin" | "html" | "ini" | "ogg" | "png" | "txt") {
            ignored.push(rel_path);
            continue;
        }
        
        let key: PathBuf = rel_path.iter()
            .map(|part| part.to_ascii_lowercase())
            .collect();
        assets.insert(key, rel_path);
    }
    
    std::env::set_current_dir(current_dir)
        .context("Failed to restore current directory")?;
    
    Ok((assets, ignored))
}

pub fn list_unused_assets(tx: mpsc::Sender<AnalysisStatus>, world_path: PathBuf) -> Result<UnusedAssets> {
    let _ = tx.send(AnalysisStatus::EnumeratingFiles);
    let (mut assets, ignored) = list_all_assets(&world_path)?;
    remove_well_known_assets(&mut assets);
    
    let _ = tx.send(AnalysisStatus::AnalyzingMap);
    let (mut indexed_assets, custom_objects_used) = analyze_map(&world_path)?;
    indexed_assets.gradients[0] = true;
    indexed_assets.songs[0] = true;
    indexed_assets.ambiance[0] = false;
    
    let _ = tx.send(AnalysisStatus::AnalyzingIni);
    let named_assets = analyze_ini(&world_path, &custom_objects_used)?;
    
    remove_named_assets(&mut assets, &named_assets);
    remove_indexed_assets(&mut assets, &indexed_assets);
    
    let assets = sort_assets(assets, ignored);
    Ok(assets)
}

fn is_range_with_affixes<B, T>(s: &str, prefix: &str, suffix: &str, range: B) -> bool
where
    B: RangeBounds<T>,
    T: FromStr + PartialOrd<T>,
{
    let Some(affixed) = s.strip_suffix(suffix)
        .and_then(|s| s.strip_prefix(prefix))
    else {
        return false;
    };

    let Ok(number) = str::parse::<T>(affixed) else {
        return false;
    };

    range.contains(&number)
}

fn sort_assets(assets: HashMap<PathBuf, PathBuf>, ignored: Vec<PathBuf>) -> UnusedAssets {
    let mut tilesets = Vec::new();
    let mut gradients = Vec::new();
    let mut songs = Vec::new();
    let mut ambiance = Vec::new();
    let mut sounds = Vec::new();
    let mut custom_objects = Vec::new();
    let mut cutscenes = Vec::new();
    let mut misc = Vec::new();
    
    for (key, value) in assets {
        let parts: Vec<_> = key.iter()
            .map(|part| part.to_str())
            .collect();
        let file_name = parts.last()
            .expect("Path should have at least one part")
            .unwrap_or("");
        
        if parts.len() == 2
            && parts[0] == Some("tilesets")
            && is_range_with_affixes(file_name, "tileset", ".png", 0..=255)
        {
            tilesets.push(value);
        }
        else if parts.len() == 2
            && parts[0] == Some("gradients")
            && is_range_with_affixes(file_name, "gradient", ".png", 0..=255)
        {
            gradients.push(value);
        }
        else if parts.len() == 2
            && parts[0] == Some("music")
            && (is_range_with_affixes(file_name, "song", ".ogg", 0..=255)
                || is_range_with_affixes(file_name, "intro", ".ogg", 0..=255))
        {
            songs.push(value);
        }
        else if parts.len() == 2
            && parts[0] == Some("ambiance")
            && is_range_with_affixes(file_name, "ambi", ".ogg", 0..=255)
        {
            ambiance.push(value);
        }
        else if parts.len() >= 2
            && parts[0] == Some("sounds")
            && key.extension().and_then(|ext| ext.to_str()) == Some("ogg")
        {
            sounds.push(value);
        }
        else if parts.len() >= 2
            && parts[0] == Some("custom objects")
            && key.extension().and_then(|ext| ext.to_str()) == Some("png")
        {
            custom_objects.push(value);
        }
        else if is_range_with_affixes(file_name, "scene", ".png", 0..)
            || is_range_with_affixes(file_name, "sound", ".ogg", 0..)
        {
            cutscenes.push(value);
        }
        else {
            misc.push(value);
        }
    }
    
    UnusedAssets{
        tilesets,
        gradients,
        songs,
        ambiance,
        sounds,
        custom_objects,
        cutscenes,
        misc,
        ignored,
    }
}

fn remove_well_known_assets(assets: &mut HashMap<PathBuf, PathBuf>) {
    for file in [
        "defaultsavegame.ini",
        "icon.png",
        "info.png",
        "info+.png",
        "map.bin",
        "world.ini",
        "custom objects/powericon0.png",
        "custom objects/powericon0.png",
        "custom objects/powericon1.png",
        "custom objects/powericon2.png",
        "custom objects/powericon3.png",
        "custom objects/powericon4.png",
        "custom objects/powericon5.png",
        "custom objects/powericon6.png",
        "custom objects/powericon7.png",
        "custom objects/powericon8.png",
        "custom objects/powericon9.png",
        "custom objects/powericon10.png",
        "custom objects/powericon11.png",
        "custom objects/powericon12.png",
        "custom objects/coinicon.png",
        "custom objects/artifacticon.png",
        "custom objects/creatureicon.png",
    ] {
        assets.remove(&PathBuf::from(file));
    }
    remove_cutscene_files(assets, "intro".as_ref());
    remove_cutscene_files(assets, "ending".as_ref());
}

fn analyze_map(world_path: &Path) -> anyhow::Result<(IndexedAssets, CustomObjectsUsed)> {
    let map_path = world_path.join("Map.bin");
    let screens = libks::map_bin::parse_map_file(&map_path)
        .with_context(|| format!("Failed to parse map file at {:?}", map_path))?;
    
    let mut tilesets = [false; 256];
    let mut gradients = [false; 256];
    let mut songs = [false; 256];
    let mut ambiance = [false; 256];
    let mut bank_255 = [false; 256];
    let mut bank_254 = [false; 256];
    
    for screen in screens {
        let mut uses_tileset_a = false;
        let mut uses_tileset_b = false;
        
        for LayerData(layer) in &screen.layers[..=3] {
            for tile in layer {
                uses_tileset_a |= tile.0 == 0 && tile.1 > 0;
                uses_tileset_b |= tile.0 == 1 && tile.1 > 0;
            }
        }
        
        for LayerData(layer) in &screen.layers[4..] {
            for tile in layer {
                if tile.1 == 0 { continue; }
                match tile.0 {
                    255 => bank_255[tile.1 as usize] = true,
                    254 => bank_254[tile.1 as usize] = true,
                    _ => {}
                };
            }
        }
        
        tilesets[screen.assets.tileset_a as usize] |= uses_tileset_a;
        tilesets[screen.assets.tileset_b as usize] |= uses_tileset_b;
        gradients[screen.assets.gradient as usize] = true;
        songs[screen.assets.music as usize] = true;
        ambiance[screen.assets.ambiance_a as usize] = true;
        ambiance[screen.assets.ambiance_b as usize] = true;
    }
    
    let indexed_assets = IndexedAssets {
        tilesets,
        gradients,
        songs,
        ambiance,
    };
    let custom_objects_used = CustomObjectsUsed {
        bank_255,
        bank_254
    };
    
    Ok((indexed_assets, custom_objects_used))
}

fn remove_indexed_assets(assets: &mut HashMap<PathBuf, PathBuf>, map_assets: &IndexedAssets) {
    for i in 0..=255 {
        if map_assets.tilesets[i] {
            assets.remove(&PathBuf::from(format!("tilesets/tileset{i}.png")));
        }
        if map_assets.gradients[i] {
            assets.remove(&PathBuf::from(format!("gradients/gradient{i}.png")));
        }
        if map_assets.songs[i] {
            assets.remove(&PathBuf::from(format!("music/song{i}.ogg")));
            assets.remove(&PathBuf::from(format!("music/intro{i}.ogg")));
        }
        if map_assets.ambiance[i] {
            assets.remove(&PathBuf::from(format!("ambiance/ambi{i}.ogg")));
        }
    }
}

fn analyze_ini(world_path: &Path, custom_objects_used: &CustomObjectsUsed) -> anyhow::Result<NamedAssets> {
    let mut cutscenes = HashSet::new();
    let mut custom_objects = HashSet::new();
    let mut sounds = HashSet::new();
    let mut songs = HashSet::new();
    let mut ambiance = HashSet::new();
    let mut savegames = HashSet::new();
    let ini_path = world_path.join("World.ini");
    let ini = libks::world_ini::load_ini(&ini_path)
        .with_context(|| format!("Failed to parse ini at {:?}", ini_path))?;
    
    for section in ini.iter_sections() {
        let Some(section_type) = interpret_world_ini_section(section.key()) else {
            continue;
        };
        match section_type {
            WorldIniSection::Screen(..) => {
                if let Some(value) = section.get("Ending") {
                    cutscenes.insert(value.to_owned());
                }
                if let Some(value) = section.get("ShiftCutscene(A)") {
                    cutscenes.insert(value.to_owned());
                }
                if let Some(value) = section.get("ShiftCutscene(B)") {
                    cutscenes.insert(value.to_owned());
                }
                if let Some(value) = section.get("ShiftCutscene(C)") {
                    cutscenes.insert(value.to_owned());
                }
                if let Some(value) = section.get("ShiftSave(A)")
                    && !value.is_empty()
                    && !value.eq_ignore_ascii_case("true")
                    && !value.eq_ignore_ascii_case("false")
                {
                    savegames.insert(value.to_owned());
                }
                if let Some(value) = section.get("ShiftSave(B)")
                    && !value.is_empty()
                    && !value.eq_ignore_ascii_case("true")
                    && !value.eq_ignore_ascii_case("false")
                {
                    savegames.insert(value.to_owned());
                }
                if let Some(value) = section.get("ShiftSave(C)")
                    && !value.is_empty()
                    && !value.eq_ignore_ascii_case("true")
                    && !value.eq_ignore_ascii_case("false")
                {
                    savegames.insert(value.to_owned());
                }
                if let Some(value) = section.get("ShiftSound(A)") {
                    sounds.insert(value.to_owned());
                }
                if let Some(value) = section.get("ShiftSound(B)") {
                    sounds.insert(value.to_owned());
                }
                if let Some(value) = section.get("ShiftSound(C)") {
                    sounds.insert(value.to_owned());
                }
                if let Some(value) = section.get("TrigSound(A)") {
                    sounds.insert(value.to_owned());
                }
                if let Some(value) = section.get("TrigSound(B)") {
                    sounds.insert(value.to_owned());
                }
                if let Some(value) = section.get("TrigSound(C)") {
                    sounds.insert(value.to_owned());
                }
                if let Some(value) = section.get("Attach") {
                    custom_objects.insert(value.to_owned());
                }
            }
            WorldIniSection::CustomObject(bank, index) => {
                let object_appears_in_level = match bank {
                    255 => custom_objects_used.bank_255[index as usize],
                    254 => custom_objects_used.bank_254[index as usize],
                    _ => false,
                };
                if !object_appears_in_level {
                    continue;
                }
                if let Some(value) = section.get("Image") {
                    custom_objects.insert(value.to_owned());
                }
            }
            WorldIniSection::CutsceneMusic => {
                for (_, value) in section.iter() {
                    match parse_cutscene_music_index(value) {
                        CutsceneMusic::Song(s) => songs.insert(s),
                        CutsceneMusic::Ambi(s) => ambiance.insert(s),
                    };
                }
            }
            WorldIniSection::CustomCharacter => {
                for (key, value) in section.iter() {
                    if is_range_with_affixes(key, "Image", "", 1..=255) {
                        custom_objects.insert(value.to_owned());
                        custom_objects.insert(format!("U{value}"));
                    }
                }
            }
            WorldIniSection::World => {
                if let Some(value) = section.get("Coin") {
                    custom_objects.insert(value.to_owned());
                }
                if let Some(value) = section.get("Powers") {
                    custom_objects.insert(value.to_owned());
                }
                if let Some(value) = section.get("Artifact1") {
                    custom_objects.insert(value.to_owned());
                }
                if let Some(value) = section.get("Artifact2") {
                    custom_objects.insert(value.to_owned());
                }
                if let Some(value) = section.get("Artifact3") {
                    custom_objects.insert(value.to_owned());
                }
                if let Some(value) = section.get("Artifact4") {
                    custom_objects.insert(value.to_owned());
                }
                if let Some(value) = section.get("Artifact5") {
                    custom_objects.insert(value.to_owned());
                }
                if let Some(value) = section.get("Artifact6") {
                    custom_objects.insert(value.to_owned());
                }
                if let Some(value) = section.get("Artifact7") {
                    custom_objects.insert(value.to_owned());
                }
                if let Some(value) = section.get("Title") {
                    custom_objects.insert(value.to_owned());
                }
                if let Some(value) = section.get("Subtitle") {
                    custom_objects.insert(value.to_owned());
                }
                if let Some(value) = section.get("Font") {
                    custom_objects.insert(value.to_owned());
                }
                if let Some(value) = section.get("Sign") {
                    custom_objects.insert(value.to_owned());
                }
            }
        };
    }
    
    // Helper functions
    fn normalize_cutscene(mut s: String, world_path: &Path) -> Option<PathBuf> {
        s.make_ascii_lowercase();
        let path = PathBuf::from(s);
        simplify_path(path, world_path)
    }
    fn normalize_custom_object(mut s: String, world_path: &Path) -> Option<PathBuf> {
        s.make_ascii_lowercase();
        let path = PathBuf::from(format!("custom objects/{s}"));
        simplify_path(path, world_path)
    }
    fn normalize_sound(mut s: String, world_path: &Path) -> Option<PathBuf> {
        s.make_ascii_lowercase();
        let path = PathBuf::from(format!("sounds/{s}.ogg"));
        simplify_path(path, world_path)
    }
    fn normalize_song(mut s: String, _world_path: &Path) -> Option<PathBuf> {
        s.make_ascii_lowercase();
        let path = PathBuf::from(format!("music/song{s}.ogg"));
        Some(path)
    }
    fn normalize_ambiance(mut s: String, _world_path: &Path) -> Option<PathBuf> {
        s.make_ascii_lowercase();
        let path = PathBuf::from(format!("ambiance/ambi{s}.ogg"));
        Some(path)
    }
    fn normalize_savegame(mut s: String, world_path: &Path) -> Option<PathBuf> {
        s.make_ascii_lowercase();
        let path = PathBuf::from(format!("savegame{s}.ini"));
        simplify_path(path, world_path)
    }
    
    let cutscenes_normalized: Vec<_> = cutscenes.into_iter()
        .filter_map(|p| normalize_cutscene(p, world_path))
        .collect();
    let custom_objects_normalized: Vec<_> = custom_objects.into_iter()
        .filter_map(|p| normalize_custom_object(p, world_path))
        .collect();
    let sounds_normalized: Vec<_> = sounds.into_iter()
        .filter_map(|p| normalize_sound(p, world_path))
        .collect();
    let songs_normalized: Vec<_> = songs.into_iter()
        .filter_map(|p| normalize_song(p, world_path))
        .collect();
    let ambiance_normalized: Vec<_> = ambiance.into_iter()
        .filter_map(|p| normalize_ambiance(p, world_path))
        .collect();
    let savegames_normalized: Vec<_> = savegames.into_iter()
        .filter_map(|p| normalize_savegame(p, world_path))
        .collect();
    
    Ok(NamedAssets {
        cutscenes: cutscenes_normalized,
        custom_objects: custom_objects_normalized,
        sounds: sounds_normalized,
        songs: songs_normalized,
        ambiance: ambiance_normalized,
        savegames: savegames_normalized,
    })
}

fn remove_named_assets(assets: &mut HashMap<PathBuf, PathBuf>, ini_assets: &NamedAssets) {
    for path in &ini_assets.cutscenes {
        remove_cutscene_files(assets, path);
    }
    for path in ini_assets.custom_objects.iter()
        .chain(ini_assets.sounds.iter())
        .chain(ini_assets.songs.iter())
        .chain(ini_assets.ambiance.iter())
        .chain(ini_assets.savegames.iter())
    {
        assets.remove(path);
    }
}

/// cutscene_name must be lowercase
fn remove_cutscene_files(assets: &mut HashMap<PathBuf, PathBuf>, cutscene_path: &Path) {
    for i in 1.. {
        let scene = format!("scene{i}.png");
        match assets.remove(&cutscene_path.join(scene)) {
            Some(_) => {
                let sound = format!("sound{i}.ogg");
                assets.remove(&cutscene_path.join(sound));
            }
            None => break
        };
    }
}

/// Returns None if rel_path is outside of the base directory
fn simplify_path(rel_path: PathBuf, base: &Path) -> Option<PathBuf> {
    
    fn is_special(part: &std::ffi::OsStr) -> bool {
        part.to_str()
            .is_some_and(|part| part.is_empty() || part.ends_with('.'))
    }
    
    if rel_path.iter().any(is_special) {
        let full_path = base.join(rel_path);
        let abs_path = std::path::absolute(full_path).expect("Path should not be empty or relative");
        match abs_path.strip_prefix(base) {
            Ok(rel_path) => Some(rel_path.to_owned()),
            Err(_) => None,
        }
    }
    else {
        Some(rel_path)
    }
}

#[allow(dead_code)]
#[derive(Debug, Copy, Clone, PartialEq)]
enum WorldIniSection {
    World,
    CutsceneMusic,
    CustomCharacter,
    CustomObject(u8, u8),
    Screen(f32, f32),
}

fn interpret_world_ini_section(key: &str) -> Option<WorldIniSection> {
    let key_lower = key.to_ascii_lowercase();
    if let Some(position) = libks::parse_xy(&key_lower) {
        Some(WorldIniSection::Screen(position.0, position.1))
    }
    else if let Some(index_str) = key_lower.strip_prefix("custom object ")
    {
        let (bank, index_str) = match index_str.strip_prefix('b') {
            Some(suffix) => (254, suffix),
            None => (255, index_str),
        };
        match str::parse::<u8>(index_str) {
            Ok(index @ 1..=255) => {
                Some(WorldIniSection::CustomObject(bank, index))
            },
            _ => None
        }
    }
    else if key_lower == "world" {
        Some(WorldIniSection::World)
    }
    else if key_lower == "cutscene music" {
        Some(WorldIniSection::CutsceneMusic)
    }
    else if key_lower == "custom character" {
        Some(WorldIniSection::CustomCharacter)
    }
    else {
        None
    }
}

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
enum CutsceneMusic {
    Song(String),
    Ambi(String),
}

fn parse_cutscene_music_index(s: &str) -> CutsceneMusic {
    let is_ambi = s.ends_with(['a', 'A']);
    match mmf_string_to_number(s) {
        Number::Int(val)   if is_ambi => CutsceneMusic::Ambi(val.to_string()),
        Number::Int(val)              => CutsceneMusic::Song(val.to_string()),
        Number::Float(val) if is_ambi => CutsceneMusic::Ambi(mmf_f32_to_string(val)),
        Number::Float(val)            => CutsceneMusic::Song(mmf_f32_to_string(val)),
    }
}

mod c {
    unsafe extern "C" {
        pub fn float_to_str(
            f: std::os::raw::c_float,
            buffer: *mut std::os::raw::c_char,
            buffer_size: usize,
        ) -> usize;
    }
}


pub fn mmf_f32_to_string(f: f32) -> String {
    match f {
        f32::INFINITY     => return String::from("1.#INF"),
        f32::NEG_INFINITY => return String::from("-1.#INF"),
        _ if f.is_nan()   => return String::from("1.$NaN"),
        _ => unsafe {
            let (buffer, _, capacity) = String::with_capacity(16).into_raw_parts();
            let length = c::float_to_str(f, buffer as *mut i8, capacity);
            String::from_raw_parts(buffer, length, capacity)
        }
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
enum Number {
    Int(i32),
    Float(f32),
}

fn mmf_string_to_number(mut s: &str) -> Number {
    #[derive(PartialEq, Eq)]
    enum ParseState {
        Start,
        AfterSign,
        Whole,
        Fraction,
        AfterE,
        AfterExpSign,
        Exp,
    }
    use ParseState::*;
    
    s = s.trim_ascii_start();
    let mut state = Start;
    let mut has_digit = false;
    let mut end = s.len();
    
    // Find the longest prefix that could be parsed as a float or integer
    // The potential patterns, determined empirically, are something like this
    // Integer: sign? digit+
    // Float:   sign? digit* . digit* ([eE] sign? digit*)? where there is at least 1 digit in the first or second group
    //          sign? digit+ [eE] sign? digit*
    for (i, ch) in s.char_indices() {
        state = match (state, ch) {
            (Start,        '+' | '-') => { AfterSign }
            (Start,        '0'..='9') => { has_digit = true; Whole }
            (Start,        '.')       => { Fraction }
            (AfterSign,    '0'..='9') => { has_digit = true; Whole }
            (AfterSign,    '.')       => { Fraction }
            (Whole,        '0'..='9') => { has_digit = true; Whole }
            (Whole,        '.')       => { Fraction }
            (Whole,        'e' | 'E') => { AfterE }
            (Fraction,     '0'..='9') => { has_digit = true; Fraction }
            (Fraction,     'e' | 'E') => { AfterE }
            (AfterE,       '+' | '-') => { AfterExpSign }
            (AfterE,       '0'..='9') => { Exp }
            (AfterExpSign, '0'..='9') => { Exp }
            (Exp,          '0'..='9') => { Exp }
            (final_state,  _)         => {
                end = i;
                state = final_state;
                break;
            }
        }
    }
    
    // Based on the final state, determine the substring that contains only parseable characters
    let parseable = match state {
        _ if !has_digit => "",
        Start           => "",
        AfterSign       => "",
        AfterE          => &s[..end - 1], // Drop the trailing e
        AfterExpSign    => &s[..end - 2], // Drop the trailing e and sign
        _               => &s[..end]
    };
    if parseable.is_empty() {
        return Number::Int(0);
    }
    
    // If there was no . or e, parse as integer
    let is_float = matches!(state, Fraction | AfterE | AfterExpSign | Exp);
    if !is_float {
        match i32::from_str(parseable) {
            Ok(val) => {
                // Oddly, there are 3 valid integers that MMF chooses to parse as floats instead of integers
                const MMF_INT_RANGE: std::ops::RangeInclusive<i32> = (i32::MIN + 2)..=(i32::MAX - 1);
                if MMF_INT_RANGE.contains(&val) {
                    return Number::Int(val);
                }
            }
            Err(err) => match err.kind() {
                // If there is overflow, we'll try parsing as a float instead
                std::num::IntErrorKind::PosOverflow => {}
                std::num::IntErrorKind::NegOverflow => {}
                // Should be unreachable since we already parsed the string, but default to 0 just in case
                _ => return Number::Int(0)
            }
        };
    }
    
    // If there was a . or e, or the integer was out of range, parse as float
    match f32::from_str(parseable) {
        Ok(val) => Number::Float(val),
        // Should be unreachable since we already parsed the string, but default to 0 just in case
        Err(_) => Number::Int(0)
    }
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use super::*;
    
    #[test]
    fn simplify_path_basic_path_is_unchanged() {
        let base = PathBuf::from(r"D:\Games\Knytt Stories\Worlds\Nifflas - The Machine");
        let rel_path = PathBuf::from(r"Custom Objects\Juni.png");
        let expected = Some(rel_path.clone());
        assert_eq!(simplify_path(rel_path, &base), expected);
    }
    
    #[test]
    fn simplify_path_removes_empty_components() {
        let base = PathBuf::from(r"D:\Games\Knytt Stories\Worlds\Nifflas - The Machine");
        let rel_path = PathBuf::from(r"Custom Objects\\Juni.png");
        let expected = Some(PathBuf::from(r"Custom Objects\Juni.png"));
        assert_eq!(simplify_path(rel_path, &base), expected);
    }
    
    #[test]
    fn simplify_path_removes_current_dir_components() {
        let base = PathBuf::from(r"D:\Games\Knytt Stories\Worlds\Nifflas - The Machine");
        let rel_path = PathBuf::from(r"Custom Objects\.\Juni.png");
        let expected = Some(PathBuf::from(r"Custom Objects\Juni.png"));
        assert_eq!(simplify_path(rel_path, &base), expected);
    }
    
    #[test]
    fn simplify_path_removes_parent_dir_components() {
        let base = PathBuf::from(r"D:\Games\Knytt Stories\Worlds\Nifflas - The Machine");
        let rel_path = PathBuf::from(r"Custom Objects\..\Custom Objects\Juni.png");
        let expected = Some(PathBuf::from(r"Custom Objects\Juni.png"));
        assert_eq!(simplify_path(rel_path, &base), expected);
    }
    
    #[test]
    fn simplify_path_handles_excessive_parent_dir_components() {
        let base = PathBuf::from(r"D:\Games\Knytt Stories\Worlds\Nifflas - The Machine");
        let rel_path = PathBuf::from(r"..\..\..\..\..\..\..\Games\Knytt Stories\Worlds\Nifflas - The Machine\Custom Objects\Juni.png");
        let expected = Some(PathBuf::from(r"Custom Objects\Juni.png"));
        assert_eq!(simplify_path(rel_path, &base), expected);
    }
    
    #[test]
    fn simplify_path_strips_trailing_dots() {
        let base = PathBuf::from(r"D:\Games\Knytt Stories\Worlds\Nifflas - The Machine");
        let rel_path = PathBuf::from(r"Custom Objects\Juni.png......");
        let expected = Some(PathBuf::from(r"Custom Objects\Juni.png"));
        assert_eq!(simplify_path(rel_path, &base), expected);
    }
    
    #[test]
    fn simplify_path_strips_single_trailing_dot_from_directories() {
        let base = PathBuf::from(r"D:\Games\Knytt Stories\Worlds\Nifflas - The Machine");
        {
            let rel_path = PathBuf::from(r"Custom Objects.\Juni.png");
            let expected = Some(PathBuf::from(r"Custom Objects\Juni.png"));
            assert_eq!(simplify_path(rel_path, &base), expected);
        }
        {
            let rel_path = PathBuf::from(r"Custom Objects..\Juni.png");
            let expected = Some(PathBuf::from(r"Custom Objects..\Juni.png"));
            assert_eq!(simplify_path(rel_path, &base), expected);
        }
    }
    
    #[test]
    fn simplify_path_handles_multiple_unusual_features() {
        let base = PathBuf::from(r"D:\Games\Knytt Stories\Worlds\Nifflas - The Machine");
        let rel_path = PathBuf::from(r"Custom Objects\..\..\Nifflas - A Strange Dream\.\Ending\..\..\Nifflas - The Machine\Custom Objects\\Juni.png");
        let expected = Some(PathBuf::from(r"Custom Objects\Juni.png"));
        assert_eq!(simplify_path(rel_path, &base), expected);
    }
    
    #[test]
    fn simplify_path_file_outside_directory_returns_none() {
        let base = PathBuf::from(r"D:\Games\Knytt Stories\Worlds\Nifflas - The Machine");
        let rel_path = PathBuf::from(r"..\Juni.png");
        let expected = None;
        assert_eq!(simplify_path(rel_path, &base), expected);
    }
    
    #[test]
    fn interpret_world_ini_section_screen() {
        let key = "x1000y1000";
        let expected = Some(WorldIniSection::Screen(1000.0, 1000.0));
        let actual = interpret_world_ini_section(key);
        assert_eq!(actual, expected);
    }
    
    #[test]
    fn interpret_world_ini_section_screen_with_negative_coords() {
        let key = "x-1000y-1000";
        let expected = Some(WorldIniSection::Screen(-1000.0, -1000.0));
        let actual = interpret_world_ini_section(key);
        assert_eq!(actual, expected);
    }
    
    #[test]
    fn interpret_world_ini_section_custom_object() {
        let key = "Custom Object 33";
        let expected = Some(WorldIniSection::CustomObject(255, 33));
        let actual = interpret_world_ini_section(key);
        assert_eq!(actual, expected);
    }
    
    #[test]
    fn interpret_world_ini_section_custom_object_b() {
        let key = "Custom Object B33";
        let expected = Some(WorldIniSection::CustomObject(254, 33));
        let actual = interpret_world_ini_section(key);
        assert_eq!(actual, expected);
    }
    
    #[test]
    fn interpret_world_ini_section_rejects_out_of_range_custom_object_indices() {
        let key = "Custom Object 0";
        let expected = None;
        let actual = interpret_world_ini_section(key);
        assert_eq!(actual, expected);
        
        let key = "Custom Object B256";
        let expected = None;
        let actual = interpret_world_ini_section(key);
        assert_eq!(actual, expected);
    }
    
    #[test]
    fn interpret_world_ini_section_custom_character() {
        let key = "Custom Character";
        let expected = Some(WorldIniSection::CustomCharacter);
        let actual = interpret_world_ini_section(key);
        assert_eq!(actual, expected);
    }
    
    #[test]
    fn interpret_world_ini_section_cutscene_music() {
        let key = "Cutscene Music";
        let expected = Some(WorldIniSection::CutsceneMusic);
        let actual = interpret_world_ini_section(key);
        assert_eq!(actual, expected);
    }
    
    #[test]
    fn interpret_world_ini_section_world() {
        let key = "World";
        let expected = Some(WorldIniSection::World);
        let actual = interpret_world_ini_section(key);
        assert_eq!(actual, expected);
    }
    
    #[test]
    fn interpret_world_ini_section_ignores_case() {
        let key = "world";
        let expected = Some(WorldIniSection::World);
        let actual = interpret_world_ini_section(key);
        assert_eq!(actual, expected);
        
        let key = "X1000Y1000";
        let expected = Some(WorldIniSection::Screen(1000.0, 1000.0));
        let actual = interpret_world_ini_section(key);
        assert_eq!(actual, expected);
        
        let key = "CuStOm ObJeCt b33";
        let expected = Some(WorldIniSection::CustomObject(254, 33));
        let actual = interpret_world_ini_section(key);
        assert_eq!(actual, expected);
    }
    
    #[test]
    fn mmf_f32_to_string_works() {
        for (input, expected) in [
            (       999.001f32, "999.001"),
            (      999.0001f32, "999"),
            (       1000.01f32, "1000.01"),
            (      1000.001f32, "1000"),
            (      999999.0f32, "999999"),
            (     1000000.0f32, "1e+006"),
            (     1000001.0f32, "1e+006"),
            (     1000010.0f32, "1.00001e+006"),
            (     1000100.0f32, "1.0001e+006"),
            (     -999999.0f32, "-999999"),
            (    -1000000.0f32, "-1e+006"),
            (  2147483647.0f32, "2.14748e+009"),
            (    f32::INFINITY, "1.#INF"),
            (f32::NEG_INFINITY, "-1.#INF"),
            (         f32::NAN, "1.$NaN")
        ] {
            let actual = mmf_f32_to_string(input);
            assert_eq!(actual.as_str(), expected);
        }
    }
    
    #[test]
    fn mmf_string_to_number_works() {
        for (input, expected) in [
            // Invalid
            ("",            Number::Int(0)),
            ("+",           Number::Int(0)),
            ("-",           Number::Int(0)),
            (".",           Number::Int(0)),
            ("e3",          Number::Int(0)),
            ("knytt",       Number::Int(0)),
            // Integers
            ("123",         Number::Int(123)),
            ("+123",        Number::Int(123)),
            ("-123",        Number::Int(-123)),
            // Max integer range
            ("2147483646",  Number::Int(2147483646)),
            ("2147483647",  Number::Float(2147483647.0)),  // float value is approximate
            ("2147483648",  Number::Float(2147483648.0)),  // float value is approximate
            // Min integer range
            ("-2147483646", Number::Int(-2147483646)),
            ("-2147483647", Number::Float(-2147483647.0)), // float value is approximate
            ("-2147483648", Number::Float(-2147483648.0)), // float value is approximate
            ("-2147483649", Number::Float(-2147483649.0)), // float value is approximate
            // Floats
            ("123.456",     Number::Float(123.456)),
            ("123.",        Number::Float(123.0)),
            (".456",        Number::Float(0.456)),
            ("+123.456",    Number::Float(123.456)),
            ("+123.",       Number::Float(123.0)),
            ("+.456",       Number::Float(0.456)),
            ("-123.456",    Number::Float(-123.456)),
            ("-123.",       Number::Float(-123.0)),
            ("-.456",       Number::Float(-0.456)),
            // Exponents
            ("123.456e",    Number::Float(123.456)),
            ("123.e",       Number::Float(123.0)),
            ("123e",        Number::Float(123.0)),
            ("123.456E",    Number::Float(123.456)),
            ("123.E",       Number::Float(123.0)),
            ("123E",        Number::Float(123.0)),
            ("123.456e3",   Number::Float(123456.0)),
            ("123.e3",      Number::Float(123000.0)),
            ("123e3",       Number::Float(123000.0)),
            ("123.456e+3",  Number::Float(123456.0)),
            ("123.456e-3",  Number::Float(0.123456)),
        ] {
            let actual = mmf_string_to_number(input);
            assert_eq!(Number::from(actual), expected, "while parsing {input}");
        }
    }
    
    #[test]
    fn parse_cutscene_music_index_works() {
        for (input, expected) in [
            ("0",           CutsceneMusic::Song("0".to_string())),
            ("-1",          CutsceneMusic::Song("-1".to_string())),
            ("256",         CutsceneMusic::Song("256".to_string())),
            ("0a",          CutsceneMusic::Ambi("0".to_string())),
            ("-1a",         CutsceneMusic::Ambi("-1".to_string())),
            ("256A",        CutsceneMusic::Ambi("256".to_string())),
            ("",            CutsceneMusic::Song("0".to_string())),
            ("a",           CutsceneMusic::Ambi("0".to_string())),
            ("10xyz",       CutsceneMusic::Song("10".to_string())),
            ("10xyza",      CutsceneMusic::Ambi("10".to_string())),
            ("xyz10",       CutsceneMusic::Song("0".to_string())),
            ("xyz10a",      CutsceneMusic::Ambi("0".to_string())),
            ("2147483646",  CutsceneMusic::Song("2147483646".to_string())),
            ("2147483647",  CutsceneMusic::Song("2.14748e+009".to_string())),
            ("-2147483646", CutsceneMusic::Song("-2147483646".to_string())),
            ("-2147483647", CutsceneMusic::Song("-2.14748e+009".to_string())),
        ] {
            let actual = parse_cutscene_music_index(input);
            assert_eq!(actual, expected);
        }
    }
}
