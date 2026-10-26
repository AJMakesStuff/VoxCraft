use bevy::prelude::{IVec3, Vec4};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Block {
    #[default]
    Air = 0,
    Grass = 1,
    Dirt = 2,
    Stone = 3,
    Sand = 4,
    Bedrock = 5,
    OakPlanks = 6,
    OakLog = 7,
    Cobblestone = 8,
    Bricks = 9,
    Snow = 10,
    Gravel = 11,
    Obsidian = 12,
    GoldBlock = 13,
    OakLogX = 14,
    OakLogZ = 15,
    Water = 16,
    SprucePlanks = 17,
    BirchPlanks = 18,
    Sandstone = 19,
    StoneBricks = 20,
    MossyCobblestone = 21,
    Clay = 22,
    CoalOre = 23,
    IronOre = 24,
    CopperOre = 25,
    DiamondOre = 26,
    EmeraldBlock = 27,
    Glass = 28,
    WhiteWool = 29,
    RedWool = 30,
    BlueWool = 31,
    Cactus = 32,
    OakLeaves = 33,
    Glowstone = 34,
    Netherrack = 35,
    Ice = 36,
    Pumpkin = 37,
    Prismarine = 38,
    Terracotta = 39,
    QuartzBlock = 40,
}

impl Block {
    pub const SOLID_PICKER: [Self; 37] = [
        Self::Grass,
        Self::Dirt,
        Self::Stone,
        Self::Sand,
        Self::Bedrock,
        Self::OakPlanks,
        Self::OakLog,
        Self::Cobblestone,
        Self::Bricks,
        Self::Snow,
        Self::Gravel,
        Self::Obsidian,
        Self::GoldBlock,
        Self::SprucePlanks,
        Self::BirchPlanks,
        Self::Sandstone,
        Self::StoneBricks,
        Self::MossyCobblestone,
        Self::Clay,
        Self::CoalOre,
        Self::IronOre,
        Self::CopperOre,
        Self::DiamondOre,
        Self::EmeraldBlock,
        Self::Glass,
        Self::WhiteWool,
        Self::RedWool,
        Self::BlueWool,
        Self::Cactus,
        Self::OakLeaves,
        Self::Glowstone,
        Self::Netherrack,
        Self::Ice,
        Self::Pumpkin,
        Self::Prismarine,
        Self::Terracotta,
        Self::QuartzBlock,
    ];

    pub const fn is_solid(self) -> bool {
        !matches!(self, Self::Air | Self::Water)
    }

    pub const fn is_opaque(self) -> bool {
        !matches!(self, Self::Air | Self::Water | Self::Glass)
    }

