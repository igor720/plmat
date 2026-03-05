use std::sync::atomic::{AtomicUsize, Ordering};
use std::fmt;

use crate::common::types::*;


// Static only, not const
static TILE_COUNT:AtomicUsize = AtomicUsize::new(0);

pub type CoordInt = i16;

#[derive(Debug, PartialEq, Eq, Hash)]
/// TileID struct
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
    /// integers to geographic coordinates in a grid pattern.
    pub fn next(limit: usize) -> Option<TileID> {
        let count = TileID::next_integer();
        return if count>=limit {
            None
        } else {
            Some(TileID {
                lon: (count/180) as CoordInt - 180,
                lat: (count%180) as CoordInt - 90
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
pub trait DataSourceOpts {
    /// Creates a new data source options instance with specified nodata and sea level values
    /// 
    /// # Arguments
    /// * `nodata` - Optional nodata value to indicate missing data
    /// * `sea_level` - Optional sea level value for elevation calculations
    /// 
    /// # Returns
    /// * `Self` - A new instance of the implementing type
    fn new_opts(nodata: Option<HeightInt>, sea_level: Option<HeightInt>) -> Self where Self:Sized;
    
    /// Gets the sea level value used for elevation calculations
    fn get_sea_level(&self) -> HeightInt;
    
    /// Gets the nodata value used to indicate missing or invalid data
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
pub trait TileData<'a> {
    /// Gets elevation height at a specific row and column in the DEM tile
    /// 
    /// # Arguments
    /// * `i` - Row index in the tile
    /// * `j` - Column index in the tile
    /// 
    /// # Returns
    /// * `Option<i16>` - Elevation height at the specified location or None if invalid
    fn get_dem_height(&self, i: usize, j: usize) -> Option<i16>;
    
    /// Calculates elevation at specific geographic coordinates
    fn calc_height(&self, geo_point: &GeoPoint) -> Option<Height>;
    
    /// Loads a DEM tile from the specified directory with given options
    /// 
    /// # Arguments
    /// * `dir_path` - Path to the directory containing tile data
    /// * `opts` - Data source options for configuration
    /// * `tile_id` - Tile ID to load
    /// 
    /// # Returns
    /// * `Result<Option<Self>, String>` - Loaded tile or error message
    fn load<'b: 'a>(dir_path: &str, opts: &'b dyn DataSourceOpts, tile_id: &TileID)
        -> Result<Option<Self>, String> where Self:Sized;
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
