//! Tile map rendering components.

use crate::prelude::*;

/// A tilemap layer component.
#[derive(Clone, Debug, TypeUlid)]
#[ulid = "01GNF7SRDRN4K8HPW32JAHKMX1"]
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
#[derive(Clone, Debug, TypeUlid, Default)]
#[ulid = "01GNZHDZV61TFPEE4GDJY4SRAM"]
pub struct Tile {
    /// The tile index in the tilemap texture.
    pub idx: usize,
    /// Whether or not to flip the tile horizontally.
    pub flip_x: bool,
    /// Whether or not to flip tile vertically.
    pub flip_y: bool,
}

impl TileLayer {
    /// Create a new tile layer
    pub fn new(grid_size: UVec2, tile_size: Vec2, atlas: Handle<Atlas>) -> Self {
        let tile_count = (grid_size.x * grid_size.y) as usize;
        let mut tiles = Vec::with_capacity(tile_count);
        for _ in 0..tile_count {
            tiles.push(None);
        }
        Self {
            tiles,
            grid_size,
            tile_size,
            atlas,
        }
    }

    #[inline]
    fn idx(&self, pos: UVec2) -> Option<usize> {
        let idx = self.grid_size.x as i32 * pos.y as i32 + pos.x as i32;
        idx.try_into().ok()
    }

    /// Get's the tile at the given position in the layer, indexed with the bottom-left of the layer
    /// being (0, 0).
    pub fn get(&self, pos: UVec2) -> Option<Entity> {
        self.idx(pos)
            .and_then(|idx| self.tiles.get(idx).cloned().flatten())
    }

    /// Set the tile at the given position, to a certain entity.
    pub fn set(&mut self, pos: UVec2, entity: Option<Entity>) {
        let idx = self.idx(pos).expect("Tile pos out of bounds");
        *self.tiles.get_mut(idx).unwrap_or_else(|| {
            panic!(
                "Tile pos out of range of tile size: pos {:?} size {:?}",
                pos, self.grid_size
            )
        }) = entity;
    }
}
