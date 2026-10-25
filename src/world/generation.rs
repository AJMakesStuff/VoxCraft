use bevy::prelude::IVec3;
use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

use super::{Block, CHUNK_SIZE, ChunkCoord, ChunkData};

pub const WORLD_HEIGHT: i32 = 48;

pub(crate) struct TerrainNoise {
    hills: Fbm<Perlin>,
    detail: Fbm<Perlin>,
    climate: Fbm<Perlin>,
    mountain_regions: Fbm<Perlin>,
    mountain_detail: Fbm<Perlin>,
}

#[derive(Clone, Copy)]
struct TerrainColumn {
    height: i32,
    surface: Block,
    subsurface: Block,
    subsurface_depth: i32,
}

impl TerrainColumn {
    fn block_at(self, y: i32) -> Block {
        if y > self.height {
            Block::Air
        } else if y == self.height {
            self.surface
        } else if y >= self.height - self.subsurface_depth {
            self.subsurface
        } else {
            Block::Stone
        }
    }
}

impl TerrainNoise {
    pub(crate) fn new(seed: u32) -> Self {
        Self {
            hills: Fbm::<Perlin>::new(seed)
                .set_octaves(3)
                .set_frequency(0.0045),
            detail: Fbm::<Perlin>::new(seed.wrapping_add(1_337))
                .set_octaves(2)
                .set_frequency(0.04),
            climate: Fbm::<Perlin>::new(seed ^ 0x5eed)
                .set_octaves(3)
                .set_frequency(0.0018),
            mountain_regions: Fbm::<Perlin>::new(seed.wrapping_add(82_451))
                .set_octaves(2)
                .set_frequency(0.0025),
            mountain_detail: Fbm::<Perlin>::new(seed ^ 0x6d6f_756e)
                .set_octaves(3)
                .set_frequency(0.006),
        }
    }

    fn terrain_column(&self, x: i32, z: i32) -> TerrainColumn {
        let point = [x as f64, z as f64];
        let desert = smoothstep(0.34, 0.66, self.climate.get(point));
        let mountain = smoothstep(0.60, 0.80, self.mountain_regions.get(point));
        let ridge = 1.0 - self.mountain_detail.get(point).abs();
        let height = (15.0 + self.hills.get(point) * 5.5 + self.detail.get(point) * 1.5
            - desert * 1.0
            + mountain * (7.0 + ridge * 17.0))
            .round()
            .clamp(6.0, 45.0) as i32;

        let (surface, subsurface, subsurface_depth) = if mountain > 0.18 {
            if height >= 35 {
                (Block::Snow, Block::Stone, 1)
            } else if mountain > 0.72 {
                (Block::Stone, Block::Stone, 0)
            } else {
                (Block::Gravel, Block::Stone, 2)
            }
        } else if desert > 0.55 {
            (Block::Sand, Block::Sand, 3)
        } else {
            (Block::Grass, Block::Dirt, 3)
        };

        TerrainColumn {
            height,
            surface,
            subsurface,
            subsurface_depth,
        }
    }

    pub(crate) fn terrain_height(&self, x: i32, z: i32) -> i32 {
        self.terrain_column(x, z).height
    }

    pub(crate) fn block_at(&self, pos: IVec3) -> Block {
        if pos.y < 0 || pos.y >= WORLD_HEIGHT {
            return Block::Air;
        }
        if pos.y == 0 {
            return Block::Bedrock;
        }
        self.terrain_column(pos.x, pos.z).block_at(pos.y)
    }
}

fn smoothstep(edge0: f64, edge1: f64, value: f64) -> f64 {
    let t = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub fn terrain_height(seed: u32, x: i32, z: i32) -> i32 {
    TerrainNoise::new(seed).terrain_height(x, z)
}

pub fn generate_chunk(seed: u32, coord: ChunkCoord) -> ChunkData {
    let mut chunk = ChunkData::default();
    let origin = coord.origin();
    let min_y = origin.y.max(0);
    let max_y = (origin.y + CHUNK_SIZE).min(WORLD_HEIGHT);
    if min_y >= max_y {
        return chunk;
    }
    let noise = TerrainNoise::new(seed);
    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            let wx = origin.x + x;
            let wz = origin.z + z;
            let column = noise.terrain_column(wx, wz);
            for wy in min_y..max_y {
                let block = if wy == 0 {
                    Block::Bedrock
                } else {
                    column.block_at(wy)
                };
                chunk.set(IVec3::new(x, wy - origin.y, z), block);
            }
        }
    }
    chunk
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_negative_y_chunk_matches_point_query() {
        let coord = ChunkCoord::new(0, -1, 0);
        let chunk = generate_chunk(77, coord);
        let noise = TerrainNoise::new(77);
        let origin = coord.origin();

        for z in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let world_pos = origin + IVec3::new(x, y, z);
                    let generated_block: Block = chunk.get(IVec3::new(x, y, z));
                    assert_eq!(
                        generated_block,
                        noise.block_at(world_pos),
                        "block mismatch at {world_pos:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn generation_is_seeded_and_surface_layers_are_consistent() {
        let a = generate_chunk(77, ChunkCoord::new(-1, 0, 2));
        let b = generate_chunk(77, ChunkCoord::new(-1, 0, 2));
        assert_eq!(a, b);
        let h = terrain_height(77, -3, 37);
        let noise = TerrainNoise::new(77);
        assert_eq!(
            noise.block_at(IVec3::new(-3, h, 37)),
            noise.terrain_column(-3, 37).surface
        );
        assert_eq!(noise.block_at(IVec3::new(0, 0, 0)), Block::Bedrock);
        assert_eq!(noise.block_at(IVec3::new(0, WORLD_HEIGHT, 0)), Block::Air);
    }
}
