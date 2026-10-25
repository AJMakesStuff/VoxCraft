use std::collections::HashMap;

use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

use super::{
    texture::{BLOCK_TEXTURE_COUNT, BLOCK_TEXTURE_SIZE},
    Block, ChunkCoord, ChunkData, TerrainNoise, WorldData, CHUNK_SIZE,
};

#[derive(Debug, Default, PartialEq)]
pub struct ChunkMesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub colors: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
}

impl ChunkMesh {
    pub fn into_mesh(self) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.colors);
        mesh.insert_indices(Indices::U32(self.indices));
        mesh
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }
}

const FACES: [([i32; 3], [[f32; 3]; 4]); 6] = [
    (
        [1, 0, 0],
        [[1., 0., 0.], [1., 1., 0.], [1., 1., 1.], [1., 0., 1.]],
    ),
    (
        [-1, 0, 0],
        [[0., 0., 1.], [0., 1., 1.], [0., 1., 0.], [0., 0., 0.]],
    ),
    (
        [0, 1, 0],
        [[0., 1., 0.], [0., 1., 1.], [1., 1., 1.], [1., 1., 0.]],
    ),
    (
        [0, -1, 0],
        [[0., 0., 1.], [0., 0., 0.], [1., 0., 0.], [1., 0., 1.]],
    ),
    (
        [0, 0, 1],
        [[1., 0., 1.], [1., 1., 1.], [0., 1., 1.], [0., 0., 1.]],
    ),
    (
        [0, 0, -1],
        [[0., 0., 0.], [0., 1., 0.], [1., 1., 0.], [1., 0., 0.]],
    ),
];

pub fn build_chunk_mesh(coord: ChunkCoord, world: &WorldData) -> ChunkMesh {
    let Some(chunk) = world.chunks.get(&coord) else {
        return ChunkMesh::default();
    };
    build_chunk_mesh_using(coord, &chunk.data, |pos| world.block_at(pos))
}

pub fn build_chunk_mesh_from_data(
    coord: ChunkCoord,
    chunk: &ChunkData,
    seed: u32,
    overrides: &HashMap<IVec3, Block>,
) -> ChunkMesh {
    let noise = TerrainNoise::new(seed);
    build_chunk_mesh_using(coord, chunk, |pos| {
        overrides
            .get(&pos)
            .copied()
            .unwrap_or_else(|| noise.block_at(pos))
    })
}

