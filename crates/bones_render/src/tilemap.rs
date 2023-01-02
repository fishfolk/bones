//! Tile map rendering components.

use crate::prelude::*;

/// A tilemap layer.
#[derive(Clone, Debug, TypeUlid)]
#[ulid = "01GNF7SRDRN4K8HPW32JAHKMX1"]
pub struct TileLayer {
    /// The vector of tile slots in this layer.
    pub tiles: Vec<Option<Entity>>,
    /// The size of the layer in tiles.
    pub grid_size: UVec2,
    /// The size of each tile in the layer.
    pub tile_size: Vec2,
}

impl TileLayer {
    /// Get's the tile at the given position in the layer, indexed with the top-left of the layer
    /// being (0, 0).
    pub fn get(&self, pos: UVec2) -> Option<Entity> {
        let idx = self.grid_size.x * pos.y + pos.x;
        self.tiles.get(idx as usize).cloned().flatten()
    }
}
