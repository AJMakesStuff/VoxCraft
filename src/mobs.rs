use bevy::prelude::*;

use crate::{
    game::GameState,
    player::Player,
    world::{CHUNK_SIZE, WorldData},
};

const WANDER_SPEED: f32 = 1.1;
const MAX_PIGS: usize = 20;
const MIN_PIG_SEPARATION: f32 = 3.0;
const MAX_SPAWN_ATTEMPTS: u32 = 48;
const MAX_TARGET_ATTEMPTS: u32 = 12;

#[derive(Component)]
struct Pig;

#[derive(Component)]
struct PigWander {
    target: Option<Vec3>,
    pause: f32,
    random_state: u32,
    walking: bool,
    walk_phase: f32,
    idle_phase: f32,
    jumping: bool,
    jump_velocity: f32,
}

#[derive(Component)]
struct PigPart {
    owner: Entity,
    kind: PigPartKind,
    base_translation: Vec3,
}

#[derive(Clone, Copy)]
enum PigPartKind {
    Body,
    Head,
    Static,
    Ear(f32),
    Leg(f32),
}

#[derive(Resource)]
struct PigAssets {
    body_mesh: Handle<Mesh>,
    head_mesh: Handle<Mesh>,
    snout_mesh: Handle<Mesh>,
    eye_mesh: Handle<Mesh>,
    ear_mesh: Handle<Mesh>,
    leg_mesh: Handle<Mesh>,
    body_material: Handle<StandardMaterial>,
    snout_material: Handle<StandardMaterial>,
    eye_material: Handle<StandardMaterial>,
    leg_material: Handle<StandardMaterial>,
}

pub struct MobPlugin;

impl Plugin for MobPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, create_pig_assets)
            .add_systems(OnEnter(GameState::MainMenu), clear_pigs)
            .add_systems(
                Update,
                (
                    spawn_pig_if_missing,
                    wander_pig,
                    advance_pig_jumps,
                    animate_pig,
                )
                    .chain(),
            );
    }
}

fn create_pig_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(PigAssets {
        body_mesh: meshes.add(Cuboid::new(0.88, 0.62, 0.88)),
        head_mesh: meshes.add(Cuboid::new(0.48, 0.46, 0.44)),
        snout_mesh: meshes.add(Cuboid::new(0.28, 0.19, 0.12)),
        eye_mesh: meshes.add(Cuboid::new(0.055, 0.075, 0.035)),
        ear_mesh: meshes.add(Cuboid::new(0.12, 0.18, 0.12)),
        leg_mesh: meshes.add(Cuboid::new(0.18, 0.42, 0.18)),
        body_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.83, 0.43, 0.46),
            perceptual_roughness: 0.92,
            ..default()
        }),
        snout_material: materials.add(Color::srgb(0.95, 0.58, 0.59)),
        eye_material: materials.add(Color::srgb(0.08, 0.045, 0.04)),
        leg_material: materials.add(Color::srgb(0.56, 0.28, 0.31)),
    });
}

fn spawn_pig_if_missing(
    mut commands: Commands,
    state: Res<State<GameState>>,
    world: Res<WorldData>,
    player: Query<&Transform, With<Player>>,
    existing_pigs: Query<&Transform, With<Pig>>,
    assets: Res<PigAssets>,
) {
    if *state.get() != GameState::Playing {
        return;
    }
    let Ok(player_transform) = player.single() else {
        return;
    };

    let mut occupied: Vec<_> = existing_pigs
        .iter()
        .map(|transform| transform.translation)
        .collect();
    while occupied.len() < MAX_PIGS {
        let index = occupied.len() as u32;
        let seed = world.seed.wrapping_add(index.wrapping_mul(0x9e37_79b9));
        let Some(spawn) = find_pig_spawn(&world, player_transform.translation, seed, &occupied)
        else {
            break;
        };
        spawn_pig(&mut commands, &assets, spawn, seed, index);
        occupied.push(spawn);
    }
}

