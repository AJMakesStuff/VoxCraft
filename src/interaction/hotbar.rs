use bevy::prelude::Resource;

use crate::world::Block;

pub const HOTBAR_SLOTS: usize = 5;

#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct Hotbar {
    pub slots: [Block; HOTBAR_SLOTS],
    pub selected: usize,
}

impl Default for Hotbar {
    fn default() -> Self {
        Self {
            slots: [
                Block::Grass,
                Block::Dirt,
                Block::Stone,
                Block::Sand,
                Block::Bedrock,
            ],
            selected: 0,
        }
    }
}

impl Hotbar {
    pub fn selected_block(&self) -> Block {
        self.slots[self.selected]
    }

    pub fn select_slot(&mut self, slot: usize) {
        if slot < HOTBAR_SLOTS {
            self.selected = slot;
        }
    }

    pub fn replace_selected(&mut self, block: Block) {
        self.slots[self.selected] = block;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picker_replaces_the_active_slot_without_changing_other_slots() {
        let mut hotbar = Hotbar::default();
        hotbar.select_slot(2);
        hotbar.replace_selected(Block::Sand);

        assert_eq!(hotbar.selected_block(), Block::Sand);
        assert_eq!(hotbar.slots[0], Block::Grass);
        assert_eq!(hotbar.slots[2], Block::Sand);
    }
}
