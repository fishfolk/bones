//! Tile map rendering components.

use crate::prelude::*;

/// A tilemap layer component.
#[derive(Clone, Debug, HasSchema, Default)]
pub struct TileLayer {
    /// The vector of tile slots in this layer.
    pub tiles: Vec<Option<Entity>>,
    /// The size of the layer in tiles.
    pub grid_size: UVec2,
    /// The size of each tile in the layer.
    pub tile_size: Vec2,
    /// The texture atlas to use for the layer
    pub atlas: Handle<Atlas>,
}

/// A tilemap tile component.
#[derive(Clone, Debug, HasSchema, Default)]
#[repr(C)]
pub struct Tile {
    /// The tile index in the tilemap texture.
    pub idx: u32,
    /// Whether or not to flip the tile horizontally.
    pub flip_x: bool,
    /// Whether or not to flip tile vertically.
    pub flip_y: bool,
    /// The tile's color tint
    pub color: Color,
}

impl TileLayer {
    /// Create a new tile layer
    pub fn new(grid_size: UVec2, tile_size: Vec2, atlas: Handle<Atlas>) -> Self {
        let mut out = Self {
            tiles: Vec::new(),
            grid_size,
            tile_size,
            atlas,
        };
        out.ensure_space();
        out
    }

    /// Makes sure the tiles vector has space for all of our tiles.
    fn ensure_space(&mut self) {
        let tile_count = (self.grid_size.x * self.grid_size.y) as usize;

        if unlikely(self.tiles.len() < tile_count) {
            self.tiles
                .extend((0..(tile_count - self.tiles.len())).map(|_| None));
        }
    }

    /// Get the index of the tile at the given position.
    #[inline]
    pub fn idx(&self, pos: UVec2) -> u32 {
        self.grid_size.x * pos.y + pos.x
    }

    /// Get the position of the tile at the given index.
    pub fn pos(&self, idx: u32) -> UVec2 {
        let y = idx / self.grid_size.x;
        let x = idx - (y * self.grid_size.x);

        UVec2::new(x, y)
    }

    /// Get's the tile at the given position in the layer, indexed with the bottom-left of the layer
    /// being (0, 0).
    pub fn get(&self, pos: UVec2) -> Option<Entity> {
        let idx = self.idx(pos);
        self.tiles.get(idx as usize).cloned().flatten()
    }

    /// Set the tile at the given position, to a certain entity.
    pub fn set(&mut self, pos: UVec2, entity: Option<Entity>) {
        self.ensure_space();

        let idx = self.idx(pos);
        *self.tiles.get_mut(idx as usize).unwrap_or_else(|| {
            panic!(
                "Tile pos out of range of tile size: pos {:?} size {:?}",
                pos, self.grid_size
            )
        }) = entity;
    }
}
