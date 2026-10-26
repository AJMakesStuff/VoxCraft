use std::collections::{HashMap, HashSet};

use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use futures_lite::future;

use super::{
    Block, CHUNK_SIZE, ChunkCoord, ChunkData, ChunkMeshes, TerrainNoise, WORLD_HEIGHT,
    build_chunk_meshes, generate_chunk, local_from_world,
};
use crate::{
    game::{GameSettings, GameState},
    player::Player,
};

const MAX_VERTICAL_CHUNKS: i32 = WORLD_HEIGHT / CHUNK_SIZE;
const MAX_BACKGROUND_TASKS: usize = 4;
const LOADS_PER_FRAME: usize = 2;
const MESHES_PER_FRAME: usize = 1;

fn background_task_limit() -> usize {
    std::thread::available_parallelism()
        .map_or(2, |threads| threads.get())
        .saturating_sub(1)
        .clamp(1, MAX_BACKGROUND_TASKS)
}

pub struct LoadedChunk {
    pub data: ChunkData,
    pub entity: Option<Entity>,
    pub water_entity: Option<Entity>,
    pub transparent_entity: Option<Entity>,
}

#[derive(Resource)]
pub struct WorldData {
    pub seed: u32,
    noise: TerrainNoise,
    pub render_distance: i32,
    pub chunks: HashMap<ChunkCoord, LoadedChunk>,
    pub overrides: HashMap<IVec3, Block>,
    desired: HashSet<ChunkCoord>,
    desired_order: Vec<ChunkCoord>,
    last_center: Option<IVec2>,
    pending: HashSet<ChunkCoord>,
    mesh_pending: HashSet<ChunkCoord>,
    mesh_revisions: HashMap<ChunkCoord, u64>,
}

impl WorldData {
    pub fn new(seed: u32, render_distance: i32) -> Self {
        Self {
            seed,
            noise: TerrainNoise::new(seed),
            render_distance: render_distance.clamp(2, 16),
            chunks: HashMap::new(),
            overrides: HashMap::new(),
            desired: HashSet::new(),
            desired_order: Vec::new(),
            last_center: None,
            pending: HashSet::new(),
            mesh_pending: HashSet::new(),
            mesh_revisions: HashMap::new(),
        }
    }

    pub fn block_at(&self, pos: IVec3) -> Block {
        if let Some(block) = self.overrides.get(&pos) {
            return *block;
        }
        let coord = ChunkCoord::from_world(pos);
        if let Some(chunk) = self.chunks.get(&coord) {
            return chunk.data.get(local_from_world(pos));
        }
        self.noise.block_at(pos)
    }

    pub fn set_block(&mut self, pos: IVec3, block: Block) {
        self.overrides.insert(pos, block);
        let coord = ChunkCoord::from_world(pos);
        if let Some(chunk) = self.chunks.get_mut(&coord) {
            chunk.data.set(local_from_world(pos), block);
        }
    }

    pub fn restore_saved_world(&mut self, seed: u32, overrides: HashMap<IVec3, Block>) {
        self.reset_session();
        self.seed = seed;
        self.noise = TerrainNoise::new(seed);
        self.overrides = overrides;
    }

    pub fn loaded_count(&self) -> usize {
        self.chunks.len()
    }

    fn reset_session(&mut self) {
        self.chunks.clear();
        self.overrides.clear();
        self.desired.clear();
        self.desired_order.clear();
        self.pending.clear();
        self.mesh_pending.clear();
        self.mesh_revisions.clear();
        self.last_center = None;
    }
}

#[derive(Component)]
struct ChunkLoadTask {
    coord: ChunkCoord,
    task: Task<ChunkData>,
}

#[derive(Component)]
struct ChunkMeshTask {
    coord: ChunkCoord,
    revision: u64,
    task: Task<ChunkMeshes>,
}

#[derive(Component)]
pub struct ChunkTag;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_world)
            .add_systems(OnEnter(GameState::MainMenu), unload_world)
            .add_systems(OnEnter(GameState::Playing), apply_world_seed)
            .add_systems(
                Update,
                (
                    update_chunk_targets,
                    finish_chunk_tasks,
                    finish_chunk_mesh_tasks,
                )
                    .chain(),
            );
    }
}

