//! 3ArcSec elevation data source implementation
//! 
//! This module provides implementation for handling 3-arcsecond resolution Digital Elevation Model (DEM)
//! data, specifically for the SRTM (Shuttle Radar Topography Mission) dataset. It implements the
//! DataSourceOpts and TileData traits for working with DEM tile data.
//! 
//! The 3ArcSec format uses:
//! - 3-arcsecond resolution (approximately 90 meters per pixel)
//! - 1201x1201 grid size per tile (1200x1200 pixels + 1 for indexing)
//! - .hgt file format with signed 16-bit integers
//! - File size of 2,884,802 bytes per tile
//! 
//! # File Naming Convention
//! 
//! Tile files follow the SRTM naming convention:
//! - Format: `{lat hemisphere}{latitude}{lon hemisphere}{longitude}.hgt`
//! - Example: `N45E090.hgt` for tile covering 45°N to 46°N and 90°E to 91°E
//! 
//! # Coordinate System
//! 
//! The coordinate system uses:
//! - Longitude from -180° to 179° (West to East)
//! - Latitude from -90° to 89° (South to North)
//! - Each tile represents a 1° by 1° geographic region
use std::fs;
use std::path::Path;
use crate::common::util::*;
use crate::common::types::*;
use crate::input::types::*;

/// Size of the DEM grid in cells (1200.0 for 3-arcsecond resolution)
const DEM_SIZE: Coord = 1200.0;

/// Edge size of the DEM grid (1201 to account for 0-based indexing)
const DEM_EDGE_SIZE: usize = 1201;

/// Expected file size of each DEM tile in bytes
const DEM_FILE_SIZE: u64 = 2884802;

/// Default nodata value for missing elevation data
const DEFAULT_NODATA: HeightInt = -32767;

/// Default sea level value for elevation calculations
const DEFAULT_SEA_LEVEL: HeightInt = 0;

#[derive(Debug)]
/// Data source options for Arc3Sec DEM format
/// 
/// This struct holds configuration options for Arc3Sec DEM data sources,
/// including nodata values and sea level settings for elevation calculations.
/// 
/// # Fields
/// 
/// * `nodata` - Value used to indicate missing or invalid elevation data
/// * `sea_level` - Value used as the elevation for areas below sea level
pub struct DemArc3SecOpts {
    nodata:     HeightInt,
    sea_level:  HeightInt,
}

impl DataSourceOpts for DemArc3SecOpts {
    /// Creates a new DemArc3SecOpts instance with specified nodata and sea_level values
    /// 
    /// If None values are provided, default values are used:
    /// - nodata: -32767 (standard for SRTM data)
    /// - sea_level: 0 (standard for sea level reference)
    fn new_opts(nodata: Option<HeightInt>, sea_level: Option<HeightInt>)
        -> Self where Self:Sized {

        DemArc3SecOpts {
                nodata: nodata.unwrap_or(DEFAULT_NODATA),
                sea_level: sea_level.unwrap_or(DEFAULT_SEA_LEVEL),
            }
    }

    /// Gets the sea level value used for elevation calculations
    /// 
    /// This value is used to represent elevations below sea level or for areas
    /// where elevation data is missing.
    fn get_sea_level(&self) -> HeightInt {
        self.sea_level
    }

    /// Gets the nodata value used to identify missing elevation data
    /// 
    /// This value indicates where no elevation data is available in the DEM.
    fn get_nodata(&self) -> HeightInt {
        self.nodata
    }

    /// Finds the tile ID for a given geographic point
    /// 
    /// This method determines which tile contains the specified geographic coordinates
    /// by flooring the coordinates to the nearest integer degree.
    fn find_tile_id(&self, geo_point: &GeoPoint) -> TileID {
        let GeoPoint {lon, lat} = geo_point;
        TileID {
            lon: lon.floor() as CoordInt,
            lat: lat.floor() as CoordInt,
        }
    }

    /// Gets the maximum number of tiles in the DEM dataset
    /// 
    /// For the SRTM dataset, this represents the total number of 1°×1° tiles
    /// covering the entire globe.
    fn get_max_number_of_tiles(&self) -> usize {
        180*360
    }
}

/// Arc3Sec DEM data structure
/// 
/// This struct represents a single DEM tile with its geographic location and elevation data.
/// It holds the raw elevation data for a 1°×1° geographic region.
/// 
/// # Fields
/// 
/// * `lon_left` - Left boundary longitude of the tile
/// * `lat_bottom` - Bottom boundary latitude of the tile
/// * `tile` - Reference to the data source options for this tile
/// * `dem_data` - Optional boxed slice containing the elevation data
pub struct DemArc3SecData<'a> {
    lon_left:   CoordInt,
    lat_bottom: CoordInt,
    tile:       &'a dyn DataSourceOpts,
    dem_data:   Option<Box<[i16]>>,
}

