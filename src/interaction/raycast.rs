use bevy::{
    ecs::system::SystemParam,
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
};

use crate::{
    game::GameState,
    interaction::{HOTBAR_SLOTS, Hotbar},
    player::{Player, player_forward},
    world::{Block, CHUNK_SIZE, ChunkCoord, WorldData, WorldMaterial, rebuild_chunk_mesh},
};

use super::particles::{
    BlockParticleAssets, clear_block_particles, setup_block_particle_assets, spawn_block_particles,
    update_block_particles,
};

const REACH: f32 = 5.0;

#[derive(Resource, Default)]
pub struct Selection(pub Option<RayHit>);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayHit {
    pub block: IVec3,
    pub previous: IVec3,
    pub distance: f32,
}

pub struct InteractionPlugin;

impl Plugin for InteractionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Selection>()
            .init_resource::<Hotbar>()
            .add_systems(Startup, setup_block_particle_assets)
            .add_systems(OnEnter(GameState::MainMenu), clear_block_particles)
            .add_systems(
                Update,
                (
                    select_hotbar,
                    update_selection,
                    edit_block,
                    update_block_particles,
                    draw_selection,
                )
                    .chain(),
            );
    }
}

pub fn raycast_blocks(
    origin: Vec3,
    direction: Vec3,
    reach: f32,
    world: &WorldData,
) -> Option<RayHit> {
    let direction = direction.normalize_or_zero();
    if direction == Vec3::ZERO {
        return None;
    }
    let mut cell = origin.floor().as_ivec3();
    let mut previous = cell;
    let step = IVec3::new(
        direction.x.signum() as i32,
        direction.y.signum() as i32,
        direction.z.signum() as i32,
    );
    let delta = Vec3::new(
        if direction.x == 0.0 {
            f32::INFINITY
        } else {
            (1.0 / direction.x).abs()
        },
        if direction.y == 0.0 {
            f32::INFINITY
        } else {
            (1.0 / direction.y).abs()
        },
        if direction.z == 0.0 {
            f32::INFINITY
        } else {
            (1.0 / direction.z).abs()
        },
    );
    let boundary = Vec3::new(
        if step.x > 0 {
            cell.x as f32 + 1.0
        } else {
            cell.x as f32
        },
        if step.y > 0 {
            cell.y as f32 + 1.0
        } else {
            cell.y as f32
        },
        if step.z > 0 {
            cell.z as f32 + 1.0
        } else {
            cell.z as f32
        },
    );
    let mut max = Vec3::new(
        if direction.x == 0.0 {
            f32::INFINITY
        } else {
            (boundary.x - origin.x) / direction.x
        },
        if direction.y == 0.0 {
            f32::INFINITY
        } else {
            (boundary.y - origin.y) / direction.y
        },
        if direction.z == 0.0 {
            f32::INFINITY
        } else {
            (boundary.z - origin.z) / direction.z
        },
    );
    let mut distance = 0.0;
    loop {
        if world.block_at(cell).is_solid() {
            return Some(RayHit {
                block: cell,
                previous,
                distance,
            });
        }
        previous = cell;
        if max.x < max.y && max.x < max.z {
            distance = max.x;
            cell.x += step.x;
            max.x += delta.x;
        } else if max.y < max.z {
            distance = max.y;
            cell.y += step.y;
            max.y += delta.y;
        } else {
            distance = max.z;
            cell.z += step.z;
            max.z += delta.z;
        }
        if distance > reach {
            return None;
        }
    }
}

fn update_selection(
    world: Res<WorldData>,
    state: Res<State<GameState>>,
    mut selection: ResMut<Selection>,
    player: Query<&Transform, With<Player>>,
) {
    if *state.get() != GameState::Playing {
        selection.0 = None;
        return;
    }
    let Ok(transform) = player.single() else {
        selection.0 = None;
        return;
    };
    selection.0 = raycast_blocks(
        transform.translation,
        player_forward(transform),
        REACH,
        &world,
    );
}

fn select_hotbar(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameState>>,
    mut scroll: MessageReader<MouseWheel>,
    mut hotbar: ResMut<Hotbar>,
    mut wheel_remainder: Local<f32>,
) {
    if *state.get() != GameState::Playing {
        scroll.clear();
        *wheel_remainder = 0.0;
        return;
    }

    for slot in 1..=HOTBAR_SLOTS {
        let key = match slot {
            1 => KeyCode::Digit1,
            2 => KeyCode::Digit2,
            3 => KeyCode::Digit3,
            4 => KeyCode::Digit4,
            _ => KeyCode::Digit5,
        };
        if keys.just_pressed(key) {
            hotbar.select_slot(slot - 1);
        }
    }

    let mut steps = 0;
    for event in scroll.read() {
        let lines = match event.unit {
            MouseScrollUnit::Line => event.y,
            MouseScrollUnit::Pixel => event.y / 100.0,
        };
        *wheel_remainder += lines;
        steps += wheel_remainder.trunc() as i32;
        *wheel_remainder = wheel_remainder.fract();
    }

    if steps != 0 {
        let slot = (hotbar.selected as i32 - steps).rem_euclid(HOTBAR_SLOTS as i32) as usize;
        hotbar.select_slot(slot);
    }
}