fn spawn_pig(commands: &mut Commands, assets: &PigAssets, spawn: Vec3, seed: u32, index: u32) {
    let mut random_state = seed ^ 0x7069_6721 ^ index;
    if random_state == 0 {
        random_state = 1;
    }
    let pause = random_unit(&mut random_state) * 2.0;

    let pig = commands
        .spawn((
            Pig,
            PigWander {
                target: None,
                pause,
                random_state,
                walking: false,
                walk_phase: 0.0,
                idle_phase: 0.0,
                jumping: false,
                jump_velocity: 0.0,
            },
            Transform::from_translation(spawn),
            Visibility::default(),
        ))
        .id();
    commands.entity(pig).with_children(|children| {
        pig_part(
            children,
            &assets.body_mesh,
            &assets.body_material,
            Vec3::new(0.0, 0.72, 0.1),
            PigPartKind::Body,
            pig,
        );
        children
            .spawn((
                Mesh3d(assets.head_mesh.clone()),
                MeshMaterial3d(assets.body_material.clone()),
                Transform::from_xyz(0.0, 0.91, -0.42),
                PigPart {
                    owner: pig,
                    kind: PigPartKind::Head,
                    base_translation: Vec3::new(0.0, 0.91, -0.42),
                },
            ))
            .with_children(|head| {
                pig_part(
                    head,
                    &assets.snout_mesh,
                    &assets.snout_material,
                    Vec3::new(0.0, -0.13, -0.27),
                    PigPartKind::Static,
                    pig,
                );
                for side in [-1.0, 1.0] {
                    pig_part(
                        head,
                        &assets.eye_mesh,
                        &assets.eye_material,
                        Vec3::new(side * 0.16, 0.08, -0.23),
                        PigPartKind::Static,
                        pig,
                    );
                    pig_part(
                        head,
                        &assets.ear_mesh,
                        &assets.body_material,
                        Vec3::new(side * 0.18, 0.29, 0.07),
                        PigPartKind::Ear(side),
                        pig,
                    );
                }
            });
        for side in [-1.0, 1.0] {
            for front in [-1.0, 1.0] {
                pig_part(
                    children,
                    &assets.leg_mesh,
                    &assets.leg_material,
                    Vec3::new(side * 0.25, 0.21, if front < 0.0 { -0.21 } else { 0.37 }),
                    PigPartKind::Leg(if side * front > 0.0 {
                        0.0
                    } else {
                        std::f32::consts::PI
                    }),
                    pig,
                );
            }
        }
    });
}

fn pig_part(
    children: &mut ChildSpawnerCommands,
    mesh: &Handle<Mesh>,
    material: &Handle<StandardMaterial>,
    position: Vec3,
    kind: PigPartKind,
    owner: Entity,
) {
    children.spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(material.clone()),
        Transform::from_translation(position),
        PigPart {
            owner,
            kind,
            base_translation: position,
        },
    ));
}

fn find_pig_spawn(
    world: &WorldData,
    player_position: Vec3,
    seed: u32,
    occupied: &[Vec3],
) -> Option<Vec3> {
    let mut random_state = seed ^ 0x51ed_270b;
    let center_x = player_position.x.floor() as i32;
    let center_z = player_position.z.floor() as i32;
    let max_radius = (world.render_distance * CHUNK_SIZE) as f32 - CHUNK_SIZE as f32 * 0.75;
    let max_radius = max_radius.max(12.0);
    let min_radius = (max_radius * 0.25).max(8.0).min(max_radius - 1.0);
    let mut fallback = None;

    for _ in 0..MAX_SPAWN_ATTEMPTS {
        let angle = random_unit(&mut random_state) * std::f32::consts::TAU;
        let radius = (min_radius.powi(2)
            + random_unit(&mut random_state) * (max_radius.powi(2) - min_radius.powi(2)))
        .sqrt();
        let x = center_x + (angle.cos() * radius).round() as i32;
        let z = center_z + (angle.sin() * radius).round() as i32;
        let Some((ground_y, block)) = ground_at(world, x, z) else {
            continue;
        };
        let position = Vec3::new(x as f32 + 0.5, ground_y, z as f32 + 0.5);
        if occupied.iter().any(|other| {
            Vec2::new(position.x - other.x, position.z - other.z).length() < MIN_PIG_SEPARATION
        }) {
            continue;
        }
        if block == crate::world::Block::Grass {
            return Some(position);
        }
        fallback.get_or_insert(position);
    }

    fallback
}

fn wander_pig(
    time: Res<Time>,
    state: Res<State<GameState>>,
    world: Res<WorldData>,
    mut pigs: Query<(&mut Transform, &mut PigWander), With<Pig>>,
) {
    if *state.get() != GameState::Playing {
        return;
    }
    let dt = time.delta_secs().min(0.05);
    for (mut transform, mut wander) in &mut pigs {
        wander.walking = false;
        wander.idle_phase = (wander.idle_phase + dt).rem_euclid(std::f32::consts::TAU * 4.0);
        wander.pause = (wander.pause - dt).max(0.0);
        if wander.target.is_none() && wander.pause == 0.0 {
            wander.target = choose_target(&world, transform.translation, &mut wander.random_state);
            if wander.target.is_none() {
                wander.pause = 1.0;
            }
        }

        let Some(target) = wander.target else {
            continue;
        };
        let offset = Vec2::new(
            target.x - transform.translation.x,
            target.z - transform.translation.z,
        );
        if offset.length() < 0.18 {
            wander.target = None;
            wander.pause = 1.0 + random_unit(&mut wander.random_state) * 2.0;
            continue;
        }

        let direction = offset.normalize();
        let next_x = transform.translation.x + direction.x * WANDER_SPEED * dt;
        let next_z = transform.translation.z + direction.y * WANDER_SPEED * dt;
        let ground = ground_at(&world, next_x.floor() as i32, next_z.floor() as i32);
        let Some((ground_y, _)) = ground else {
            wander.target = None;
            wander.pause = 0.5;
            continue;
        };
        let rise = ground_y - transform.translation.y;
        if rise > 1.01 {
            wander.target = None;
            wander.pause = 0.5;
            continue;
        }
        if rise > 0.1 && !wander.jumping {
            wander.jumping = true;
            wander.jump_velocity = 5.5;
        }

        transform.translation.x = next_x;
        transform.translation.z = next_z;
        if !wander.jumping {
            transform.translation.y = ground_y;
        }
        transform.rotation = Quat::from_rotation_y((-direction.x).atan2(-direction.y));
        wander.walking = true;
        wander.walk_phase = (wander.walk_phase + dt * 11.0).rem_euclid(std::f32::consts::TAU);
    }
}

