use std::f32::consts::TAU;

use bevy::{audio::Volume, prelude::*};

#[derive(Resource)]
pub(super) struct BlockSounds {
    pub(super) placement: Handle<AudioSource>,
    pub(super) breaking: Handle<AudioSource>,
}

pub(super) fn setup_block_sounds(
    mut audio_sources: ResMut<Assets<AudioSource>>,
    mut commands: Commands,
) {
    let placement = audio_sources.add(AudioSource {
        bytes: make_wav(SoundKind::Place).into(),
    });
    let breaking = audio_sources.add(AudioSource {
        bytes: make_wav(SoundKind::Break).into(),
    });
    commands.insert_resource(BlockSounds {
        placement,
        breaking,
    });
}

pub(super) fn play_block_sound(commands: &mut Commands, sound: Handle<AudioSource>) {
    commands.spawn((
        AudioPlayer::new(sound),
        PlaybackSettings::DESPAWN.with_volume(Volume::Linear(0.35)),
    ));
}

#[derive(Clone, Copy)]
enum SoundKind {
    Place,
    Break,
}

fn make_wav(kind: SoundKind) -> Vec<u8> {
    const SAMPLE_RATE: u32 = 22_050;
    let duration = match kind {
        SoundKind::Place => 0.11,
        SoundKind::Break => 0.16,
    };
    let sample_count = (SAMPLE_RATE as f32 * duration) as u32;
    let data_bytes = sample_count * 2;
    let mut bytes = Vec::with_capacity(44 + data_bytes as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    bytes.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    bytes.extend_from_slice(&2_u16.to_le_bytes());
    bytes.extend_from_slice(&16_u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_bytes.to_le_bytes());

    let mut filtered_noise = 0.0;
    for index in 0..sample_count {
        let time = index as f32 / SAMPLE_RATE as f32;
        filtered_noise += (noise(index) - filtered_noise) * 0.12;
        let sample = match kind {
            SoundKind::Place => {
                let phase = TAU * (145.0 * time - 270.0 * time * time);
                let envelope = (-22.0 * time).exp();
                let body = phase.sin() * envelope * 0.72;
                let weight = (phase * 0.5).sin() * envelope * 0.18;
                let attack = noise(index) * (-300.0 * time).exp() * 0.08;
                body + weight + attack
            }
            SoundKind::Break => {
                let phase = TAU * (115.0 * time - 210.0 * time * time);
                let envelope = (-16.0 * time).exp();
                let body = phase.sin() * envelope * 0.72;
                let weight = (phase * 0.5).sin() * envelope * 0.16;
                let muffled_crack = filtered_noise * (-24.0 * time).exp() * 0.24;
                body + weight + muffled_crack
            }
        };
        let pcm = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        bytes.extend_from_slice(&pcm.to_le_bytes());
    }

    bytes
}

fn noise(index: u32) -> f32 {
    let mut value = index.wrapping_add(0x9e37_79b9);
    value ^= value >> 16;
    value = value.wrapping_mul(0x7feb_352d);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846c_a68b);
    value ^= value >> 16;
    (value as f32 / u32::MAX as f32) * 2.0 - 1.0
}
