use std::{collections::HashMap, env, fs, path::PathBuf};

use bevy::prelude::{IVec3, Resource, Transform};
use serde::{Deserialize, Serialize};

use crate::{
    interaction::{HOTBAR_SLOTS, Hotbar},
    world::{Block, WorldData},
};

const SAVE_VERSION: u32 = 1;
pub const MAX_SAVE_SLOTS: usize = 3;
pub const MAX_SAVE_NAME_CHARS: usize = 24;

pub const DEFAULT_MOUSE_SENSITIVITY: f32 = 0.0022;
pub const MIN_MOUSE_SENSITIVITY: f32 = 0.0001;
pub const MAX_MOUSE_SENSITIVITY: f32 = 0.02;
pub const DEFAULT_GROUND_SHIFT_SPEED: f32 = 8.0;
pub const DEFAULT_FLYING_SHIFT_SPEED: f32 = 12.0;
pub const MIN_SHIFT_SPEED: f32 = 5.0;
pub const MAX_GROUND_SHIFT_SPEED: f32 = 20.0;
pub const MAX_FLYING_SHIFT_SPEED: f32 = 40.0;
pub const DEFAULT_DAY_NIGHT_SPEED: f32 = 1.0;
pub const MIN_DAY_NIGHT_SPEED: f32 = 0.25;
pub const MAX_DAY_NIGHT_SPEED: f32 = 4.0;

#[derive(Resource)]
pub struct SaveStatus(pub String);

impl Default for SaveStatus {
    fn default() -> Self {
        Self("Your world saves automatically when you return to the main menu.".to_owned())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveFile {
    version: u32,
    #[serde(default)]
    pub name: String,
    #[serde(alias = "seed")]
    pub terrain_seed: u32,
    pub player_position: [f32; 3],
    pub player_rotation: [f32; 4],
    #[serde(default)]
    pub was_flying: bool,
    pub hotbar_blocks: [u8; HOTBAR_SLOTS],
    pub hotbar_selected: usize,
    pub edits: Vec<SavedEdit>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SavedEdit {
    pub position: [i32; 3],
    pub block: u8,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
struct UserSettingsFile {
    mouse_sensitivity: f32,
    ground_shift_speed: f32,
    flying_shift_speed: f32,
    day_night_speed: f32,
}

impl Default for UserSettingsFile {
    fn default() -> Self {
        Self {
            mouse_sensitivity: DEFAULT_MOUSE_SENSITIVITY,
            ground_shift_speed: DEFAULT_GROUND_SHIFT_SPEED,
            flying_shift_speed: DEFAULT_FLYING_SHIFT_SPEED,
            day_night_speed: DEFAULT_DAY_NIGHT_SPEED,
        }
    }
}

impl SaveFile {
    pub fn capture(
        world: &WorldData,
        player: &Transform,
        hotbar: &Hotbar,
        was_flying: bool,
        name: String,
    ) -> Self {
        let mut edits: Vec<_> = world
            .overrides
            .iter()
            .map(|(position, block)| SavedEdit {
                position: position.to_array(),
                block: *block as u8,
            })
            .collect();
        edits.sort_unstable_by_key(|edit| edit.position);

        Self {
            version: SAVE_VERSION,
            name,
            terrain_seed: world.seed,
            player_position: player.translation.to_array(),
            player_rotation: player.rotation.to_array(),
            was_flying,
            hotbar_blocks: hotbar.slots.map(|block| block as u8),
            hotbar_selected: hotbar.selected,
            edits,
        }
    }

    pub fn decoded_hotbar(&self) -> Result<[Block; HOTBAR_SLOTS], String> {
        let mut blocks = [Block::Grass; HOTBAR_SLOTS];
        for (index, id) in self.hotbar_blocks.iter().copied().enumerate() {
            blocks[index] = Block::from_id(id)
                .ok_or_else(|| format!("Save contains unknown hotbar block id {id}."))?;
        }
        Ok(blocks)
    }

    pub fn decoded_edits(&self) -> Result<HashMap<IVec3, Block>, String> {
        let mut edits = HashMap::with_capacity(self.edits.len());
        for edit in &self.edits {
            let block = Block::from_id(edit.block)
                .ok_or_else(|| format!("Save contains unknown block id {}.", edit.block))?;
            edits.insert(IVec3::from_array(edit.position), block);
        }
        Ok(edits)
    }

    fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty()
            || self.name.chars().count() > MAX_SAVE_NAME_CHARS
            || self.name.chars().any(char::is_control)
        {
            return Err(format!(
                "Save name must contain 1 to {MAX_SAVE_NAME_CHARS} printable characters."
            ));
        }
        if self.version != SAVE_VERSION {
            return Err(format!(
                "This save uses version {}, but VoxCraft supports version {}.",
                self.version, SAVE_VERSION
            ));
        }
        if !self.player_position.iter().all(|value| value.is_finite())
            || !self.player_rotation.iter().all(|value| value.is_finite())
            || self
                .player_rotation
                .iter()
                .map(|value| value * value)
                .sum::<f32>()
                < 0.001
        {
            return Err("Save has invalid player position or rotation data.".to_owned());
        }
        if self.hotbar_selected >= HOTBAR_SLOTS {
            return Err("Save has an invalid selected hotbar slot.".to_owned());
        }
        self.decoded_hotbar()?;
        self.decoded_edits()?;
        Ok(())
    }
}

pub fn save_to_disk(slot: usize, save: &SaveFile) -> Result<(), String> {
    save.validate()?;
    let path = save_file_path(slot)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Could not create save folder: {error}"))?;
    }
    let json = serde_json::to_vec_pretty(save)
        .map_err(|error| format!("Could not encode save: {error}"))?;
    fs::write(path, json).map_err(|error| format!("Could not write save: {error}"))
}

pub fn load_from_disk(slot: usize) -> Result<SaveFile, String> {
    let path = save_file_path(slot)?;
    let path = if path.exists() {
        path
    } else if slot == 1 && legacy_save_file_path().exists() {
        legacy_save_file_path()
    } else {
        path
    };
    let json = fs::read_to_string(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            format!("Save slot {slot} is empty.")
        } else {
            format!("Could not read save: {error}")
        }
    })?;
    let mut save: SaveFile = serde_json::from_str(&json)
        .map_err(|error| format!("Save file could not be read: {error}"))?;
    save.name = normalize_save_name(&save.name, slot);
    save.validate()?;
    Ok(save)
}

