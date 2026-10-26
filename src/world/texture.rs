use bevy::{
    asset::RenderAssetUsages,
    image::{Image, ImageAddressMode, ImageSampler, ImageSamplerDescriptor},
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

pub fn water_texture() -> Image {
    const SIZE: u32 = 128;
    let mut pixels = vec![0; (SIZE * SIZE * 4) as usize];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let u = x as f32 / SIZE as f32;
            let v = y as f32 / SIZE as f32;
            // Keep the diffuse texture close to white so the material tint controls
            // water color without introducing a noticeable repeating tile pattern.
            let broad = (std::f32::consts::TAU * (2.0 * u + v)).sin();
            let cross = (std::f32::consts::TAU * (-u + 2.0 * v)).cos();
            let ripple = broad * cross * 2.0;
            let index = ((y * SIZE + x) * 4) as usize;
            pixels[index] = (250.0 + ripple).clamp(0.0, 255.0) as u8;
            pixels[index + 1] = (252.0 + ripple).clamp(0.0, 255.0) as u8;
            pixels[index + 2] = (255.0 + ripple).clamp(0.0, 255.0) as u8;
            pixels[index + 3] = 255;
        }
    }

    let mut image = Image::new(
        Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let mut sampler = ImageSamplerDescriptor::linear();
    sampler.set_address_mode(ImageAddressMode::Repeat);
    image.sampler = ImageSampler::Descriptor(sampler);
    image
}

