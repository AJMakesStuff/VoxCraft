use bevy::prelude::IVec3;
use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

use super::{Block, CHUNK_SIZE, ChunkCoord, ChunkData};

pub const WORLD_HEIGHT: i32 = 48;

pub(crate) struct TerrainNoise {
    hills: Fbm<Perlin>,
    detail: Fbm<Perlin>,
    dry: Fbm<Perlin>,
}

impl TerrainNoise {
    pub(crate) fn new(seed: u32) -> Self {
        Self {
            hills: Fbm::<Perlin>::new(seed).set_octaves(4).set_frequency(0.008),
            detail: Fbm::<Perlin>::new(seed.wrapping_add(1_337))
                .set_octaves(2)
                .set_frequency(0.035),
            dry: Fbm::<Perlin>::new(seed ^ 0x5eed)
                .set_octaves(2)
                .set_frequency(0.003),
        }
    }

    pub(crate) fn terrain_height(&self, x: i32, z: i32) -> i32 {
        let point = [x as f64, z as f64];
        let hills = self.hills.get(point);
        let detail = self.detail.get(point);
        (16.0 + hills * 10.0 + detail * 2.0)
            .round()
            .clamp(5.0, 32.0) as i32
    }

    fn is_sandy(&self, x: i32, z: i32, height: i32) -> bool {
        height <= 9 || self.dry.get([x as f64, z as f64]) > 0.42
    }

    pub(crate) fn block_at(&self, pos: IVec3) -> Block {
        if pos.y < 0 || pos.y >= WORLD_HEIGHT {
            return Block::Air;
        }
        if pos.y == 0 {
            return Block::Bedrock;
        }
        let height = self.terrain_height(pos.x, pos.z);
        if pos.y > height {
            return Block::Air;
        }
        if self.is_sandy(pos.x, pos.z, height) {
            return if pos.y >= height - 2 {
                Block::Sand
            } else {
                Block::Stone
            };
        }
        if pos.y == height {
            Block::Grass
        } else if pos.y >= height - 3 {
            Block::Dirt
        } else {
            Block::Stone
        }
    }
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
            let height = noise.terrain_height(wx, wz);
            let sandy = noise.is_sandy(wx, wz, height);
            for wy in min_y..max_y {
                let block = if wy == 0 {
                    Block::Bedrock
                } else if wy > height {
                    Block::Air
                } else if sandy && wy >= height - 2 {
                    Block::Sand
                } else if sandy {
                    Block::Stone
                } else if wy == height {
                    Block::Grass
                } else if wy >= height - 3 {
                    Block::Dirt
                } else {
                    Block::Stone
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
        let sandy = TerrainNoise::new(77).is_sandy(-3, 37, h);
        let noise = TerrainNoise::new(77);
        assert_eq!(
            noise.block_at(IVec3::new(-3, h, 37)),
            if sandy { Block::Sand } else { Block::Grass }
        );
        assert_eq!(noise.block_at(IVec3::new(0, 0, 0)), Block::Bedrock);
        assert_eq!(noise.block_at(IVec3::new(0, WORLD_HEIGHT, 0)), Block::Air);
    }
}