pub fn default_save_name(slot: usize) -> String {
    format!("World {slot}")
}

pub fn normalize_save_name(name: &str, slot: usize) -> String {
    let normalized: String = name
        .trim()
        .chars()
        .filter(|character| !character.is_control())
        .take(MAX_SAVE_NAME_CHARS)
        .collect::<String>()
        .trim()
        .to_owned();
    if normalized.is_empty() {
        default_save_name(slot)
    } else {
        normalized
    }
}

pub fn save_slot_exists(slot: usize) -> bool {
    if !(1..=MAX_SAVE_SLOTS).contains(&slot) {
        return false;
    }
    save_file_path(slot).is_ok_and(|path| path.exists())
        || (slot == 1 && legacy_save_file_path().exists())
}

pub fn delete_save_slot(slot: usize) -> Result<(), String> {
    let mut paths = vec![save_file_path(slot)?];
    if slot == 1 {
        paths.push(legacy_save_file_path());
    }

    let mut removed = false;
    for path in paths {
        match fs::remove_file(&path) {
            Ok(()) => removed = true,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("Could not delete save file: {error}")),
        }
    }
    if removed {
        Ok(())
    } else {
        Err(format!("Save slot {slot} is empty."))
    }
}

pub fn load_mouse_sensitivity() -> Option<f32> {
    let settings = load_user_settings_file()?;
    settings.mouse_sensitivity.is_finite().then(|| {
        settings
            .mouse_sensitivity
            .clamp(MIN_MOUSE_SENSITIVITY, MAX_MOUSE_SENSITIVITY)
    })
}

pub fn load_shift_speeds() -> Option<(f32, f32)> {
    let settings = load_user_settings_file()?;
    let ground_speed = if settings.ground_shift_speed.is_finite() {
        settings
            .ground_shift_speed
            .clamp(MIN_SHIFT_SPEED, MAX_GROUND_SHIFT_SPEED)
    } else {
        DEFAULT_GROUND_SHIFT_SPEED
    };
    let flying_speed = if settings.flying_shift_speed.is_finite() {
        settings
            .flying_shift_speed
            .clamp(MIN_SHIFT_SPEED, MAX_FLYING_SHIFT_SPEED)
    } else {
        DEFAULT_FLYING_SHIFT_SPEED
    };
    Some((ground_speed, flying_speed))
}