fn init_world(mut commands: Commands, settings: Res<GameSettings>) {
    commands.insert_resource(WorldData::new(settings.seed, settings.render_distance));
}

fn apply_world_seed(mut world: ResMut<WorldData>, settings: Res<GameSettings>) {
    if world.seed != settings.seed {
        world.reset_session();
        world.seed = settings.seed;
        world.noise = TerrainNoise::new(settings.seed);
    }
}

type WorldCleanupQuery<'w, 's> =
    Query<'w, 's, Entity, Or<(With<ChunkTag>, With<ChunkLoadTask>, With<ChunkMeshTask>)>>;

fn unload_world(
    mut commands: Commands,
    world: Option<ResMut<WorldData>>,
    chunks_and_tasks: WorldCleanupQuery,
) {
    for entity in &chunks_and_tasks {
        commands.entity(entity).despawn();
    }
    if let Some(mut world) = world {
        world.reset_session();
    }
}

fn update_chunk_targets(
    mut world: ResMut<WorldData>,
    settings: Res<GameSettings>,
    state: Res<State<GameState>>,
    player: Query<&Transform, With<Player>>,
    mut commands: Commands,
    load_tasks: Query<Entity, With<ChunkLoadTask>>,
    mesh_tasks: Query<Entity, With<ChunkMeshTask>>,
) {
    if matches!(*state.get(), GameState::MainMenu | GameState::Options) {
        return;
    }
    let render_distance = settings.render_distance.clamp(2, 16);
    if world.render_distance != render_distance {
        world.render_distance = render_distance;
        world.last_center = None;
    }
    let Ok(player_transform) = player.single() else {
        return;
    };
    let cx = (player_transform.translation.x.floor() as i32).div_euclid(CHUNK_SIZE);
    let cz = (player_transform.translation.z.floor() as i32).div_euclid(CHUNK_SIZE);
    let center = IVec2::new(cx, cz);
    if world.last_center != Some(center) {
        let mut desired = Vec::new();
        for z in -world.render_distance..=world.render_distance {
            for x in -world.render_distance..=world.render_distance {
                if x * x + z * z > world.render_distance * world.render_distance {
                    continue;
                }
                for y in 0..MAX_VERTICAL_CHUNKS {
                    let coord = ChunkCoord::new(cx + x, y, cz + z);
                    desired.push((x * x + z * z, coord));
                }
            }
        }
        desired.sort_unstable_by_key(|(distance, coord)| (*distance, coord.0.y));
        world.desired_order = desired.iter().map(|(_, coord)| *coord).collect();
        world.desired = world.desired_order.iter().copied().collect();
        world.last_center = Some(center);

        let stale: Vec<_> = world
            .chunks
            .keys()
            .copied()
            .filter(|coord| !world.desired.contains(coord))
            .collect();
        for coord in stale {
            if let Some(chunk) = world.chunks.remove(&coord) {
                if let Some(entity) = chunk.entity {
                    commands.entity(entity).despawn();
                }
                if let Some(entity) = chunk.water_entity {
                    commands.entity(entity).despawn();
                }
                if let Some(entity) = chunk.transparent_entity {
                    commands.entity(entity).despawn();
                }
            }
        }
    }

    let active_task_count = load_tasks.iter().count() + mesh_tasks.iter().count();
    let to_start = background_task_limit().saturating_sub(active_task_count);
    let candidates: Vec<_> = world
        .desired_order
        .iter()
        .copied()
        .filter(|coord| {
            !world.chunks.contains_key(coord)
                && !world.pending.contains(coord)
                && !world.mesh_pending.contains(coord)
        })
        .take(to_start)
        .collect();
    for coord in candidates {
        let seed = world.seed;
        let task_pool = AsyncComputeTaskPool::get();
        let task = task_pool.spawn(async move { generate_chunk(seed, coord) });
        commands.spawn(ChunkLoadTask { coord, task });
        world.pending.insert(coord);
    }
}

