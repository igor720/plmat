use std::sync::atomic::{AtomicUsize, Ordering};
use std::fmt;

use crate::common::types::*;


// XXX: static only, not const
static TILE_COUNT:AtomicUsize = AtomicUsize::new(0);


pub type CoordInt = i16;

#[derive(Debug, PartialEq, Eq, Hash)]
/// TileID struct
pub struct TileID {
    pub lon: CoordInt,
    pub lat: CoordInt
}

impl TileID {
    fn next_integer() -> usize {
        TILE_COUNT.fetch_add(1, Ordering::SeqCst) as usize
    }

    /// Tile sequence generator
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TileId ({}, {})", self.lon, self.lat)
    }
}

pub trait DataSourceOpts {
    /// Constructor for Tile
    fn new_opts(nodata: Option<HeightInt>, sea_level: Option<HeightInt>) -> Self where Self:Sized;
    /// Get sea level
    fn get_sea_level(&self) -> HeightInt;
    /// Get nodata value
    fn get_nodata(&self) -> HeightInt;
    /// Get tileId for tile containing specified geopoint
    fn find_tile_id(&self, geo_point: &GeoPoint) -> TileID;
    /// Get maximum number of tiles
    fn get_max_number_of_tiles(&self) -> usize;
}

pub trait TileData<'a> {
    /// Get elevation at i row and j column in dem tile
    fn get_dem_height(&self, i: usize, j: usize) -> Option<i16>;
    /// Calculate elevation at geographic cooedinates
    fn calc_height(&self, geo_point: &GeoPoint) -> Option<Height>;
    /// Loads dem tile
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