fn advance_pig_jumps(
    time: Res<Time>,
    state: Res<State<GameState>>,
    world: Res<WorldData>,
    mut pigs: Query<(&mut Transform, &mut PigWander), With<Pig>>,
) {
    if *state.get() != GameState::Playing {
        return;
    }
    let dt = time.delta_secs().min(0.05);
    for (mut transform, mut wander) in &mut pigs {
        if !wander.jumping {
            continue;
        }
        let Some((ground_y, _)) = ground_at(
            &world,
            transform.translation.x.floor() as i32,
            transform.translation.z.floor() as i32,
        ) else {
            continue;
        };

        wander.jump_velocity -= 12.0 * dt;
        let next_y = transform.translation.y + wander.jump_velocity * dt;
        if next_y <= ground_y && wander.jump_velocity < 0.0 {
            transform.translation.y = ground_y;
            wander.jump_velocity = 0.0;
            wander.jumping = false;
        } else {
            transform.translation.y = next_y;
        }
    }
}

fn animate_pig(
    state: Res<State<GameState>>,
    pigs: Query<&PigWander, With<Pig>>,
    mut parts: Query<(&mut Transform, &PigPart)>,
) {
    if *state.get() != GameState::Playing {
        return;
    }

    for (mut transform, part) in &mut parts {
        let Ok(wander) = pigs.get(part.owner) else {
            continue;
        };
        transform.translation = part.base_translation;
        transform.rotation = Quat::IDENTITY;
        transform.scale = Vec3::ONE;
        match part.kind {
            PigPartKind::Body if wander.walking => {
                transform.translation.y += (wander.walk_phase * 2.0).sin().abs() * 0.025;
                transform.rotation = Quat::from_rotation_x((wander.walk_phase * 2.0).sin() * 0.025);
            }
            PigPartKind::Body => {
                let breathing = (wander.idle_phase * 1.4).sin() * 0.012;
                transform.translation.y += breathing;
                transform.scale.y = 1.0 + breathing;
            }
            PigPartKind::Head if wander.walking => {
                transform.rotation =
                    Quat::from_rotation_x((wander.walk_phase * 2.0).sin() * -0.035);
            }
            PigPartKind::Head => {
                transform.rotation = Quat::from_euler(
                    EulerRot::XYZ,
                    (wander.idle_phase * 0.9).sin() * 0.025,
                    (wander.idle_phase * 0.65).sin() * 0.07,
                    0.0,
                );
            }
            PigPartKind::Static => {}
            PigPartKind::Ear(side) if wander.walking => {
                transform.rotation =
                    Quat::from_rotation_z(side * (wander.walk_phase * 2.0).sin() * 0.08);
            }
            PigPartKind::Ear(side) => {
                let twitch = (wander.idle_phase * 1.6 + side).sin().powi(8) * 0.12;
                transform.rotation = Quat::from_rotation_z(side * twitch);
            }
            PigPartKind::Leg(phase) if wander.walking => {
                let stride = (wander.walk_phase + phase).sin();
                transform.rotation = Quat::from_rotation_x(stride * 0.58);
                transform.translation.y += stride.max(0.0) * 0.14;
            }
            PigPartKind::Leg(_) => {}
        }
    }
}

fn choose_target(world: &WorldData, position: Vec3, random_state: &mut u32) -> Option<Vec3> {
    for _ in 0..MAX_TARGET_ATTEMPTS {
        let angle = random_unit(random_state) * std::f32::consts::TAU;
        let radius = 2.0 + random_unit(random_state) * 5.0;
        let x = (position.x + angle.cos() * radius).floor() as i32;
        let z = (position.z + angle.sin() * radius).floor() as i32;
        if let Some((ground_y, _)) = ground_at(world, x, z)
            && (ground_y - position.y).abs() <= 1.0
        {
            return Some(Vec3::new(x as f32 + 0.5, ground_y, z as f32 + 0.5));
        }
    }
    None
}

fn ground_at(world: &WorldData, x: i32, z: i32) -> Option<(f32, crate::world::Block)> {
    for y in (1..crate::world::WORLD_HEIGHT).rev() {
        let block = world.block_at(IVec3::new(x, y, z));
        if !block.is_solid() {
            continue;
        }
        if world.block_at(IVec3::new(x, y + 1, z)) != crate::world::Block::Air {
            return None;
        }
        return Some(((y + 1) as f32, block));
    }
    None
}

fn random_unit(state: &mut u32) -> f32 {
    if *state == 0 {
        *state = 0x6d2b_79f5;
    }
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    *state as f32 / u32::MAX as f32
}

fn clear_pigs(mut commands: Commands, pigs: Query<Entity, With<Pig>>) {
    for pig in &pigs {
        commands.entity(pig).despawn();
    }
}
