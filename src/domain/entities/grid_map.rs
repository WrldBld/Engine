//! Grid map for tactical combat

use crate::domain::value_objects::{GridMapId, WorldId};

/// A tactical grid map for combat
#[derive(Debug, Clone)]
pub struct GridMap {
    pub id: GridMapId,
    pub world_id: WorldId,
    pub name: String,
    pub width: u32,
    pub height: u32,
    /// Path to the tilesheet asset
    pub tilesheet_asset: String,
    /// Tile size in pixels (for rendering)
    pub tile_size: u32,
    /// The grid of tiles
    pub tiles: Vec<Vec<Tile>>,
}

impl GridMap {
    pub fn new(
        world_id: WorldId,
        name: impl Into<String>,
        width: u32,
        height: u32,
        tilesheet_asset: impl Into<String>,
    ) -> Self {
        let tiles = (0..height)
            .map(|_| (0..width).map(|_| Tile::default()).collect())
            .collect();

        Self {
            id: GridMapId::new(),
            world_id,
            name: name.into(),
            width,
            height,
            tilesheet_asset: tilesheet_asset.into(),
            tile_size: 32,
            tiles,
        }
    }

    pub fn get_tile(&self, x: u32, y: u32) -> Option<&Tile> {
        self.tiles.get(y as usize)?.get(x as usize)
    }

    pub fn get_tile_mut(&mut self, x: u32, y: u32) -> Option<&mut Tile> {
        self.tiles.get_mut(y as usize)?.get_mut(x as usize)
    }

    pub fn set_tile(&mut self, x: u32, y: u32, tile: Tile) {
        if let Some(row) = self.tiles.get_mut(y as usize) {
            if let Some(cell) = row.get_mut(x as usize) {
                *cell = tile;
            }
        }
    }

    /// Calculate movement cost between adjacent tiles considering elevation
    pub fn movement_cost(&self, from: (u32, u32), to: (u32, u32)) -> Option<u32> {
        let from_tile = self.get_tile(from.0, from.1)?;
        let to_tile = self.get_tile(to.0, to.1)?;

        if !to_tile.passable {
            return None;
        }

        let elevation_diff = (to_tile.elevation - from_tile.elevation).abs();
        let base_cost = to_tile.terrain_type.movement_cost();

        // Climbing costs extra
        Some(base_cost + elevation_diff as u32)
    }
}

/// A single tile on the grid map
#[derive(Debug, Clone)]
pub struct Tile {
    pub terrain_type: TerrainType,
    /// Elevation level (supports height differences)
    pub elevation: i32,
    /// Index into the tilesheet
    pub tile_index: u32,
    /// Whether units can move through this tile
    pub passable: bool,
    /// Cover value for combat (0 = none, 1 = light, 2 = heavy)
    pub cover_value: u8,
}

impl Default for Tile {
    fn default() -> Self {
        Self {
            terrain_type: TerrainType::Ground,
            elevation: 0,
            tile_index: 0,
            passable: true,
            cover_value: 0,
        }
    }
}

impl Tile {
    pub fn new(terrain_type: TerrainType, tile_index: u32) -> Self {
        Self {
            terrain_type,
            elevation: 0,
            tile_index,
            passable: terrain_type.default_passable(),
            cover_value: terrain_type.default_cover(),
        }
    }

    pub fn with_elevation(mut self, elevation: i32) -> Self {
        self.elevation = elevation;
        self
    }

    pub fn blocking(mut self) -> Self {
        self.passable = false;
        self
    }

    pub fn with_cover(mut self, cover: u8) -> Self {
        self.cover_value = cover;
        self
    }
}

/// Types of terrain
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TerrainType {
    #[default]
    Ground,
    Water,
    Wall,
    Difficult,
    Hazard,
    Pit,
}

impl TerrainType {
    pub fn movement_cost(&self) -> u32 {
        match self {
            Self::Ground => 1,
            Self::Water => 2,
            Self::Wall => u32::MAX,
            Self::Difficult => 2,
            Self::Hazard => 1,
            Self::Pit => u32::MAX,
        }
    }

    pub fn default_passable(&self) -> bool {
        !matches!(self, Self::Wall | Self::Pit)
    }

    pub fn default_cover(&self) -> u8 {
        match self {
            Self::Wall => 2,
            _ => 0,
        }
    }
}