#[derive(SystemParam)]
struct BlockEditParams<'w, 's> {
    buttons: Res<'w, ButtonInput<MouseButton>>,
    state: Res<'w, State<GameState>>,
    selection: Res<'w, Selection>,
    player: Query<'w, 's, &'static Transform, With<Player>>,
    world: ResMut<'w, WorldData>,
    hotbar: ResMut<'w, Hotbar>,
    particle_assets: Res<'w, BlockParticleAssets>,
    commands: Commands<'w, 's>,
    meshes: ResMut<'w, Assets<Mesh>>,
    material: Res<'w, WorldMaterial>,
}

fn edit_block(params: BlockEditParams) {
    let BlockEditParams {
        buttons,
        state,
        selection,
        player,
        mut world,
        mut hotbar,
        particle_assets,
        mut commands,
        mut meshes,
        material,
    } = params;
    if *state.get() != GameState::Playing {
        return;
    }
    let Some(hit) = selection.0 else {
        return;
    };
    let mut edited = None;
    let mut broken = None;
    if buttons.just_pressed(MouseButton::Left) {
        let block = world.block_at(hit.block);
        if block != Block::Bedrock {
            edited = Some((hit.block, Block::Air));
            broken = Some((hit.block, block));
        }
    } else if buttons.just_pressed(MouseButton::Middle) {
        let block = match world.block_at(hit.block) {
            Block::OakLogX | Block::OakLogZ => Block::OakLog,
            block => block,
        };
        hotbar.replace_selected(block);
    } else if buttons.just_pressed(MouseButton::Right) {
        let pos = hit.previous;
        let Ok(player) = player.single() else {
            return;
        };
        if !intersects_player(pos, player.translation) && !world.block_at(pos).is_solid() {
            let face_normal = hit.previous - hit.block;
            let block = hotbar.selected_block().oriented_for_placement(face_normal);
            edited = Some((pos, block));
        }
    }
    let Some((pos, block)) = edited else {
        return;
    };
    world.set_block(pos, block);
    if let Some((broken_pos, broken_block)) = broken {
        spawn_block_particles(&mut commands, broken_pos, broken_block, &particle_assets);
    }
    let coord = ChunkCoord::from_world(pos);
    let mut affected = vec![coord];
    let local = pos - coord.origin();
    if local.x == 0 {
        affected.push(ChunkCoord(coord.0 - IVec3::X));
    }
    if local.x == CHUNK_SIZE - 1 {
        affected.push(ChunkCoord(coord.0 + IVec3::X));
    }
    if local.y == 0 {
        affected.push(ChunkCoord(coord.0 - IVec3::Y));
    }
    if local.y == CHUNK_SIZE - 1 {
        affected.push(ChunkCoord(coord.0 + IVec3::Y));
    }
    if local.z == 0 {
        affected.push(ChunkCoord(coord.0 - IVec3::Z));
    }
    if local.z == CHUNK_SIZE - 1 {
        affected.push(ChunkCoord(coord.0 + IVec3::Z));
    }
    affected.sort_unstable_by_key(|coord| (coord.0.x, coord.0.y, coord.0.z));
    affected.dedup();
    for chunk in affected {
        if world.chunks.contains_key(&chunk) {
            rebuild_chunk_mesh(&mut commands, &mut world, &mut meshes, &material.0, chunk);
        }
    }
}

fn intersects_player(block: IVec3, eye: Vec3) -> bool {
    let block_min = block.as_vec3();
    let block_max = block_min + Vec3::ONE;
    let player_min = Vec3::new(eye.x - 0.3, eye.y - 1.62, eye.z - 0.3);
    let player_max = Vec3::new(eye.x + 0.3, eye.y + 0.18, eye.z + 0.3);
    block_min.cmplt(player_max).all() && block_max.cmpgt(player_min).all()
}

fn draw_selection(selection: Res<Selection>, mut gizmos: Gizmos) {
    if let Some(hit) = selection.0 {
        gizmos.cube(
            Transform::from_translation(hit.block.as_vec3() + Vec3::splat(0.5))
                .with_scale(Vec3::splat(1.01)),
            Color::WHITE,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::WorldData;

    #[test]
    fn raycast_finds_solid_block_and_stops_at_reach() {
        let mut world = WorldData::new(23_041, 8);
        world.overrides.insert(IVec3::new(2, 20, 2), Block::Stone);
        let hit = raycast_blocks(Vec3::new(0.5, 20.5, 2.5), Vec3::X, 5.0, &world).unwrap();
        assert_eq!(hit.block, IVec3::new(2, 20, 2));
        assert_eq!(hit.previous, IVec3::new(1, 20, 2));
        assert!(raycast_blocks(Vec3::new(0.5, 20.5, 2.5), Vec3::X, 1.0, &world).is_none());
    }
}
