use bevy::prelude::IVec3;

use super::Block;

pub const CHUNK_SIZE: i32 = 16;
pub const CHUNK_VOLUME: usize = (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ChunkCoord(pub IVec3);

impl ChunkCoord {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self(IVec3::new(x, y, z))
    }
    pub fn origin(self) -> IVec3 {
        self.0 * CHUNK_SIZE
    }
    pub fn from_world(pos: IVec3) -> Self {
        Self(chunk_from_world(pos))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChunkData {
    blocks: Box<[Block; CHUNK_VOLUME]>,
}

impl Default for ChunkData {
    fn default() -> Self {
        Self {
            blocks: Box::new([Block::Air; CHUNK_VOLUME]),
        }
    }
}

impl ChunkData {
    pub fn get(&self, local: IVec3) -> Block {
        self.blocks[index(local)].to_owned()
    }

    pub fn set(&mut self, local: IVec3, block: Block) {
        self.blocks[index(local)] = block;
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.iter().all(|block| *block == Block::Air)
    }
}

pub fn chunk_from_world(pos: IVec3) -> IVec3 {
    IVec3::new(
        pos.x.div_euclid(CHUNK_SIZE),
        pos.y.div_euclid(CHUNK_SIZE),
        pos.z.div_euclid(CHUNK_SIZE),
    )
}

pub fn local_from_world(pos: IVec3) -> IVec3 {
    IVec3::new(
        pos.x.rem_euclid(CHUNK_SIZE),
        pos.y.rem_euclid(CHUNK_SIZE),
        pos.z.rem_euclid(CHUNK_SIZE),
    )
}

pub fn index(local: IVec3) -> usize {
    let x = local.x as usize;
    let y = local.y as usize;
    let z = local.z as usize;
    x + CHUNK_SIZE as usize * (z + CHUNK_SIZE as usize * y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negative_world_coordinates_use_euclidean_chunk_coordinates() {
        assert_eq!(
            chunk_from_world(IVec3::new(-1, 0, -17)),
            IVec3::new(-1, 0, -2)
        );
        assert_eq!(
            local_from_world(IVec3::new(-1, 0, -17)),
            IVec3::new(15, 0, 15)
        );
    }

    #[test]
    fn chunk_boundaries_map_to_adjacent_local_cells() {
        assert_eq!(chunk_from_world(IVec3::new(15, 0, 0)).x, 0);
        assert_eq!(local_from_world(IVec3::new(15, 0, 0)).x, 15);
        assert_eq!(chunk_from_world(IVec3::new(16, 0, 0)).x, 1);
        assert_eq!(local_from_world(IVec3::new(16, 0, 0)).x, 0);
        assert_eq!(chunk_from_world(IVec3::new(-16, 0, 0)).x, -1);
        assert_eq!(local_from_world(IVec3::new(-16, 0, 0)).x, 0);
    }

    #[test]
    fn storage_round_trips_each_axis() {
        let mut chunk = ChunkData::default();
        let local = IVec3::new(15, 7, 3);
        chunk.set(local, Block::Sand);
        assert_eq!(chunk.get(local), Block::Sand);
    }
}