pub fn load_day_night_speed() -> Option<f32> {
    let speed = load_user_settings_file()?.day_night_speed;
    Some(if speed.is_finite() {
        speed.clamp(MIN_DAY_NIGHT_SPEED, MAX_DAY_NIGHT_SPEED)
    } else {
        DEFAULT_DAY_NIGHT_SPEED
    })
}

pub fn save_user_settings(
    mouse_sensitivity: f32,
    ground_shift_speed: f32,
    flying_shift_speed: f32,
    day_night_speed: f32,
) -> Result<(), String> {
    if !mouse_sensitivity.is_finite()
        || !ground_shift_speed.is_finite()
        || !flying_shift_speed.is_finite()
        || !day_night_speed.is_finite()
    {
        return Err("User settings must be finite numbers.".to_owned());
    }
    let path = settings_file_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Could not create settings folder: {error}"))?;
    }
    let settings = UserSettingsFile {
        mouse_sensitivity: mouse_sensitivity.clamp(MIN_MOUSE_SENSITIVITY, MAX_MOUSE_SENSITIVITY),
        ground_shift_speed: ground_shift_speed.clamp(MIN_SHIFT_SPEED, MAX_GROUND_SHIFT_SPEED),
        flying_shift_speed: flying_shift_speed.clamp(MIN_SHIFT_SPEED, MAX_FLYING_SHIFT_SPEED),
        day_night_speed: day_night_speed.clamp(MIN_DAY_NIGHT_SPEED, MAX_DAY_NIGHT_SPEED),
    };
    let json = serde_json::to_vec_pretty(&settings)
        .map_err(|error| format!("Could not encode settings: {error}"))?;
    fs::write(path, json).map_err(|error| format!("Could not write settings: {error}"))
}

fn load_user_settings_file() -> Option<UserSettingsFile> {
    let json = fs::read_to_string(settings_file_path()).ok()?;
    serde_json::from_str(&json).ok()
}

fn save_file_path(slot: usize) -> Result<PathBuf, String> {
    if !(1..=MAX_SAVE_SLOTS).contains(&slot) {
        return Err(format!("Save slot must be between 1 and {MAX_SAVE_SLOTS}."));
    }
    Ok(save_data_dir()
        .join("saves")
        .join(format!("slot_{slot}.json")))
}

fn legacy_save_file_path() -> PathBuf {
    #[cfg(windows)]
    if let Some(root) = env::var_os("APPDATA") {
        return PathBuf::from(root).join("VoxCraft").join("save.json");
    }

    #[cfg(target_os = "macos")]
    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("VoxCraft")
            .join("save.json");
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    if let Some(root) = env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(root).join("voxcraft").join("save.json");
    } else if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("voxcraft")
            .join("save.json");
    }

    env::current_dir()
        .unwrap_or_default()
        .join("voxcraft_save.json")
}

fn save_data_dir() -> PathBuf {
    #[cfg(windows)]
    if let Some(root) = env::var_os("APPDATA") {
        return PathBuf::from(root).join("VoxCraft");
    }

    #[cfg(target_os = "macos")]
    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("VoxCraft");
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    if let Some(root) = env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(root).join("voxcraft");
    } else if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("voxcraft");
    }

    env::current_dir().unwrap_or_default().join("voxcraft")
}

fn settings_file_path() -> PathBuf {
    #[cfg(windows)]
    if let Some(root) = env::var_os("APPDATA") {
        return PathBuf::from(root).join("VoxCraft").join("settings.json");
    }

    #[cfg(target_os = "macos")]
    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("VoxCraft")
            .join("settings.json");
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    if let Some(root) = env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(root).join("voxcraft").join("settings.json");
    } else if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".config")
            .join("voxcraft")
            .join("settings.json");
    }

    env::current_dir()
        .unwrap_or_default()
        .join("voxcraft_settings.json")
}