fn finish_chunk_tasks(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut ChunkLoadTask)>,
    mut world: ResMut<WorldData>,
) {
    let mut finished = 0;
    for (entity, mut task) in &mut tasks {
        if finished >= LOADS_PER_FRAME {
            break;
        }
        let Some(mut data) = future::block_on(future::poll_once(&mut task.task)) else {
            continue;
        };
        finished += 1;
        commands.entity(entity).despawn();
        world.pending.remove(&task.coord);
        if !world.desired.contains(&task.coord) {
            continue;
        }
        let coord = task.coord;
        let chunk_overrides: Vec<_> = world
            .overrides
            .iter()
            .filter_map(|(&pos, &block)| {
                (ChunkCoord::from_world(pos) == coord).then_some((pos, block))
            })
            .collect();
        for (pos, block) in chunk_overrides {
            data.set(local_from_world(pos), block);
        }
        if data.is_empty() {
            world.chunks.insert(
                coord,
                LoadedChunk {
                    data,
                    entity: None,
                    water_entity: None,
                    transparent_entity: None,
                },
            );
            continue;
        }
        let mesh_chunk = data.clone();
        world.chunks.insert(
            coord,
            LoadedChunk {
                data,
                entity: None,
                water_entity: None,
                transparent_entity: None,
            },
        );
        let overrides = world.overrides.clone();
        let seed = world.seed;
        let revision = world
            .mesh_revisions
            .get(&coord)
            .copied()
            .unwrap_or_default();
        let task = AsyncComputeTaskPool::get().spawn(async move {
            super::meshing::build_chunk_meshes_from_data(coord, &mesh_chunk, seed, &overrides)
        });
        commands.spawn(ChunkMeshTask {
            coord,
            revision,
            task,
        });
        world.mesh_pending.insert(coord);
    }
}

fn finish_chunk_mesh_tasks(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut ChunkMeshTask)>,
    mut world: ResMut<WorldData>,
    mut meshes: ResMut<Assets<Mesh>>,
    materials: Res<WorldMaterial>,
) {
    let mut finished = 0;
    for (entity, mut task) in &mut tasks {
        if finished >= MESHES_PER_FRAME {
            break;
        }
        let Some(chunk_mesh) = future::block_on(future::poll_once(&mut task.task)) else {
            continue;
        };
        finished += 1;
        let coord = task.coord;
        let revision = task.revision;
        commands.entity(entity).despawn();
        world.mesh_pending.remove(&coord);
        if !world.desired.contains(&coord)
            || world
                .mesh_revisions
                .get(&coord)
                .copied()
                .unwrap_or_default()
                != revision
        {
            continue;
        }
        let Some(chunk) = world.chunks.get_mut(&coord) else {
            continue;
        };
        if !chunk_mesh.solid.is_empty() {
            let mesh = meshes.add(chunk_mesh.solid.into_mesh());
            chunk.entity = Some(
                commands
                    .spawn((
                        Mesh3d(mesh),
                        MeshMaterial3d(materials.terrain.clone()),
                        Transform::from_translation(coord.origin().as_vec3()),
                        ChunkTag,
                    ))
                    .id(),
            );
        }
        if !chunk_mesh.water.is_empty() {
            let mesh = meshes.add(chunk_mesh.water.into_mesh());
            chunk.water_entity = Some(
                commands
                    .spawn((
                        Mesh3d(mesh),
                        MeshMaterial3d(materials.water.clone()),
                        Transform::from_translation(coord.origin().as_vec3()),
                        ChunkTag,
                    ))
                    .id(),
            );
        }
        if !chunk_mesh.transparent.is_empty() {
            let mesh = meshes.add(chunk_mesh.transparent.into_mesh());
            chunk.transparent_entity = Some(
                commands
                    .spawn((
                        Mesh3d(mesh),
                        MeshMaterial3d(materials.glass.clone()),
                        Transform::from_translation(coord.origin().as_vec3()),
                        ChunkTag,
                    ))
                    .id(),
            );
        }
    }
}