fn build_chunk_mesh_using(
    coord: ChunkCoord,
    chunk: &ChunkData,
    mut block_at_neighbor: impl FnMut(IVec3) -> Block,
) -> ChunkMesh {
    let mut mesh = ChunkMesh::default();
    let origin = coord.origin();
    for y in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let local = IVec3::new(x, y, z);
                let block = chunk.get(local);
                if !block.is_solid() {
                    continue;
                }
                for (face_id, (normal_i, corners)) in FACES.iter().enumerate() {
                    let normal = IVec3::new(normal_i[0], normal_i[1], normal_i[2]);
                    let neighbor_local = local + normal;
                    let neighbor_is_solid = if (0..CHUNK_SIZE).contains(&neighbor_local.x)
                        && (0..CHUNK_SIZE).contains(&neighbor_local.y)
                        && (0..CHUNK_SIZE).contains(&neighbor_local.z)
                    {
                        chunk.get(neighbor_local).is_solid()
                    } else {
                        match (
                            origin.x.checked_add(neighbor_local.x),
                            origin.y.checked_add(neighbor_local.y),
                            origin.z.checked_add(neighbor_local.z),
                        ) {
                            (Some(x), Some(y), Some(z)) => {
                                block_at_neighbor(IVec3::new(x, y, z)).is_solid()
                            }
                            _ => false,
                        }
                    };
                    if neighbor_is_solid {
                        continue;
                    }
                    let first = mesh.positions.len() as u32;
                    let color = block.face_color(face_id);
                    let texture_index = block.texture_index_for_face(face_id) as f32;
                    let texture_count = BLOCK_TEXTURE_COUNT as f32;
                    let inset = 0.5 / BLOCK_TEXTURE_SIZE as f32;
                    let u_min = (texture_index + inset) / texture_count;
                    let u_max = (texture_index + 1.0 - inset) / texture_count;
                    let v_min = inset;
                    let v_max = 1.0 - inset;
                    let rotate_log_uvs = match block {
                        Block::OakLogX => face_id >= 2,
                        Block::OakLogZ => face_id <= 1,
                        _ => false,
                    };
                    let face_uvs = if rotate_log_uvs {
                        [
                            [u_min, v_min],
                            [u_max, v_min],
                            [u_max, v_max],
                            [u_min, v_max],
                        ]
                    } else {
                        [
                            [u_min, v_min],
                            [u_min, v_max],
                            [u_max, v_max],
                            [u_max, v_min],
                        ]
                    };
                    for (corner, uv) in corners.iter().zip(face_uvs) {
                        mesh.positions.push([
                            x as f32 + corner[0],
                            y as f32 + corner[1],
                            z as f32 + corner[2],
                        ]);
                        mesh.normals.push([
                            normal_i[0] as f32,
                            normal_i[1] as f32,
                            normal_i[2] as f32,
                        ]);
                        mesh.uvs.push(uv);
                        mesh.colors.push([color.x, color.y, color.z, color.w]);
                    }
                    mesh.indices.extend_from_slice(&[
                        first,
                        first + 1,
                        first + 2,
                        first,
                        first + 2,
                        first + 3,
                    ]);
                }
            }
        }
    }
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::manager::LoadedChunk;
    use crate::world::{Block, ChunkData};

    #[test]
    fn extreme_world_coordinate_mesh_does_not_overflow() {
        let coord = ChunkCoord::from_world(IVec3::new(i32::MAX, 4, 4));
        let mut chunk = ChunkData::default();
        chunk.set(IVec3::new(15, 4, 4), Block::Stone);

        let mesh = build_chunk_mesh_from_data(coord, &chunk, 11, &HashMap::new());

        assert_eq!(mesh.indices.len(), 36);
    }

    #[test]
    fn mesh_contains_only_exposed_faces() {
        let coord = ChunkCoord::new(0, 0, 0);
        let mut chunk = ChunkData::default();
        chunk.set(IVec3::new(3, 4, 5), Block::Stone);
        chunk.set(IVec3::new(4, 4, 5), Block::Dirt);
        let mut world = WorldData::new(11, 2);
        world.chunks.insert(
            coord,
            LoadedChunk {
                data: chunk,
                entity: None,
            },
        );

        let mesh = build_chunk_mesh(coord, &world);

        assert_eq!(mesh.positions.len(), 40);
        assert_eq!(mesh.indices.len(), 60);
    }

    #[test]
    fn mesh_culls_faces_against_neighboring_chunks() {
        let coord = ChunkCoord::new(0, 2, 0);
        let mut chunk = ChunkData::default();
        chunk.set(IVec3::new(15, 8, 5), Block::Stone);
        let mut world = WorldData::new(11, 2);
        world.chunks.insert(
            coord,
            LoadedChunk {
                data: chunk,
                entity: None,
            },
        );
        world.overrides.insert(IVec3::new(16, 40, 5), Block::Dirt);

        let mesh = build_chunk_mesh(coord, &world);

        assert_eq!(mesh.indices.len(), 30);
    }

    #[test]
    fn background_mesh_matches_loaded_world_mesh() {
        let coord = ChunkCoord::new(0, 3, 0);
        let mut chunk = ChunkData::default();
        chunk.set(IVec3::new(3, 4, 5), Block::Stone);
        chunk.set(IVec3::new(4, 4, 5), Block::Dirt);
        let mut world = WorldData::new(11, 2);
        world.chunks.insert(
            coord,
            LoadedChunk {
                data: chunk.clone(),
                entity: None,
            },
        );

        let loaded_world_mesh = build_chunk_mesh(coord, &world);
        let background_mesh =
            build_chunk_mesh_from_data(coord, &chunk, world.seed, &world.overrides);

        assert_eq!(background_mesh, loaded_world_mesh);
    }
}
