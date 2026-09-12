//! Tile map rendering components.

use crate::prelude::*;

/// Used for mapping entities to a sized grid. This is used in the [`TileLayer`]
/// component for mapping entities with [`Tile`] components to it.
#[derive(Clone, Debug, HasSchema, Default)]
pub struct EntityGridMap {
    /// The vector of tile slots in this layer.
    pub tiles: Vec<Option<Entity>>,
    /// The size of the map in tiles (or columns and rows).
    pub grid_size: UVec2,
}

/// A tilemap layer component.
#[derive(Clone, Debug, HasSchema, Default, Deref, DerefMut)]
pub struct TileLayer {
    #[deref]
    /// The grid map for this [`TileLayer`]'s [`Tile`]s.
    pub map: EntityGridMap,
    /// The texture atlas to use for the layer
    pub atlas: Handle<Atlas>,
}
impl TileLayer {
    /// Create a new [`TileLayer`].
    pub fn new(grid_size: UVec2, atlas: Handle<Atlas>) -> Self {
        Self {
            map: EntityGridMap::new(grid_size),
            atlas,
        }
    }
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
}

impl EntityGridMap {
    /// Create a new tile layer
    pub fn new(grid_size: UVec2) -> Self {
        let mut out = Self {
            tiles: Vec::new(),
            grid_size,
        };
        out.ensure_space();
        out
    }

    /// Get's the entity at the given position in the grid if set, indexed with
    /// the bottom-left of the grid being (0, 0).
    ///
    /// Returns `None` is no `Entity` assigned to that position.
    ///
    /// Panics if the position is out of the grid bounds.
    pub fn get(&self, pos: UVec2) -> Option<Entity> {
        let idx = self.index(pos);
        self.tiles.get(idx as usize).cloned().flatten()
    }

    /// Get's the entity at the given position in the grid if set, indexed with
    /// the bottom-left of the grid being (0, 0).
    ///
    /// Returns `None` if the position is out of the grid bounds or there is no
    /// `Entity` assigned to that position.
    pub fn try_get(&self, pos: UVec2) -> Option<Entity> {
        self.try_index(pos).and_then(|idx| self.tiles[idx as usize])
    }

    /// Get the position on the grid with the given entity index.
    ///
    /// Panics if the index is out of the grid bounds.
    pub fn pos(&self, idx: u32) -> UVec2 {
        if idx as usize >= self.tiles.len() {
            panic!("index out of grid bounds")
        }
        let y = idx / self.grid_size.x;
        let x = idx - (y * self.grid_size.x);

        UVec2::new(x, y)
    }
    /// Try to get the position on the grid with the given entity index.
    ///
    /// Returns `None` if the index is out of the grid bounds.
    pub fn try_pos(&self, idx: u32) -> Option<UVec2> {
        if idx as usize >= self.tiles.len() {
            None
        } else {
            let y = idx / self.grid_size.x;
            let x = idx - (y * self.grid_size.x);

            UVec2::new(x, y).into()
        }
    }

    /// Get the index of the tile at the given position.
    #[inline]
    #[track_caller]
    pub fn index(&self, pos: UVec2) -> u32 {
        if pos.x + 1 > self.grid_size.x || pos.y + 1 > self.grid_size.y {
            panic!("grid postion out of bounds")
        }
        self.grid_size.x * pos.y + pos.x
    }
    /// Try to get the index of the tile at the given position.
    #[inline]
    pub fn try_index(&self, pos: UVec2) -> Option<u32> {
        if pos.x + 1 > self.grid_size.x || pos.y + 1 > self.grid_size.y {
            None
        } else {
            Some(self.grid_size.x * pos.y + pos.x)
        }
    }

    /// Set the tile at the given position, to a certain entity.
    pub fn set(&mut self, pos: UVec2, entity: Option<Entity>) {
        self.ensure_space();

        let idx = self.index(pos);
        self.tiles[idx as usize] = entity;
    }

    /// Makes sure the tiles vector has space for all of our tiles.
    fn ensure_space(&mut self) {
        let tile_count = (self.grid_size.x * self.grid_size.y) as usize;

        if unlikely(self.tiles.len() < tile_count) {
            self.tiles
                .extend((0..(tile_count - self.tiles.len())).map(|_| None));
        }
    }

    /// Set a tile position, resizing the map to accommodate the new tile if the map grid is not
    /// already large enough.
    pub fn set_with_resize(&mut self, pos: UVec2, entity: Option<Entity>) {
        // Calculate whatever size is larger, the grid size, or the size needed to store the tile
        // position.
        let required_grid_size = self.grid_size.max(pos + UVec2::ONE);

        // If the larger size is not the grid size we have currently
        if required_grid_size != self.grid_size {
            // Resize the grid to fit the position
            self.resize_grid(required_grid_size);
        }

        // Set the tile position
        self.set(pos, entity)
    }

    /// Copy all the tiles from another tileset into this one.
    ///
    /// This can be useful for resizing a map, for example.
    ///
    /// The tiles from the other layer are copied to the corresponding positions in this layer, if
    /// they fit into this layer's size.
    pub fn copy_from(&mut self, other: &EntityGridMap) {
        // Get the smallest size so that we know we won't go out of bounds while iterating over the
        // tiles.
        let visit_size = other.grid_size.min(self.grid_size);

        // Iterate over every position
        for x in 0..visit_size.x {
            for y in 0..visit_size.y {
                let pos = uvec2(x, y);
                // Set the position in this layer from the position in the other layer
                self.set(pos, other.get(pos))
            }
        }
    }

    /// Resize the layer, setting a new grid size.
    ///
    /// If shrinking the layer, tiles that are out-of-bounds will be removed.
    pub fn resize_grid(&mut self, grid_size: UVec2) {
        // Create a new layer with the desired grid size
        let mut new_layer = EntityGridMap::new(grid_size);

        // Copy the tiles from this layer into the new one
        new_layer.copy_from(self);

        // Swap out the new layer and the current one
        std::mem::swap(self, &mut new_layer);
    }
}

// TODO: add tests for EntityGridMap