pub fn water_normal_texture() -> Image {
    const SIZE: u32 = 128;
    let mut image = Image::new(
        Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![0; (SIZE * SIZE * 4) as usize],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let mut sampler = ImageSamplerDescriptor::linear();
    sampler.set_address_mode(ImageAddressMode::Repeat);
    image.sampler = ImageSampler::Descriptor(sampler);
    update_water_normal_texture(&mut image, 0.0);
    image
}

pub(crate) fn update_water_normal_texture(image: &mut Image, time: f32) {
    let Some(pixels) = image.data.as_mut() else {
        return;
    };
    const SIZE: u32 = 128;
    let tau = std::f32::consts::TAU;
    for y in 0..SIZE {
        for x in 0..SIZE {
            let u = x as f32 / SIZE as f32;
            let v = y as f32 / SIZE as f32;
            let phase_a = tau * (2.0 * u + v) - time * 0.48;
            let phase_b = tau * (-u + 2.0 * v) + time * 0.34;
            let phase_c = tau * (3.0 * u - 2.0 * v) - time * 0.62;
            let slope_u = phase_a.cos() * 0.045 - phase_b.cos() * 0.032 + phase_c.cos() * 0.012;
            let slope_v = phase_a.cos() * 0.030 + phase_b.cos() * 0.040 - phase_c.cos() * 0.010;
            let normal = bevy::math::Vec3::new(-slope_u, 1.0, -slope_v).normalize();
            let index = ((y * SIZE + x) * 4) as usize;
            pixels[index] = ((normal.x * 0.5 + 0.5) * 255.0) as u8;
            pixels[index + 1] = ((normal.y * 0.5 + 0.5) * 255.0) as u8;
            pixels[index + 2] = ((normal.z * 0.5 + 0.5) * 255.0) as u8;
            pixels[index + 3] = 255;
        }
    }
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
        Block::Snow => (248, 4),
        Block::Gravel => (235, 28),
        Block::Obsidian => (20, 7),
        Block::GoldBlock => (244, 14),
        Block::SprucePlanks => (232, 24),
        Block::BirchPlanks => (246, 13),
        Block::Sandstone => (244, 18),
        Block::StoneBricks => (236, 24),
        Block::MossyCobblestone => (230, 35),
        Block::Clay => (238, 17),
        Block::CoalOre => (231, 23),
        Block::IronOre => (232, 24),
        Block::CopperOre => (232, 25),
        Block::DiamondOre => (238, 22),
        Block::EmeraldBlock => (244, 13),
        Block::Glass => (250, 4),
        Block::WhiteWool => (244, 10),
        Block::RedWool | Block::BlueWool => (239, 20),
        Block::Cactus => (236, 20),
        Block::OakLeaves => (232, 28),
        Block::Glowstone => (239, 18),
        Block::Netherrack => (227, 31),
        Block::Ice => (250, 5),
        Block::Pumpkin => (237, 20),
        Block::Prismarine => (234, 26),
        Block::Terracotta => (235, 20),
        Block::QuartzBlock => (248, 8),
        Block::Water => (255, 0),
    };

    let mut hash = (block as u32).wrapping_mul(0x9e37_79b9)
        ^ x.wrapping_mul(0x85eb_ca6b)
        ^ y.wrapping_mul(0xc2b2_ae35);
    hash ^= hash >> 16;
    hash = hash.wrapping_mul(0x7feb_352d);
    hash ^= hash >> 15;

    let spread = variation * 2 + 1;
    let grain = (hash % spread as u32) as i32 - variation;
    let fleck = if block == Block::Snow {
        0
    } else if hash.rotate_left(11) % 19 == 0 {
        -22
    } else if hash.rotate_left(19) % 23 == 0 {
        10
    } else {
        0
    };
    let grain_value = if block == Block::Obsidian {
        (base + grain + fleck).clamp(8, 36)
    } else {
        (base + grain + fleck).clamp(190, 255)
    };
    match block {
        Block::OakPlanks | Block::SprucePlanks | Block::BirchPlanks => {
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
        Block::Sandstone => {
            if y % 5 == 0 {
                177
            } else if y % 5 == 1 {
                (grain_value + 4).clamp(190, 255) as u8
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
        Block::StoneBricks | Block::MossyCobblestone => {
            let row_offset = (y / 5) % 2 * 4;
            let seam = y % 5 == 0 || (x + row_offset) % 8 == 0;
            let moss = block == Block::MossyCobblestone && hash.rotate_left(7) % 5 == 0;
            if seam {
                160
            } else if moss {
                (grain_value - 36).clamp(160, 255) as u8
            } else {
                grain_value as u8
            }
        }
        Block::CoalOre | Block::IronOre | Block::CopperOre | Block::DiamondOre => {
            let ore_spot = hash.rotate_left(9) % 9 == 0 || hash.rotate_left(17) % 23 == 0;
            if ore_spot {
                match block {
                    Block::CoalOre => 82,
                    Block::IronOre => 177,
                    Block::CopperOre => 166,
                    Block::DiamondOre => 206,
                    _ => grain_value as u8,
                }
            } else {
                grain_value as u8
            }
        }
        Block::Snow => {
            let cell_x = (x + (y / 4) % 2) / 2;
            let cell_y = y / 2;
            let mut patch_hash =
                cell_x.wrapping_mul(0x9e37_79b9) ^ cell_y.wrapping_mul(0x85eb_ca6b);
            patch_hash ^= patch_hash >> 16;
            patch_hash = patch_hash.wrapping_mul(0x7feb_352d);
            patch_hash ^= patch_hash >> 15;
            let patch = (patch_hash % 7) as i32 - 3;
            if hash.rotate_left(9) % 127 == 0 {
                228
            } else {
                (grain_value + patch).clamp(238, 255) as u8
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
                55
            } else if hash.rotate_left(7) % 53 == 0 {
                72
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
        Block::Glass | Block::Ice => {
            if x % 8 == 0 || y % 8 == 0 {
                224
            } else if block == Block::Ice && (x + y * 3) % 17 == 0 {
                239
            } else {
                grain_value as u8
            }
        }
        Block::Cactus => {
            if x % 7 == 0 {
                179
            } else if y % 6 == 0 {
                (grain_value + 8).clamp(190, 255) as u8
            } else {
                grain_value as u8
            }
        }
        Block::OakLeaves => {
            if hash.rotate_left(5) % 4 == 0 {
                (grain_value - 38).clamp(170, 255) as u8
            } else {
                grain_value as u8
            }
        }
        Block::Glowstone | Block::Prismarine => {
            let cell = (x / 4 + (y / 4) * 4).wrapping_mul(37);
            let cell_shade = (cell % 17) as i32 - 8;
            if x % 4 == 0 || y % 4 == 0 {
                (grain_value - 24).clamp(180, 255) as u8
            } else {
                (grain_value + cell_shade).clamp(190, 255) as u8
            }
        }
        Block::Netherrack => {
            if hash.rotate_left(11) % 7 == 0 {
                (grain_value - 42).clamp(170, 255) as u8
            } else {
                grain_value as u8
            }
        }
        Block::Pumpkin => {
            if x % 4 == 0 {
                (grain_value - 31).clamp(175, 255) as u8
            } else {
                grain_value as u8
            }
        }
        Block::QuartzBlock => {
            if y % 6 == 0 {
                (grain_value - 20).clamp(190, 255) as u8
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
