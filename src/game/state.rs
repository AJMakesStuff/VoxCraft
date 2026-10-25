use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use bevy::prelude::*;

use crate::{
    interaction::InteractionPlugin, player::PlayerPlugin, rendering::RenderingPlugin, ui::UiPlugin,
    world::WorldPlugin,
};

use super::save::{
    DEFAULT_FLYING_SHIFT_SPEED, DEFAULT_GROUND_SHIFT_SPEED, DEFAULT_MOUSE_SENSITIVITY,
    MAX_MOUSE_SENSITIVITY, MIN_MOUSE_SENSITIVITY, load_mouse_sensitivity, load_shift_speeds,
};

pub const DISPLAY_RESOLUTIONS: [(u32, u32); 5] = [
    (1280, 720),
    (1600, 900),
    (1920, 1080),
    (2560, 1440),
    (3840, 2160),
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DisplayMode {
    #[default]
    Windowed,
    Borderless,
    Fullscreen,
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct GameSettings {
    pub seed: u32,
    seed_override: Option<u32>,
    pub render_distance: i32,
    pub mouse_sensitivity: f32,
    pub ground_shift_speed: f32,
    pub flying_shift_speed: f32,
    pub vsync_enabled: bool,
    pub adaptive_sync_enabled: bool,
    pub anti_aliasing_samples: u32,
    pub display_mode: DisplayMode,
    /// Zero selects the current monitor's native size; other values index DISPLAY_RESOLUTIONS.
    pub resolution_index: usize,
    pub field_of_view_degrees: f32,
    pub brightness: f32,
    pub contrast: f32,
    pub gamma: f32,
    pub hdr_enabled: bool,
}

impl GameSettings {
    pub fn from_args() -> Self {
        let (ground_shift_speed, flying_shift_speed) =
            load_shift_speeds().unwrap_or((DEFAULT_GROUND_SHIFT_SPEED, DEFAULT_FLYING_SHIFT_SPEED));
        let mut settings = Self {
            seed: fresh_seed(),
            seed_override: None,
            render_distance: 8,
            mouse_sensitivity: load_mouse_sensitivity().unwrap_or(DEFAULT_MOUSE_SENSITIVITY),
            ground_shift_speed,
            flying_shift_speed,
            vsync_enabled: false,
            adaptive_sync_enabled: false,
            anti_aliasing_samples: 4,
            display_mode: DisplayMode::Windowed,
            resolution_index: 0,
            field_of_view_degrees: 75.0,
            brightness: 1.0,
            contrast: 1.0,
            gamma: 1.0,
            hdr_enabled: false,
        };
        for arg in std::env::args().skip(1) {
            if let Some(value) = arg.strip_prefix("--seed=") {
                if let Ok(seed) = value.parse() {
                    settings.seed = seed;
                    settings.seed_override = Some(seed);
                }
            } else if let Some(value) = arg.strip_prefix("--render-distance=")
                && let Ok(distance) = value.parse::<i32>()
            {
                settings.render_distance = distance.clamp(2, 16);
            } else if let Some(value) = arg.strip_prefix("--mouse-sensitivity=")
                && let Ok(sensitivity) = value.parse::<f32>()
            {
                settings.mouse_sensitivity = clamp_mouse_sensitivity(sensitivity);
            }
        }
        settings
    }

    pub fn prepare_new_world_seed(&mut self) {
        self.seed = self.seed_override.unwrap_or_else(fresh_seed);
    }

    pub fn adjust_render_distance(&mut self, delta: i32) {
        self.render_distance = self.render_distance.saturating_add(delta).clamp(2, 16);
    }

    pub fn step_anti_aliasing(&mut self, direction: i8) {
        self.anti_aliasing_samples = match (self.anti_aliasing_samples, direction.signum()) {
            (1, 1) | (4, -1) => 2,
            (2, 1) | (1, -1) => 4,
            (4, 1) | (2, -1) => 1,
            (samples, _) => samples,
        };
    }

    pub fn step_display_mode(&mut self, direction: i8) {
        let index = match self.display_mode {
            DisplayMode::Windowed => 0,
            DisplayMode::Borderless => 1,
            DisplayMode::Fullscreen => 2,
        };
        let next = (index + direction.signum() as i32).rem_euclid(3);
        self.display_mode = match next {
            0 => DisplayMode::Windowed,
            1 => DisplayMode::Borderless,
            _ => DisplayMode::Fullscreen,
        };
    }

    pub fn step_resolution(&mut self, direction: i8) {
        let count = DISPLAY_RESOLUTIONS.len() as i32 + 1;
        self.resolution_index =
            (self.resolution_index as i32 + direction.signum() as i32).rem_euclid(count) as usize;
    }
}

fn clamp_mouse_sensitivity(sensitivity: f32) -> f32 {
    if sensitivity.is_finite() {
        sensitivity.clamp(MIN_MOUSE_SENSITIVITY, MAX_MOUSE_SENSITIVITY)
    } else {
        DEFAULT_MOUSE_SENSITIVITY
    }
}

fn fresh_seed() -> u32 {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let mut value =
        time ^ ((std::process::id() as u64) << 32) ^ COUNTER.fetch_add(1, Ordering::Relaxed);
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^= value >> 31;
    value as u32
}

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameState {
    #[default]
    MainMenu,
    Playing,
    Paused,
    Options,
    Inventory,
}

#[derive(Resource, Default)]
pub struct OptionsReturn(pub GameState);

#[derive(Resource)]
pub struct DayNightCycle {
    pub(crate) phase: f32,
    pub(crate) day: u32,
}

impl Default for DayNightCycle {
    fn default() -> Self {
        Self {
            phase: 0.32,
            day: 1,
        }
    }
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .init_resource::<OptionsReturn>()
            .init_resource::<DayNightCycle>()
            .add_plugins((
                RenderingPlugin,
                WorldPlugin,
                PlayerPlugin,
                InteractionPlugin,
                UiPlugin,
            ));
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn extreme_render_distance_delta_clamps_without_overflow() {
        let mut settings = super::GameSettings::from_args();
        settings.render_distance = 8;

        settings.adjust_render_distance(i32::MAX);

        assert_eq!(settings.render_distance, 16);
    }

    #[test]
    fn mouse_sensitivity_nan_is_clamped_to_a_finite_value() {
        let sensitivity = super::clamp_mouse_sensitivity("NaN".parse::<f32>().unwrap());

        assert!(sensitivity.is_finite());
        assert!((0.0001..=0.02).contains(&sensitivity));
    }
}