    pub const fn from_id(id: u8) -> Option<Self> {
        match id {
            0 => Some(Self::Air),
            1 => Some(Self::Grass),
            2 => Some(Self::Dirt),
            3 => Some(Self::Stone),
            4 => Some(Self::Sand),
            5 => Some(Self::Bedrock),
            6 => Some(Self::OakPlanks),
            7 => Some(Self::OakLog),
            8 => Some(Self::Cobblestone),
            9 => Some(Self::Bricks),
            10 => Some(Self::Snow),
            11 => Some(Self::Gravel),
            12 => Some(Self::Obsidian),
            13 => Some(Self::GoldBlock),
            14 => Some(Self::OakLogX),
            15 => Some(Self::OakLogZ),
            16 => Some(Self::Water),
            17 => Some(Self::SprucePlanks),
            18 => Some(Self::BirchPlanks),
            19 => Some(Self::Sandstone),
            20 => Some(Self::StoneBricks),
            21 => Some(Self::MossyCobblestone),
            22 => Some(Self::Clay),
            23 => Some(Self::CoalOre),
            24 => Some(Self::IronOre),
            25 => Some(Self::CopperOre),
            26 => Some(Self::DiamondOre),
            27 => Some(Self::EmeraldBlock),
            28 => Some(Self::Glass),
            29 => Some(Self::WhiteWool),
            30 => Some(Self::RedWool),
            31 => Some(Self::BlueWool),
            32 => Some(Self::Cactus),
            33 => Some(Self::OakLeaves),
            34 => Some(Self::Glowstone),
            35 => Some(Self::Netherrack),
            36 => Some(Self::Ice),
            37 => Some(Self::Pumpkin),
            38 => Some(Self::Prismarine),
            39 => Some(Self::Terracotta),
            40 => Some(Self::QuartzBlock),
            _ => None,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Air => "Air",
            Self::Grass => "Grass",
            Self::Dirt => "Dirt",
            Self::Stone => "Stone",
            Self::Sand => "Sand",
            Self::Bedrock => "Bedrock",
            Self::OakPlanks => "Oak Planks",
            Self::OakLog => "Oak Log",
            Self::OakLogX | Self::OakLogZ => "Oak Log",
            Self::Water => "Water",
            Self::Cobblestone => "Cobblestone",
            Self::Bricks => "Bricks",
            Self::Snow => "Snow",
            Self::Gravel => "Gravel",
            Self::Obsidian => "Obsidian",
            Self::GoldBlock => "Gold Block",
            Self::SprucePlanks => "Spruce Planks",
            Self::BirchPlanks => "Birch Planks",
            Self::Sandstone => "Sandstone",
            Self::StoneBricks => "Stone Bricks",
            Self::MossyCobblestone => "Mossy Cobble",
            Self::Clay => "Clay",
            Self::CoalOre => "Coal Ore",
            Self::IronOre => "Iron Ore",
            Self::CopperOre => "Copper Ore",
            Self::DiamondOre => "Diamond Ore",
            Self::EmeraldBlock => "Emerald Block",
            Self::Glass => "Glass",
            Self::WhiteWool => "White Wool",
            Self::RedWool => "Red Wool",
            Self::BlueWool => "Blue Wool",
            Self::Cactus => "Cactus",
            Self::OakLeaves => "Oak Leaves",
            Self::Glowstone => "Glowstone",
            Self::Netherrack => "Netherrack",
            Self::Ice => "Ice",
            Self::Pumpkin => "Pumpkin",
            Self::Prismarine => "Prismarine",
            Self::Terracotta => "Terracotta",
            Self::QuartzBlock => "Quartz Block",
        }
    }

    pub const fn texture_index(self) -> usize {
        match self {
            Self::Air | Self::Grass | Self::Water => 0,
            Self::Dirt => 1,
            Self::Stone => 2,
            Self::Sand => 3,
            Self::Bedrock => 4,
            Self::OakPlanks => 5,
            Self::OakLog | Self::OakLogX | Self::OakLogZ => 6,
            Self::Cobblestone => 7,
            Self::Bricks => 8,
            Self::Snow => 9,
            Self::Gravel => 10,
            Self::Obsidian => 11,
            Self::GoldBlock => 12,
            Self::SprucePlanks => 13,
            Self::BirchPlanks => 14,
            Self::Sandstone => 15,
            Self::StoneBricks => 16,
            Self::MossyCobblestone => 17,
            Self::Clay => 18,
            Self::CoalOre => 19,
            Self::IronOre => 20,
            Self::CopperOre => 21,
            Self::DiamondOre => 22,
            Self::EmeraldBlock => 23,
            Self::Glass => 24,
            Self::WhiteWool => 25,
            Self::RedWool => 26,
            Self::BlueWool => 27,
            Self::Cactus => 28,
            Self::OakLeaves => 29,
            Self::Glowstone => 30,
            Self::Netherrack => 31,
            Self::Ice => 32,
            Self::Pumpkin => 33,
            Self::Prismarine => 34,
            Self::Terracotta => 35,
            Self::QuartzBlock => 36,
        }
    }

