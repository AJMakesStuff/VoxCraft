mod block;
mod chunk;
mod generation;
mod manager;
mod meshing;
mod texture;

pub use block::Block;
pub use chunk::{CHUNK_SIZE, ChunkCoord, ChunkData, local_from_world};
pub(crate) use generation::TerrainNoise;
pub use generation::{WORLD_HEIGHT, generate_chunk, terrain_spawn_height};
pub use manager::{WorldData, WorldMaterial, WorldPlugin, rebuild_chunk_mesh};
#[allow(unused_imports)]
pub use meshing::{ChunkMesh, build_chunk_mesh, build_chunk_mesh_from_data};
pub(crate) use meshing::{ChunkMeshes, build_chunk_meshes};
pub(crate) use texture::block_texture_pixel_for_face;
pub(crate) use texture::update_water_normal_texture;
pub use texture::{block_texture_atlas, water_normal_texture, water_texture};
