//! Geographic data types and utilities for elevation mapping
//! 
//! This module provides core data structures and traits for working with
//! geographic elevation data, particularly digital elevation models (DEMs).
//! 
//! The module defines:
//! 
//! - `TileID`: Geographic coordinates for identifying DEM tiles
//! - `DataSourceOpts`: Interface for configuring data source options
//! - `TileData`: Interface for accessing elevation data from tiles
//! 
//! # Tile Coordinate System
//! 
//! Tiles are organized in a geographic grid system where:
//! 
//! - Longitude (`lon`) ranges from -180 to 179 degrees
//! - Latitude (`lat`) ranges from -90 to 89 degrees
//! - Each tile represents a 1-degree by 1-degree geographic region
//! 
//! # Data Source Configuration
//! 
//! The `DataSourceOpts` trait allows for flexible configuration of data sources
//! including:
//! 
//! - Nodata values for indicating missing data
//! - Sea level for elevation calculations
//! - Tile limit specifications
//! 
//! # Tile Data Operations
//! 
//! The `TileData` trait provides methods for:
//! 
//! - Accessing elevation data at specific grid positions
//! - Calculating elevation at arbitrary geographic coordinates
//! - Loading tiles from disk
use std::sync::atomic::{AtomicUsize, Ordering};
use std::fmt;
use std::path::Path;
use crate::common::types::*;

// Static only, not const
static TILE_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Integer type used for geographic coordinates
/// 
/// This type is used for representing longitude and latitude coordinates
/// in the tile grid system. It's defined as i16 to accommodate the range
/// of geographic coordinates (-180 to 179 for longitude, -90 to 89 for latitude).
pub type CoordInt = i16;

#[derive(Debug, PartialEq, Eq, Hash)]
/// TileID struct
/// 
/// Represents a geographic tile in the DEM grid system. Each tile corresponds
/// to a 1-degree by 1-degree region on the planet's surface.
pub struct TileID {
    /// Longitude coordinate of the tile
    pub lon: CoordInt,
    /// Latitude coordinate of the tile
    pub lat: CoordInt
}

impl TileID {
    /// Gets the next sequential integer for tile generation
    /// 
    /// This is an internal helper function that maintains a global counter
    /// for generating unique tile IDs.
    fn next_integer() -> usize {
        TILE_COUNT.fetch_add(1, Ordering::SeqCst) as usize
    }

    /// Generates the next TileID in sequence up to a specified limit
    /// 
    /// This function creates tile IDs in a systematic way, mapping sequential
    /// integers to geographic coordinates in a grid pattern. The tiles are
    /// generated in row-major order, starting from (-180, -90) and proceeding
    /// across longitude and then latitude.
    pub fn next(limit: usize) -> Option<TileID> {
        let count = TileID::next_integer();
        return if count >= limit {
            None
        } else {
            Some(TileID {
                lon: (count / 180) as CoordInt - 180,
                lat: (count % 180) as CoordInt - 90
            })
        }
    }
}

impl fmt::Display for TileID {
    /// Formats the TileID for display purposes
    /// 
    /// This implementation provides a human-readable string representation
    /// of the TileID showing its longitude and latitude coordinates.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TileId ({}, {})", self.lon, self.lat)
    }
}

/// Trait defining the interface for data source options
/// 
/// This trait provides methods for configuring and accessing properties
/// of data sources used in geographic data processing, particularly for
/// digital elevation models.
/// 
/// Implementers of this trait should provide configuration options for
/// handling nodata values, sea level, and tile identification.
pub trait DataSourceOpts {
    /// Creates a new data source options instance with specified nodata and sea level values
    fn new_opts(nodata: Option<HeightInt>, sea_level: Option<HeightInt>) -> Self where Self:Sized;
    
    /// Gets the sea level value used for elevation calculations
    /// 
    /// This value is used as the elevation for areas that are below sea level
    /// or where elevation data is unavailable.
    fn get_sea_level(&self) -> HeightInt;
    
    /// Gets the nodata value used to indicate missing or invalid data
    /// 
    /// This value is used to identify areas in the elevation data where no valid
    /// elevation measurement exists.
    fn get_nodata(&self) -> HeightInt;
    
    /// Finds the tile ID that contains a specified geographic point
    fn find_tile_id(&self, geo_point: &GeoPoint) -> TileID;
    
    /// Gets the maximum number of tiles that can be processed
    fn get_max_number_of_tiles(&self) -> usize;
}

/// Trait defining the interface for tile data operations
/// 
/// This trait provides methods for accessing elevation data from tiles
/// and performing geographic calculations on that data.
/// 
/// Implementers of this trait should provide functionality for:
/// - Accessing elevation data at specific grid positions
/// - Calculating elevation at arbitrary geographic coordinates
/// - Loading tiles from disk
pub trait TileData<'a> {
    /// Gets elevation height at a specific row and column in the DEM tile
    /// 
    /// This method retrieves the elevation value at a specific grid position
    /// within the tile. The grid positions are typically indexed from 0.
    fn get_dem_height(&self, i: usize, j: usize) -> Option<HeightInt>;
    
    /// Calculates elevation at specific geographic coordinates
    /// 
    /// This method interpolates elevation values to determine the elevation
    /// at an arbitrary geographic point within the tile.
    fn calc_height(&self, geo_point: &GeoPoint) -> Option<Height>;
    
    /// Loads a DEM tile from the specified directory with given options
    /// 
    /// This static method loads a tile from disk using the provided directory
    /// path and data source options.
    fn load<'b: 'a>(dir_path: &Path, opts: &'b dyn DataSourceOpts, tile_id: &TileID)
        -> Result<Option<Self>, String> where Self: Sized;
}


#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches as am;

    #[test]
    fn next_t0() {
        am::assert_matches!(TileID::next(180*360), Some(TileID{lon:-180, lat:-90}));
        am::assert_matches!(TileID::next(180*360), Some(TileID{lon:-180, lat:-89}));
        am::assert_matches!(TileID::next(180*360), Some(TileID{lon:-180, lat:-88}));
    }
}