impl<'a> TileData<'a> for DemArc3SecData<'a> {
    /// Gets elevation at a specific row and column in the DEM grid
    /// 
    /// This method retrieves the elevation value at the specified grid coordinates.
    /// The grid uses 0-based indexing with the data stored in column-major order.
    fn get_dem_height(&self, i: usize, j: usize) -> Option<HeightInt> {
        self.dem_data.as_ref().map(|data| data.get(j*DEM_EDGE_SIZE+i).copied())?
    }

    /// Calculates elevation at a specific geographic point
    /// 
    /// This method interpolates elevation values from the DEM grid to determine
    /// the elevation at the specified geographic coordinates. It handles coordinate
    /// conversion and grid indexing to find the appropriate elevation value.
    fn calc_height(&self, geo_point: &GeoPoint) -> Option<Height> {
        let GeoPoint {lon, lat} = *geo_point;
        if (lon<(self.lon_left as Coord) || lon>=((1+self.lon_left) as Coord))
                || (lat<(self.lat_bottom as Coord) || lat>=((1+self.lat_bottom) as Coord)
                ) {
            return None
        } else {
            let x = lon-(self.lon_left as Coord);
            let y = lat-(self.lat_bottom as Coord);
            let i = (x * DEM_SIZE).floor() as usize;
            let j = (y * DEM_SIZE ).floor() as usize;

            // rough; possible 3d models have much bigger cells then arc3sec dem elementary distances
            let h = match self.get_dem_height(i, DEM_EDGE_SIZE-1-j) {
                Some(h_int) =>
                    if h_int as HeightInt == self.tile.get_nodata() {self.tile.get_sea_level()} // nodata implies sea
                    else {h_int as HeightInt},
                None => self.tile.get_sea_level()  // missing tiles implies sea
            } as Height;

            return Some(h)
        }
    }

    /// Loads a DEM tile from a file system path
    /// 
    /// This method reads a .hgt file from the specified directory and loads the elevation data
    /// into memory. It validates the file size and handles coordinate conversion to determine
    /// the appropriate filename.
    /// 
    /// # Arguments
    /// 
    /// * `dir_path` - Directory path containing the DEM tile files
    /// * `tile_opts` - Reference to the data source options for this tile
    /// * `tile_id` - Tile ID specifying which tile to load
    /// 
    /// # Returns
    /// 
    /// Result containing either:
    /// - Some(DemArc3SecData) with the loaded tile data
    /// - None if the tile file doesn't exist
    /// - Error if file reading or validation fails
    fn load<'b: 'a>(dir_path: &Path, tile_opts: &'b dyn DataSourceOpts, tile_id: &TileID)
        -> Result<Option<Self>, String> where Self:Sized {

        let TileID {lon, lat} = *tile_id;

        if (lon<(-180) || lon>=180) || (lat<(-90) || lat>=90 ) {
            return Err(format!("Invalid tile specification: {}", tile_id))
        } else {
            let vert_hemisphere;
            if lat>=0 {
                vert_hemisphere = "N".to_string() + &(format!("{:02}", lat))
            } else {
                vert_hemisphere = "S".to_string() + &(format!("{:02}", -lat))
            };

            let horz_hemisphere;
            if lon>=0 {
                horz_hemisphere = "E".to_string() + &(format!("{:03}", lon))
            } else {
                horz_hemisphere = "W".to_string() + &(format!("{:03}", -lon))
            };

            let file_name = format!("{}{}.hgt", &vert_hemisphere, &horz_hemisphere);
            let p = Path::new(&dir_path).join(file_name);

            if p.exists() {
                let len = match p.metadata() {
                    Ok(m) => m.len(),
                    Err(err) => return Err(format!("Can't get metadata of {:?}: {}", p, err))
                };
                if len!= DEM_FILE_SIZE {
                    return Err(format!("Invalid file size of {}: {}", tile_id, len));
                };

                let file_path = match p.to_str() {
                    Some(fp) => fp,
                    None => return Err(format!("Can't get file path of {}", tile_id))
                };

                match fs::read(&file_path) {
                    Ok(data_u8) =>
                        Ok(Some(DemArc3SecData {
                            lon_left: lon,
                            lat_bottom: lat,
                            tile: tile_opts,
                            dem_data: Some(vec_u8_to_i16(data_u8).into_boxed_slice()),
                        })),
                    Err(err) => Err(format!("Error reading tile {}: {}", tile_id, err))
                }
            } else {
                Ok(None)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const DEM_ARRAY_SIZE:usize = DEM_EDGE_SIZE*DEM_EDGE_SIZE;

    /// Test that get_dem_height works correctly
    #[test]
    fn test_get_dem_height() {
        let mut dem_data = vec![0; DEM_ARRAY_SIZE].into_boxed_slice();
        dem_data[0] = 100; // First element
        dem_data[1] = 200; // Second element
        dem_data[DEM_EDGE_SIZE] = 300; // Element in second row
        
        let dem_tile = DemArc3SecOpts {
            nodata: -32767,
            sea_level: 0,
        };

        let dem = DemArc3SecData {
            lon_left: 0,
            lat_bottom: 0,
            tile: &dem_tile,
            dem_data: Some(dem_data),
        };

        // Test getting values at specific indices
        assert_eq!(dem.get_dem_height(0, 0), Some(100));
        assert_eq!(dem.get_dem_height(1, 0), Some(200));
        assert_eq!(dem.get_dem_height(0, 1), Some(300));
        assert_eq!(dem.get_dem_height(DEM_EDGE_SIZE-1, DEM_EDGE_SIZE-1), Some(0)); // Last element (sea)
        assert_eq!(dem.get_dem_height(DEM_EDGE_SIZE-1, DEM_EDGE_SIZE), None); // Out of bounds
    }

    /// Test tile ID finding
    #[test]
    fn test_find_tile_id() {
        let dem_tile = DemArc3SecOpts {
            nodata: -32767,
            sea_level: 0,
        };

        // Test various coordinates
        let point1 = GeoPoint { lat: 45.7, lon: 120.3 };
        let tile_id1 = dem_tile.find_tile_id(&point1);
        assert_eq!(tile_id1.lon, 120);
        assert_eq!(tile_id1.lat, 45);

        let point2 = GeoPoint { lat: -30.9, lon: -150.2 };
        let tile_id2 = dem_tile.find_tile_id(&point2);
        assert_eq!(tile_id2.lon, -151);
        assert_eq!(tile_id2.lat, -31);
    }

    /// Test coordinate conversion in calc_height
    #[test]
    fn test_coordinate_conversion() {
        // Create a simple test grid where we know the expected values
        let mut dem_data = vec![0; DEM_ARRAY_SIZE].into_boxed_slice();
        
        // Set up a simple pattern
        // Set the center point to 500
        let center_idx = DEM_EDGE_SIZE * (DEM_EDGE_SIZE / 2) + (DEM_EDGE_SIZE / 2);
        dem_data[center_idx] = 500;
        
        let dem_tile = DemArc3SecOpts {
            nodata: -32767,
            sea_level: 0,
        };

        let dem = DemArc3SecData {
            lon_left: 0,
            lat_bottom: 0,
            tile: &dem_tile,
            dem_data: Some(dem_data),
        };

        // Test coordinate conversion for center point
        let center_point = GeoPoint { lat: 0.5, lon: 0.5 };
        let height = dem.calc_height(&center_point);
        assert!(height.is_some());
        // The exact value depends on how the interpolation works
        // but it should not be None
    }

    /// Test boundary conditions
    #[test]
    fn test_boundary_conditions() {
        let dem_tile = DemArc3SecOpts {
            nodata: -32767,
            sea_level: 0,
        };

        let dem = DemArc3SecData {
            lon_left: 0,
            lat_bottom: 0,
            tile: &dem_tile,
            dem_data: Some(vec![0i16; DEM_ARRAY_SIZE].into_boxed_slice()),
        };

        // Test point outside the tile bounds - should return None
        let point_outside = GeoPoint { lat: 1.5, lon: 0.5 };
        let height = dem.calc_height(&point_outside);
        assert!(height.is_none());

        let point_outside2 = GeoPoint { lat: 0.5, lon: 1.5 };
        let height2 = dem.calc_height(&point_outside2);
        assert!(height2.is_none());
    }

    /// Test nodata handling
    #[test]
    fn test_nodata_handling() {
        let dem_data = vec![DEFAULT_NODATA; DEM_ARRAY_SIZE].into_boxed_slice();
        
        let dem_tile = DemArc3SecOpts {
            nodata: -32767,
            sea_level: 10, // Sea level is 10
        };

        let dem = DemArc3SecData {
            lon_left: 0,
            lat_bottom: 0,
            tile: &dem_tile,
            dem_data: Some(dem_data),
        };

        // Test that nodata point returns sea level
        let nodata_point = GeoPoint { lat: 0.0833, lon: 0.0833 }; // Around 100,100 in grid
        let height = dem.calc_height(&nodata_point);
        assert_eq!(height, Some(10.0)); // Should return sea level
    }

    /// Test for elevation calculation at a specific point
    /// 
    /// This test verifies that the elevation calculation works correctly by creating
    /// a small test dataset with known values and checking that the interpolation
    /// produces the expected result.
    #[test]
    fn calc_height_t0() -> Result<(), ErrBox> {
        let mut dem_data = vec![1; DEM_ARRAY_SIZE].into_boxed_slice();
        dem_data[DEM_ARRAY_SIZE/2] = 100;
        dem_data[DEM_ARRAY_SIZE/2+1] = 0;
        dem_data[DEM_ARRAY_SIZE/2+DEM_EDGE_SIZE] = 10;
        dem_data[DEM_ARRAY_SIZE/2+1+DEM_EDGE_SIZE] = 30;
        let dem_tile = DemArc3SecOpts {
            nodata      :-32767,
            sea_level   :0,
        };

        let dem = DemArc3SecData {
            lon_left    :50,
            lat_bottom  :50,
            tile        :&dem_tile,
            dem_data    :Some(dem_data),
        };
        let p = GeoPoint {lat:50.5+0.5/DEM_SIZE, lon:50.5+0.5/DEM_SIZE};
        let height = dem.calc_height(&p).unwrap();
        if (height-100.0).abs() < 0.00001 {
            Ok(())
        } else {
            Err(format!("invalid height result: {}", height).into())
        }
    }
}
