use bevy::{
    asset::RenderAssetUsages,
    image::{Image, ImageSampler},
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use super::Block;

pub const BLOCK_TEXTURE_SIZE: u32 = 16;
pub const BLOCK_TEXTURE_COUNT: u32 = Block::SOLID_PICKER.len() as u32 + 1;
pub const BLOCK_ATLAS_WIDTH: u32 = BLOCK_TEXTURE_SIZE * BLOCK_TEXTURE_COUNT;

pub fn block_texture_atlas() -> Image {
    let mut pixels = vec![0; (BLOCK_ATLAS_WIDTH * BLOCK_TEXTURE_SIZE * 4) as usize];

    for (tile, block) in Block::SOLID_PICKER.into_iter().enumerate() {
        for y in 0..BLOCK_TEXTURE_SIZE {
            for x in 0..BLOCK_TEXTURE_SIZE {
                let value = block_texture_pixel(block, x, y);
                let index =
                    ((y * BLOCK_ATLAS_WIDTH + tile as u32 * BLOCK_TEXTURE_SIZE + x) * 4) as usize;
                pixels[index] = value;
                pixels[index + 1] = value;
                pixels[index + 2] = value;
                pixels[index + 3] = 255;
            }
        }
    }

    let end_tile = Block::SOLID_PICKER.len() as u32;
    for y in 0..BLOCK_TEXTURE_SIZE {
        for x in 0..BLOCK_TEXTURE_SIZE {
            let value = oak_log_end_pixel(x, y);
            let index = ((y * BLOCK_ATLAS_WIDTH + end_tile * BLOCK_TEXTURE_SIZE + x) * 4) as usize;
            pixels[index] = value;
            pixels[index + 1] = value;
            pixels[index + 2] = value;
            pixels[index + 3] = 255;
        }
    }

    let mut image = Image::new(
        Extent3d {
            width: BLOCK_ATLAS_WIDTH,
            height: BLOCK_TEXTURE_SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::nearest();
    image
}

pub(crate) fn block_texture_pixel(block: Block, x: u32, y: u32) -> u8 {
    let (base, variation) = match block {
        Block::Air => (244, 0),
        Block::Grass => (244, 20),
        Block::Dirt => (239, 27),
        Block::Stone => (238, 25),
        Block::Sand => (246, 11),
        Block::Bedrock => (228, 34),
        Block::OakPlanks => (242, 24),
        Block::OakLog | Block::OakLogX | Block::OakLogZ => (232, 30),
        Block::Cobblestone => (232, 32),
        Block::Bricks => (238, 22),
        Block::Snow => (252, 5),
        Block::Gravel => (235, 28),
        Block::Obsidian => (226, 30),
        Block::GoldBlock => (244, 14),
    };

    let mut hash = (block as u32).wrapping_mul(0x9e37_79b9)
        ^ x.wrapping_mul(0x85eb_ca6b)
        ^ y.wrapping_mul(0xc2b2_ae35);
    hash ^= hash >> 16;
    hash = hash.wrapping_mul(0x7feb_352d);
    hash ^= hash >> 15;

    let spread = variation * 2 + 1;
    let grain = (hash % spread as u32) as i32 - variation;
    let fleck = if hash.rotate_left(11) % 19 == 0 {
        -22
    } else if hash.rotate_left(19) % 23 == 0 {
        10
    } else {
        0
    };
    let grain_value = (base + grain + fleck).clamp(190, 255);
    match block {
        Block::OakPlanks => {
            let plank_row = y / 5;
            let row_pixel = y % 5;
            let joint_x = (plank_row * 5 + 4) % BLOCK_TEXTURE_SIZE;
            if row_pixel == 0 || (x == joint_x && row_pixel > 0) {
                150
            } else if row_pixel == 1 {
                (grain_value + 8).clamp(190, 255) as u8
            } else {
                grain_value as u8
            }
        }
        Block::OakLog | Block::OakLogX | Block::OakLogZ => match (x + (y / 4) % 2) % 5 {
            0 => 157,
            1 => 188,
            _ if hash.rotate_left(13) % 17 == 0 => 218,
            _ => grain_value as u8,
        },
        Block::Cobblestone => {
            let row_offset = (y / 4) % 2 * 2;
            let local_x = (x + row_offset) % 4;
            let local_y = y % 4;
            if local_x == 0 || local_y == 0 {
                158
            } else {
                let cell_hash = (x / 4 + y / 4 * 4 + row_offset).wrapping_mul(37);
                (grain_value + (cell_hash % 25) as i32 - 12).clamp(190, 255) as u8
            }
        }
        Block::Bricks => {
            let brick_row = y / 4;
            let mortar_x = (x + if brick_row % 2 == 0 { 0 } else { 4 }) % 8;
            if y % 4 == 0 || mortar_x == 0 {
                158
            } else if y % 4 == 1 {
                (grain_value + 7).clamp(190, 255) as u8
            } else {
                grain_value as u8
            }
        }
        Block::Snow => {
            if hash % 41 == 0 {
                214
            } else if hash.rotate_left(9) % 19 == 0 {
                239
            } else {
                grain_value as u8
            }
        }
        Block::Gravel => {
            if hash % 7 == 0 {
                (grain_value - 34).clamp(170, 255) as u8
            } else {
                grain_value as u8
            }
        }
        Block::Obsidian => {
            let vein = (x * 3 + y * 5 + (hash % 3)) % 13 == 0;
            if vein {
                177
            } else if hash.rotate_left(7) % 53 == 0 {
                255
            } else {
                grain_value as u8
            }
        }
        Block::GoldBlock => {
            let edge = x % 8 == 0 || y % 8 == 0;
            let highlight = x % 8 == 1 || y % 8 == 1;
            if edge {
                171
            } else if highlight {
                255
            } else {
                grain_value as u8
            }
        }
        _ => grain_value as u8,
    }
}

pub(crate) fn block_texture_pixel_for_face(block: Block, face: usize, x: u32, y: u32) -> u8 {
    if block.is_oak_log_end_face(face) {
        oak_log_end_pixel(x, y)
    } else {
        block_texture_pixel(block, x, y)
    }
}

fn oak_log_end_pixel(x: u32, y: u32) -> u8 {
    let dx = (x as i32 - 7).abs();
    let dy = (y as i32 - 7).abs();
    let ring = dx.max(dy);
    if ring >= 7 {
        158
    } else if ring == 0 {
        250
    } else if ring % 3 == 0 {
        180
    } else if ring % 3 == 1 {
        242
    } else {
        211
    }
}
