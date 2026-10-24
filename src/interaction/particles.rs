use bevy::prelude::*;

use crate::{game::GameState, world::Block};

const PARTICLE_COUNT: usize = 12;
const PARTICLE_LIFETIME_SECS: f32 = 0.55;

#[derive(Resource)]
pub(super) struct BlockParticleAssets {
    mesh: Handle<Mesh>,
    materials: [Handle<StandardMaterial>; Block::SOLID_PICKER.len()],
}

#[derive(Component)]
pub(super) struct BlockParticle {
    velocity: Vec3,
    angular_velocity: Vec3,
    lifetime: Timer,
}

pub(super) fn setup_block_particle_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let materials = Block::SOLID_PICKER.map(|block| {
        let color = block.face_color(0);
        materials.add(StandardMaterial {
            base_color: Color::linear_rgba(color.x, color.y, color.z, 1.0),
            perceptual_roughness: 1.0,
            ..default()
        })
    });

    commands.insert_resource(BlockParticleAssets { mesh, materials });
}

pub(super) fn spawn_block_particles(
    commands: &mut Commands,
    position: IVec3,
    block: Block,
    assets: &BlockParticleAssets,
) {
    if block == Block::Air {
        return;
    }
    let material_index = block.texture_index();
    let block_origin = position.as_vec3();
    let material = assets.materials[material_index].clone();

    for index in 0..PARTICLE_COUNT {
        let angle = index as f32 * std::f32::consts::TAU / PARTICLE_COUNT as f32;
        let horizontal_speed = 0.7 + (index % 4) as f32 * 0.18;
        let upward_speed = 0.9 + (index % 3) as f32 * 0.22;
        let size = 0.08 + (index % 3) as f32 * 0.015;
        let spread = if index / 6 == 0 { 0.25 } else { 0.75 };
        let cross_spread = 1.0 - spread;
        let (surface_offset, face_normal) = match index % 6 {
            0 => (Vec3::new(-0.03, spread, cross_spread), Vec3::NEG_X),
            1 => (Vec3::new(1.03, spread, cross_spread), Vec3::X),
            2 => (Vec3::new(spread, -0.03, cross_spread), Vec3::NEG_Y),
            3 => (Vec3::new(spread, 1.03, cross_spread), Vec3::Y),
            4 => (Vec3::new(spread, cross_spread, -0.03), Vec3::NEG_Z),
            _ => (Vec3::new(spread, cross_spread, 1.03), Vec3::Z),
        };
        let horizontal_drift = Vec3::new(angle.cos(), 0.0, angle.sin()) * 0.35;

        commands.spawn((
            Mesh3d(assets.mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(block_origin + surface_offset)
                .with_scale(Vec3::splat(size))
                .with_rotation(Quat::from_euler(
                    EulerRot::XYZ,
                    angle * 0.7,
                    angle,
                    angle * 0.4,
                )),
            BlockParticle {
                velocity: Vec3::new(
                    face_normal.x * horizontal_speed + horizontal_drift.x,
                    face_normal.y * horizontal_speed + upward_speed,
                    face_normal.z * horizontal_speed + horizontal_drift.z,
                ),
                angular_velocity: Vec3::new(2.5 + angle.sin(), 3.0, 2.0 + angle.cos()),
                lifetime: Timer::from_seconds(PARTICLE_LIFETIME_SECS, TimerMode::Once),
            },
        ));
    }
}

pub(super) fn update_block_particles(
    time: Res<Time>,
    state: Res<State<GameState>>,
    mut commands: Commands,
    mut particles: Query<(Entity, &mut Transform, &mut BlockParticle)>,
) {
    if *state.get() != GameState::Playing {
        return;
    }

    let dt = time.delta_secs().min(0.05);
    for (entity, mut transform, mut particle) in &mut particles {
        particle.lifetime.tick(time.delta());
        particle.velocity.y -= 8.0 * dt;
        transform.translation += particle.velocity * dt;
        transform.rotate(Quat::from_euler(
            EulerRot::XYZ,
            particle.angular_velocity.x * dt,
            particle.angular_velocity.y * dt,
            particle.angular_velocity.z * dt,
        ));

        if particle.lifetime.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

pub(super) fn clear_block_particles(
    mut commands: Commands,
    particles: Query<Entity, With<BlockParticle>>,
) {
    for entity in &particles {
        commands.entity(entity).despawn();
    }
}
