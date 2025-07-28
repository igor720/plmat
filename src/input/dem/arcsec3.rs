use std::fs;
use std::path::Path;

use crate::common::util::*;
use crate::common::types::*;
use crate::input::types::*;


const DEM_SIZE: Coord = 1200.0;
const DEM_EDGE_SIZE: usize = 1201;
const DEM_FILE_SIZE: u64 = 2884802;
const DEFAULT_NODATA: HeightInt = -32767;
const DEFAULT_SEA_LEVEL: HeightInt = 0;


#[derive(Debug)]
/// Arc3Sec dem options type
pub struct DemArc3SecOpts {
    nodata:     HeightInt,
    sea_level:  HeightInt,
}

impl DataSourceOpts for DemArc3SecOpts {
    /// Constructor of DemArc3SecOpts struct
    fn new_opts(nodata: Option<HeightInt>, sea_level: Option<HeightInt>)
        -> Self where Self:Sized {

        DemArc3SecOpts {
                nodata: nodata.unwrap_or(DEFAULT_NODATA),
                sea_level: sea_level.unwrap_or(DEFAULT_SEA_LEVEL),
            }
    }

    /// Get sea level
    fn get_sea_level(&self) -> HeightInt {
        self.sea_level
    }

    /// Get nodata value
    fn get_nodata(&self) -> HeightInt {
        self.nodata
    }

    /// Finds tileId for tile containing specified geopoint
    fn find_tile_id(&self, geo_point: &GeoPoint) -> TileID {
        let GeoPoint {lon, lat} = geo_point;
        TileID {
            lon: lon.floor() as CoordInt,
            lat: lat.floor() as CoordInt,
        }
    }

    /// Get maximum number of tiles
    fn get_max_number_of_tiles(&self) -> usize {
        180*360
    }
}

/// Arc3Sec dem data type
pub struct DemArc3SecData<'a> {
    lon_left:   CoordInt,
    lat_bottom: CoordInt,
    tile:       &'a dyn DataSourceOpts,
    dem_data:   Option<Box<[i16]>>,
}

impl<'a> TileData<'a> for DemArc3SecData<'a> {
    /// Get elevation at i row and j column in dem tile
    fn get_dem_height(&self, i: usize, j: usize) -> Option<i16> {
        self.dem_data.as_ref().map(|data| {data[j*DEM_EDGE_SIZE+i]})
    }

    /// Calculate elevation at a geopoint
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

            // XXX: rough; possible 3d models have much bigger cells then arc3sec dem elementary distances
            let h = match self.get_dem_height(i, 1200-j) {
                Some(h_int) =>
                    if h_int as HeightInt == self.tile.get_nodata() {self.tile.get_sea_level()} //XXX: nodata implies sea
                    else {h_int as HeightInt},
                None => self.tile.get_sea_level()  //XXX: missing tiles implies sea
            } as Height;

            return Some(h)
        }
    }

    /// Loads dem tile
    fn load<'b: 'a>(dir_path: &str, tile: &'b dyn DataSourceOpts, tile_id: &TileID)
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
                            tile,
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

    #[test]
    fn calc_height_t0() -> Result<(), String> {
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
            tile        : &dem_tile,
            dem_data    :Some(dem_data),
        };
        let p = GeoPoint {lat:50.5+0.5/DEM_SIZE, lon:50.5+0.5/DEM_SIZE};
        let height = dem.calc_height(&p).unwrap();
        if (height-100.0).abs() < 0.00001 {
            Ok(())
        } else {
            Err(format!("invalid height result: {}", height))
        }
    }
}



