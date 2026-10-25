use bevy::{audio::Volume, prelude::*};

#[derive(Resource)]
pub(super) struct BlockSounds {
    pub(super) placement: Handle<AudioSource>,
    pub(super) breaking: Handle<AudioSource>,
}

pub(super) fn setup_block_sounds(asset_server: Res<AssetServer>, mut commands: Commands) {
    commands.insert_resource(BlockSounds {
        placement: asset_server.load("sounds/place.ogg"),
        breaking: asset_server.load("sounds/break.ogg"),
    });
}

pub(super) fn play_block_sound(commands: &mut Commands, sound: Handle<AudioSource>) {
    commands.spawn((
        AudioPlayer::new(sound),
        PlaybackSettings::DESPAWN.with_volume(Volume::Linear(0.35)),
    ));
}
