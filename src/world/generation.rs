use bevy::prelude::IVec3;
use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

use super::{Block, CHUNK_SIZE, ChunkCoord, ChunkData};

pub const WORLD_HEIGHT: i32 = 48;
pub const WATER_LEVEL: i32 = 13;

const LAKE_CELL_SIZE: i32 = 72;

pub(crate) struct TerrainNoise {
    hills: Fbm<Perlin>,
    detail: Fbm<Perlin>,
    climate: Fbm<Perlin>,
    mountain_regions: Fbm<Perlin>,
    mountain_detail: Fbm<Perlin>,
    continents: Fbm<Perlin>,
    spawn_continentalness: f64,
    seed: u32,
}

#[derive(Clone, Copy)]
struct TerrainColumn {
    height: i32,
    surface: Block,
    subsurface: Block,
    subsurface_depth: i32,
    water_level: Option<i32>,
}

impl TerrainColumn {
    fn block_at(self, y: i32) -> Block {
        if y == self.height {
            self.surface
        } else if y < self.height && y >= self.height - self.subsurface_depth {
            self.subsurface
        } else if y < self.height {
            Block::Stone
        } else if self.water_level.is_some_and(|level| y <= level) {
            Block::Water
        } else {
            Block::Air
        }
    }
}

impl TerrainNoise {
    pub(crate) fn new(seed: u32) -> Self {
        let continents = Fbm::<Perlin>::new(seed ^ 0x0cea_71e5)
            .set_octaves(2)
            .set_frequency(0.0042);
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
            spawn_continentalness: continents.get([0.0, 0.0]),
            continents,
            seed,
        }
    }

    fn terrain_column(&self, x: i32, z: i32) -> TerrainColumn {
        let mut column = self.base_terrain_column(x, z);
        let ocean = self.ocean_strength(x, z);
        if ocean > 0.0 {
            column.height = (column.height - (ocean * 22.0).round() as i32).clamp(4, 45);
        }

        let mut water_level = (column.height < WATER_LEVEL).then_some(WATER_LEVEL);
        if ocean > 0.18 && column.height <= WATER_LEVEL - 2 {
            column.surface = if ocean > 0.72 {
                Block::Gravel
            } else {
                Block::Sand
            };
            column.subsurface = column.surface;
            column.subsurface_depth = 3;
        }

        if water_level.is_none()
            && ocean < 0.08
            && let Some((distance, radius, lake_level)) = self.lake_at(x, z)
        {
            let basin = 1.0 - distance / radius as f64;
            let basin_floor = lake_level - 1 - (basin * 4.0).round() as i32;
            column.height = column.height.min(basin_floor);
            column.surface = Block::Sand;
            column.subsurface = Block::Sand;
            column.subsurface_depth = 2;
            water_level = Some(lake_level);
        }
        column.water_level = water_level;
        column
    }

    fn base_terrain_column(&self, x: i32, z: i32) -> TerrainColumn {
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
            water_level: None,
        }
    }

    fn ocean_strength(&self, x: i32, z: i32) -> f64 {
        let point = [x as f64, z as f64];
        let continentalness = self.continents.get(point) - self.spawn_continentalness;
        smoothstep(-0.12, -0.46, continentalness)
    }

    fn lake_at(&self, x: i32, z: i32) -> Option<(f64, i32, i32)> {
        let cell_x = x.div_euclid(LAKE_CELL_SIZE);
        let cell_z = z.div_euclid(LAKE_CELL_SIZE);
        let mut nearest = None;

        for dz in -1..=1 {
            for dx in -1..=1 {
                let feature_x = cell_x + dx;
                let feature_z = cell_z + dz;
                let hash = lake_hash(self.seed, feature_x, feature_z);
                if hash % 100 >= 38 {
                    continue;
                }

                let center_x = feature_x as i64 * LAKE_CELL_SIZE as i64
                    + ((hash >> 8) % LAKE_CELL_SIZE as u32) as i64;
                let center_z = feature_z as i64 * LAKE_CELL_SIZE as i64
                    + ((hash.rotate_left(13) >> 8) % LAKE_CELL_SIZE as u32) as i64;
                if !(i32::MIN as i64..=i32::MAX as i64).contains(&center_x)
                    || !(i32::MIN as i64..=i32::MAX as i64).contains(&center_z)
                {
                    continue;
                }
                let center_x = center_x as i32;
                let center_z = center_z as i32;
                let radius = 7 + ((hash.rotate_left(21) >> 12) % 10) as i32;
                let center = self.base_terrain_column(center_x, center_z);
                if !(WATER_LEVEL + 4..=28).contains(&center.height)
                    || self.ocean_strength(center_x, center_z) > 0.05
                {
                    continue;
                }

                let offset_x = x as i64 - center_x as i64;
                let offset_z = z as i64 - center_z as i64;
                let distance = ((offset_x * offset_x + offset_z * offset_z) as f64).sqrt();
                if distance >= radius as f64 {
                    continue;
                }

                let lake_level = (center.height - 2).clamp(WATER_LEVEL + 1, WATER_LEVEL + 7);
                if nearest.is_none_or(|(best_distance, _, _)| distance < best_distance) {
                    nearest = Some((distance, radius, lake_level));
                }
            }
        }

        nearest
    }

    #[allow(dead_code)]
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

#[allow(dead_code)]
pub fn terrain_height(seed: u32, x: i32, z: i32) -> i32 {
    TerrainNoise::new(seed).terrain_height(x, z)
}

pub fn terrain_spawn_height(seed: u32, x: i32, z: i32) -> i32 {
    let column = TerrainNoise::new(seed).terrain_column(x, z);
    column
        .water_level
        .map_or(column.height, |level| level.max(column.height))
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

fn lake_hash(seed: u32, cell_x: i32, cell_z: i32) -> u32 {
    let mut value = seed
        ^ (cell_x as u32).wrapping_mul(0x9e37_79b9)
        ^ (cell_z as u32).wrapping_mul(0x85eb_ca6b);
    value ^= value >> 16;
    value = value.wrapping_mul(0x7feb_352d);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846c_a68b);
    value ^ (value >> 16)
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
