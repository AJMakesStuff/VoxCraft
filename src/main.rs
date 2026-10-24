mod game;
mod interaction;
mod player;
mod rendering;
mod ui;
mod world;

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game::{GamePlugin, GameSettings};

fn main() {
    let settings = GameSettings::from_args();
    App::new()
        .insert_resource(settings)
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "VoxCraft".to_owned(),
                resolution: (1280, 720).into(),
                present_mode: bevy::window::PresentMode::AutoNoVsync,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(GamePlugin)
        .add_systems(Startup, maximize_primary_window)
        .run();
}

fn maximize_primary_window(mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    if let Ok(mut window) = windows.single_mut() {
        window.set_maximized(true);
    }
}
