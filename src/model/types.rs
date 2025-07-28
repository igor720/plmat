use std::collections::HashMap;
use std::collections::BTreeMap;
use std::thread;
use std::sync::{Mutex};
use std::ops::DerefMut;

use crate::common::args::*;
use crate::common::settings::*;
use crate::common::types::*;
use crate::common::color::*;
use crate::input::types::*;
use crate::input::dem::*;


const DEFAULT_COLOR_PROFILE_FILE: &str = "./color_profile";


#[derive(Debug)]
pub struct ModelPoints {
    pub geopoints: GeoPoints,
    pub points_map_opt: Option<PointsMapping>,
}

/// Geopoints data
pub type GeoPoints = BTreeMap<GeoPointIndex, GeoPoint>;

/// Texture points to geopoints mapping
pub type PointsMapping = HashMap<GeoPointIndex, GeoPointIndex>;

/// Geopoints to tiles mapping data
pub type GeoPointsToTilesMapping<'a> = HashMap<TileID, Vec<(GeoPointIndex, &'a GeoPoint)>>;

/// Model elements data
pub type Elements = Vec<(GeoPointIndex, GeoPointIndex, GeoPointIndex)>;

/// Elevations data
pub type Heights = BTreeMap<GeoPointIndex, Height>;

/// Colors data
pub type Colors = BTreeMap<GeoPointIndex, RGB>;

/// Texture coordinates data
pub type TextureCoordinates = Vec<(TextureCoordinate, TextureCoordinate)>;

#[derive(Debug)]
/// Model type specific data
pub enum ModelTypeData {Color(Colors), Texture(TextureCoordinates)}


/// Makes specific data source Opts struct
pub fn make_data_source_opts(nodata: Option<HeightInt>, sea_level: Option<HeightInt>,
        data_source_name: &DataSourceName) -> impl DataSourceOpts {

    match data_source_name {
        DataSourceName::DemArcSec3 => arcsec3::DemArc3SecOpts::new_opts(nodata, sea_level),
    }
}

/// Loads specific data source tile data
pub fn load_tile_data<'a>(data_source_path: &str, data_source_name: &DataSourceName, opts: &'a dyn DataSourceOpts,
        tile_id: &'a TileID) -> Result<Option<impl (TileData<'a>)>, String> {

    match data_source_name {
        DataSourceName::DemArcSec3 =>
            arcsec3::DemArc3SecData::load(data_source_path, opts, &tile_id)
    }
}

/// Struct for passing to mutex
struct MutexStruct {
    heights: Heights,
    colors: Colors,
}