#[derive(Resource)]
pub struct WorldMaterial {
    pub terrain: Handle<StandardMaterial>,
    pub water: Handle<StandardMaterial>,
    pub glass: Handle<StandardMaterial>,
}

pub fn rebuild_chunk_mesh(
    commands: &mut Commands,
    world: &mut WorldData,
    meshes: &mut Assets<Mesh>,
    material: &WorldMaterial,
    coord: ChunkCoord,
) {
    let revision = world.mesh_revisions.entry(coord).or_default();
    *revision = revision.wrapping_add(1);
    if let Some(chunk) = world.chunks.get(&coord) {
        if let Some(entity) = chunk.entity {
            commands.entity(entity).despawn();
        }
        if let Some(entity) = chunk.water_entity {
            commands.entity(entity).despawn();
        }
        if let Some(entity) = chunk.transparent_entity {
            commands.entity(entity).despawn();
        }
    }
    let mesh_data = build_chunk_meshes(coord, world);
    let entity = if mesh_data.solid.is_empty() {
        None
    } else {
        let mesh = meshes.add(mesh_data.solid.into_mesh());
        Some(
            commands
                .spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(material.terrain.clone()),
                    Transform::from_translation(coord.origin().as_vec3()),
                    ChunkTag,
                ))
                .id(),
        )
    };
    let water_entity = if mesh_data.water.is_empty() {
        None
    } else {
        let mesh = meshes.add(mesh_data.water.into_mesh());
        Some(
            commands
                .spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(material.water.clone()),
                    Transform::from_translation(coord.origin().as_vec3()),
                    ChunkTag,
                ))
                .id(),
        )
    };
    let transparent_entity = if mesh_data.transparent.is_empty() {
        None
    } else {
        let mesh = meshes.add(mesh_data.transparent.into_mesh());
        Some(
            commands
                .spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(material.glass.clone()),
                    Transform::from_translation(coord.origin().as_vec3()),
                    ChunkTag,
                ))
                .id(),
        )
    };
    if let Some(chunk) = world.chunks.get_mut(&coord) {
        chunk.entity = entity;
        chunk.water_entity = water_entity;
        chunk.transparent_entity = transparent_entity;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_edits_update_loaded_chunks_and_persist_in_overrides() {
        let mut world = WorldData::new(23_041, 8);
        let coord = ChunkCoord::new(0, 0, 0);
        world.chunks.insert(
            coord,
            LoadedChunk {
                data: ChunkData::default(),
                entity: None,
                water_entity: None,
                transparent_entity: None,
            },
        );
        let pos = IVec3::new(15, 10, 7);

        world.set_block(pos, Block::Stone);
        assert_eq!(world.block_at(pos), Block::Stone);
        world.set_block(pos, Block::Air);
        assert_eq!(world.block_at(pos), Block::Air);
        assert_eq!(world.overrides.get(&pos), Some(&Block::Air));
    }

    #[test]
    fn returning_to_menu_clears_loaded_terrain_and_edits() {
        let mut world = WorldData::new(23_041, 8);
        world.chunks.insert(
            ChunkCoord::new(0, 0, 0),
            LoadedChunk {
                data: ChunkData::default(),
                entity: None,
                water_entity: None,
                transparent_entity: None,
            },
        );
        world.overrides.insert(IVec3::new(1, 2, 3), Block::Stone);
        world.desired.insert(ChunkCoord::new(0, 0, 0));
        world.desired_order.push(ChunkCoord::new(0, 0, 0));
        world.pending.insert(ChunkCoord::new(1, 0, 0));
        world.mesh_pending.insert(ChunkCoord::new(2, 0, 0));
        world.mesh_revisions.insert(ChunkCoord::new(0, 0, 0), 1);
        world.last_center = Some(IVec2::ZERO);

        world.reset_session();

        assert_eq!(world.loaded_count(), 0);
        assert!(world.overrides.is_empty());
        assert!(world.desired.is_empty());
        assert!(world.desired_order.is_empty());
        assert!(world.pending.is_empty());
        assert!(world.mesh_pending.is_empty());
        assert!(world.mesh_revisions.is_empty());
        assert_eq!(world.last_center, None);
    }
}