    pub const fn texture_index_for_face(self, face: usize) -> usize {
        if self.is_oak_log_end_face(face) {
            Self::SOLID_PICKER.len()
        } else {
            self.texture_index()
        }
    }

    pub const fn is_oak_log_end_face(self, face: usize) -> bool {
        match self {
            Self::OakLog => face == 2 || face == 3,
            Self::OakLogX => face == 0 || face == 1,
            Self::OakLogZ => face == 4 || face == 5,
            _ => false,
        }
    }

    pub const fn oriented_for_placement(self, face_normal: IVec3) -> Self {
        if !matches!(self, Self::OakLog) {
            return self;
        }
        if face_normal.x != 0 {
            Self::OakLogX
        } else if face_normal.z != 0 {
            Self::OakLogZ
        } else {
            Self::OakLog
        }
    }

    pub fn face_color(self, face: usize) -> Vec4 {
        let (r, g, b) = match self {
            Self::Air => (0.0, 0.0, 0.0),
            Self::Grass if face == 2 => (0.07, 0.17, 0.055),
            Self::Grass if face == 3 => (0.43, 0.29, 0.17),
            Self::Grass => (0.11, 0.22, 0.08),
            Self::Dirt => (0.48, 0.31, 0.18),
            Self::Stone => (0.48, 0.50, 0.52),
            Self::Sand => (0.82, 0.74, 0.48),
            Self::Bedrock => (0.23, 0.24, 0.25),
            Self::OakPlanks => (0.62, 0.42, 0.20),
            Self::OakLog if face == 2 || face == 3 => (0.69, 0.51, 0.29),
            Self::OakLogX if face == 0 || face == 1 => (0.69, 0.51, 0.29),
            Self::OakLogZ if face == 4 || face == 5 => (0.69, 0.51, 0.29),
            Self::OakLog | Self::OakLogX | Self::OakLogZ => (0.39, 0.24, 0.12),
            Self::Cobblestone => (0.40, 0.41, 0.42),
            Self::Bricks => (0.62, 0.24, 0.18),
            Self::Snow => (0.88, 0.91, 0.96),
            Self::Gravel => (0.50, 0.48, 0.43),
            Self::Obsidian => (0.16, 0.14, 0.22),
            Self::GoldBlock => (0.92, 0.67, 0.12),
            Self::SprucePlanks => (0.40, 0.25, 0.13),
            Self::BirchPlanks => (0.78, 0.66, 0.43),
            Self::Sandstone => (0.76, 0.65, 0.42),
            Self::StoneBricks => (0.48, 0.50, 0.51),
            Self::MossyCobblestone => (0.35, 0.43, 0.32),
            Self::Clay => (0.57, 0.58, 0.56),
            Self::CoalOre => (0.31, 0.32, 0.32),
            Self::IronOre => (0.58, 0.46, 0.36),
            Self::CopperOre => (0.62, 0.34, 0.20),
            Self::DiamondOre => (0.10, 0.69, 0.70),
            Self::EmeraldBlock => (0.10, 0.54, 0.29),
            Self::Glass => (0.68, 0.82, 0.87),
            Self::WhiteWool => (0.87, 0.84, 0.78),
            Self::RedWool => (0.70, 0.16, 0.13),
            Self::BlueWool => (0.16, 0.31, 0.68),
            Self::Cactus => (0.18, 0.44, 0.19),
            Self::OakLeaves => (0.24, 0.48, 0.16),
            Self::Glowstone => (0.94, 0.69, 0.30),
            Self::Netherrack => (0.47, 0.20, 0.18),
            Self::Ice => (0.63, 0.83, 0.93),
            Self::Pumpkin => (0.91, 0.36, 0.09),
            Self::Prismarine => (0.27, 0.60, 0.54),
            Self::Terracotta => (0.65, 0.34, 0.24),
            Self::QuartzBlock => (0.88, 0.85, 0.79),
            Self::Water => (0.30, 0.68, 0.79),
        };
        Vec4::new(r, g, b, 1.0)
    }
}