pub trait Model<'a> {
    /// Define spacing parameter
    fn define_spacing(model_size: GeoPointIndex) -> Coord;

    /// Define valid model size
    fn make_valid_model_size(model_size: Option<GeoPointIndex>) -> GeoPointIndex;

    // XXX: Currently, we use the same models for all modes
    /// Creates geopoints for the model
    fn create_modelpoints(model_size: GeoPointIndex, spacing: Coord) -> (ModelPoints, Elements);

    /// Creates texture coordinates
    fn create_texture_coordinates(_model_size: GeoPointIndex) -> TextureCoordinates {
        vec![]
    }

    /// Creates geopoints to tiles mapping
    fn create_geopoints_tiles<'b>(opts: &'b impl DataSourceOpts, geopoints: &'b GeoPoints) -> GeoPointsToTilesMapping<'b> {
        let mut geopoints_tiles: GeoPointsToTilesMapping =
            HashMap::with_capacity(opts.get_max_number_of_tiles());
        for (k, geo_point) in geopoints {
            let tile_id = opts.find_tile_id(geo_point);
            match geopoints_tiles.get_mut(&tile_id) {
                Some(v) => v.push((*k, geo_point)),
                None => {geopoints_tiles.insert(tile_id, vec![(*k, geo_point)]);},
            }
        }
        geopoints_tiles
    }

    /// Checks files and directories
    fn options_check(settings: &'a Settings) -> Result<(), String>;

    /// Creates texture model from model data
    fn build_texture_model(
        settings:               &'a Settings,
        model_size:             GeoPointIndex,
        spacing:                Coord,
        heights:                Heights,
        modelpoints:            ModelPoints,
        elements:               Elements,
        model_type_data:        ModelTypeData) -> Result<Self, String> where Self:Sized;

    /// Creates color model from model data
    fn build_color_model(
        args:                   &'a Settings,
        model_size:             GeoPointIndex,
        spacing:                Coord,
        heights:                Heights,
        modelpoints:            ModelPoints,
        elements:               Elements,
        model_type_data:        ModelTypeData) -> Result<Self, String> where Self:Sized;

    /// Creates model for texture model type
    fn create_with_texture(settings: &'a Settings) -> Result<Self, String> where Self:Sized {
        Self::options_check(settings)?;
        let data_source_name = &settings.data_source;
        let opts = make_data_source_opts(settings.nodata, settings.sea_level, data_source_name);

        let model_size = Self::make_valid_model_size(settings.model_size);
        let spacing = Self::define_spacing(model_size);

        let (modelpoints, elements) = Self::create_modelpoints(model_size, spacing);
        let geopoints_tiles = Self::create_geopoints_tiles(&opts, &modelpoints.geopoints);

        let texture_coordinates = Self::create_texture_coordinates(model_size);

        let mut heights: Heights = BTreeMap::new();
        for k in 0..(2*(model_size+1)*(model_size+1)-1) {
            assert_eq!(heights.insert(k, 0.0), None)
        }

        let tiles_limit=opts.get_max_number_of_tiles();
        let mutex=Mutex::new(heights);

        thread::scope(|scope|{
            for _job in 1..=settings.jobs { scope.spawn(|| {
                while let Some(tile_id) = TileID::next(tiles_limit) {
                    let mut tile_heights: Heights = BTreeMap::new();
                    match geopoints_tiles.get(&tile_id) {
                        Some(tile_geopoints) => {
                            let load_result =
                                    load_tile_data(&settings.data_source_dir, data_source_name, &opts, &tile_id);
                            match load_result {
                                Err(err) => eprintln!("{}", err),
                                Ok(None) => (),
                                Ok(Some(dem_tile)) => {
                                    for (k, geo_point) in tile_geopoints {
                                        match dem_tile.calc_height(geo_point) {
                                            None => (), // Geo point not in the tile
                                            Some(h) => {
                                                tile_heights.insert(*k, h);
                                            }
                                        }
                                    }
                                    let mut heights = mutex.lock().unwrap();
                                    heights.append(&mut tile_heights);
                                    drop(heights);
                                }
                            }
                        },
                        None => (),
                    }
                } });
            }
        });

        let heights_ = mutex.into_inner().unwrap();

        let model_type_data = ModelTypeData::Texture(texture_coordinates);

        Self::build_texture_model(settings, model_size, spacing, heights_, modelpoints, elements, model_type_data)
    }

    /// Creates model for color model type
    fn create_with_color(settings: &'a Settings) -> Result<Self, String> where Self:Sized {
        Self::options_check(settings)?;
        let data_source_name = &settings.data_source;
        let opts = make_data_source_opts(settings.nodata, settings.sea_level, data_source_name);

        let model_size = Self::make_valid_model_size(settings.model_size);
        let spacing = Self::define_spacing(model_size);

        let (modelpoints, elements) = Self::create_modelpoints(model_size, spacing);
        let geopoints_tiles = Self::create_geopoints_tiles(&opts, &modelpoints.geopoints);

        let mut heights: Heights = BTreeMap::new();
        for k in 0..(2*(model_size+1)*(model_size+1)-1) {
            assert_eq!(heights.insert(k, 0.0), None)
        }

        let color_profile_file =
                settings.get_parameter_string("color_profile_file", DEFAULT_COLOR_PROFILE_FILE)?;

        let color_mapping =
                match get_color_mapping(&color_profile_file) {
                    Err(err) =>
                        return Err(format!("Can't find color profile file '{}': {}", &color_profile_file, err)),
                    Ok(func) => func
                };

        let mut colors: Colors = BTreeMap::new();
        for k in 0..(2*(model_size+1)*(model_size+1)-1) {
            colors.insert(k, color_mapping(opts.get_sea_level()));   // XXX: default color is a color of sea_level
        };

        let tiles_limit=opts.get_max_number_of_tiles();
        let mutex=Mutex::new(MutexStruct {heights: heights, colors: colors});

        thread::scope(|scope|{
            for _job in 1..=settings.jobs { scope.spawn(|| {
                while let Some(tile_id) = TileID::next(tiles_limit) {
                    let mut tile_heights: Heights = BTreeMap::new();
                    let mut tile_colors: Colors = BTreeMap::new();
                    match geopoints_tiles.get(&tile_id) {
                        Some(tile_geopoints) => {
                            let load_result =
                                load_tile_data(&settings.data_source_dir, data_source_name, &opts, &tile_id);
                            match load_result {
                                Err(err) => eprintln!("{}", err),
                                Ok(None) => (),
                                Ok(Some(dem_tile)) => {
                                    for (k, geo_point) in tile_geopoints {
                                        match dem_tile.calc_height(geo_point) {
                                            None => (), // Geo point not in the tile
                                            Some(h) => {
                                                let c = color_mapping(h.floor() as HeightInt);
                                                tile_colors.insert(*k, c);
                                                tile_heights.insert(*k, h);
                                            }
                                        }
                                    }
                                    let mut ms = mutex.lock().unwrap();
                                    let MutexStruct {heights, colors } = ms.deref_mut();
                                    heights.append(&mut tile_heights);
                                    colors.append(&mut tile_colors);
                                    drop(ms);
                                }
                            }

                        },
                        None => (),
                    };
                } });
            }
        });

        let MutexStruct {
            heights: heights_,
            colors: colors_,
        } = mutex.into_inner().unwrap();

        let model_type_data = ModelTypeData::Color(colors_);

        Self::build_color_model(settings, model_size, spacing, heights_, modelpoints, elements, model_type_data)
    }

    /// Saves model data to resulting files
    fn save(&self) -> Result<(), String>;
}

