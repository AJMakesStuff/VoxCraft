use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use bevy::prelude::*;

use crate::{
    interaction::InteractionPlugin, player::PlayerPlugin, rendering::RenderingPlugin, ui::UiPlugin,
    world::WorldPlugin,
};

#[derive(Resource, Clone, Copy, Debug)]
pub struct GameSettings {
    pub seed: u32,
    seed_override: Option<u32>,
    pub render_distance: i32,
    pub mouse_sensitivity: f32,
}

impl GameSettings {
    pub fn from_args() -> Self {
        let mut settings = Self {
            seed: fresh_seed(),
            seed_override: None,
            render_distance: 8,
            mouse_sensitivity: 0.0022,
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
                settings.mouse_sensitivity = sensitivity.clamp(0.0001, 0.02);
            }
        }
        settings
    }

    pub fn prepare_new_world_seed(&mut self) {
        self.seed = self.seed_override.unwrap_or_else(fresh_seed);
    }

    pub fn adjust_mouse_sensitivity(&mut self, delta: f32) {
        if delta.is_finite() {
            self.mouse_sensitivity = (self.mouse_sensitivity + delta).clamp(0.0001, 0.02);
        }
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

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .init_resource::<OptionsReturn>()
            .add_plugins((
                RenderingPlugin,
                WorldPlugin,
                PlayerPlugin,
                InteractionPlugin,
                UiPlugin,
            ));
    }
}
