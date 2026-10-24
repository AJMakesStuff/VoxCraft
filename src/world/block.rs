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
}

impl Block {
    pub const SOLID_PICKER: [Self; 13] = [
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
    ];

    pub const fn is_solid(self) -> bool {
        !matches!(self, Self::Air)
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
            Self::Cobblestone => "Cobblestone",
            Self::Bricks => "Bricks",
            Self::Snow => "Snow",
            Self::Gravel => "Gravel",
            Self::Obsidian => "Obsidian",
            Self::GoldBlock => "Gold Block",
        }
    }

    pub const fn texture_index(self) -> usize {
        match self {
            Self::Air | Self::Grass => 0,
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
        };
        Vec4::new(r, g, b, 1.0)
    }
}
