//! # Model Module
//!
//! This module provides the core functionality for creating and managing 3D models
//! from Digital Elevation Model (DEM) data. It handles the creation of both texture
//! and color-based models using multi-threaded processing for improved performance.
//!
//! ## Overview
//!
//! The module implements a trait-based approach for model creation, allowing for
//! flexible model types (texture or color) while maintaining a consistent interface.
//! It manages the processing of DEM tiles in parallel using threads, efficiently
//! handling large datasets by distributing the workload across available CPU cores.
//!
//! ## Key Components
//!
//! - **Model Points**: Geographic points that make up the 3D model
//! - **Tile Mapping**: Association between geographic points and DEM tiles
//! - **Multi-threading**: Parallel processing of tiles for improved performance
//! - **Data Sources**: Support for different DEM data sources (currently 3-arcsecond SRTM)
//! - **Color Mapping**: Conversion of elevation data to visual colors using color profiles
//!
//! ## Features
//!
//! - Thread-safe processing of DEM data using mutexes
//! - Support for both texture and color model creation
//! - Automatic handling of tile boundaries and data overlap
//! - Configurable model size and spacing
//! - Integration with color profile files for elevation-to-color mapping
//! - Comprehensive error handling and validation
//!
//! ## Usage
//!
//! Models are created using the `create_with_texture` or `create_with_color` methods
//! which handle the complete workflow from data loading to model construction.
//!
//! ## Data Flow
//!
//! 1. Configuration validation
//! 2. Model size and spacing calculation
//! 3. Geographic point generation
//! 4. Tile mapping and distribution
//! 5. Parallel tile data loading and processing
//! 6. Model construction and output generation
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

/// Default path for color profile file
const DEFAULT_COLOR_PROFILE_FILE: &str = "./color_profile";

/// Container for model points data including geopoints and optional point mapping
#[derive(Debug)]
pub struct ModelPoints {
    /// Collection of geographic points that make up the model
    pub geopoints: GeoPoints,
    /// Optional mapping between texture points and geopoints
    pub points_map_opt: Option<PointsMapping>,
}

/// Type alias for geographic points collection using BTreeMap
/// Key: GeoPointIndex, Value: GeoPoint
pub type GeoPoints = BTreeMap<GeoPointIndex, GeoPoint>;

/// Type alias for mapping between texture points and geopoints using HashMap
/// Key: GeoPointIndex, Value: GeoPointIndex
pub type PointsMapping = HashMap<GeoPointIndex, GeoPointIndex>;

/// Type alias for mapping geographic points to tiles using HashMap
/// Key: TileID, Value: Vector of (GeoPointIndex, &GeoPoint) tuples
pub type GeoPointsToTilesMapping<'a> = HashMap<TileID, Vec<(GeoPointIndex, &'a GeoPoint)>>;

/// Type alias for model elements (triangles) using vector of tuples
/// Each tuple represents a triangle defined by three GeoPointIndex values
pub type Elements = Vec<(GeoPointIndex, GeoPointIndex, GeoPointIndex)>;

/// Type alias for elevation data using BTreeMap
/// Key: GeoPointIndex, Value: Height (floating-point elevation value)
pub type Heights = BTreeMap<GeoPointIndex, Height>;

/// Type alias for color data using BTreeMap
/// Key: GeoPointIndex, Value: RGB color value
pub type Colors = BTreeMap<GeoPointIndex, RGB>;

/// Type alias for texture coordinates data using vector of tuples
/// Each tuple represents (u, v) texture coordinates
pub type TextureCoordinates = Vec<(TextureCoordinate, TextureCoordinate)>;

/// Enum representing model type specific data
#[derive(Debug)]
pub enum ModelTypeData {
    /// Color data for the model
    Color(Colors),
    /// Texture coordinates data for the model
    Texture(TextureCoordinates),
}

/// Creates and returns a data source options struct based on the specified data source name
pub fn make_data_source_opts(nodata: Option<HeightInt>, sea_level: Option<HeightInt>,
        data_source_name: &DataSourceName) -> impl DataSourceOpts {

    match data_source_name {
        DataSourceName::DemArcSec3 => arcsec3::DemArc3SecOpts::new_opts(nodata, sea_level),
    }
}

/// Loads and returns tile data for a specific data source and tile ID
pub fn load_tile_data<'a>(data_source_path: &str, data_source_name: &DataSourceName, opts: &'a dyn DataSourceOpts,
        tile_id: &'a TileID) -> Result<Option<impl TileData<'a>>, String> {

    match data_source_name {
        DataSourceName::DemArcSec3 =>
            arcsec3::DemArc3SecData::load(data_source_path, opts, &tile_id)
    }
}

/// Trait defining the interface for model creation and management
/// This trait provides methods for creating different types of models (texture/color) 
/// and managing their data processing
pub trait Model<'a> {
    /// Defines the spacing parameter for model creation based on model size
    fn define_spacing(model_size: GeoPointIndex) -> Coord;

    /// Defines a valid model size, ensuring it meets minimum requirements
    fn make_valid_model_size(model_size: Option<GeoPointIndex>) -> GeoPointIndex;

    /// Creates geographic points and elements for the model
    fn create_modelpoints(model_size: GeoPointIndex, spacing: Coord) -> (ModelPoints, Elements);

    /// Returns number of points in the model
    fn num_model_vertices(_: GeoPointIndex, modelpoints: &ModelPoints) -> GeoPointIndex {
        modelpoints.geopoints.len() as GeoPointIndex // 2*(model_size+1)*(model_size+1)-1
    }

    /// Creates texture coordinates for the model
    fn create_texture_coordinates(_model_size: GeoPointIndex) -> TextureCoordinates {
        vec![]
    }

    /// Creates mapping between geographic points and tiles
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

    /// Checks that required files and directories exist for the model
    fn options_check(settings: &'a Settings) -> Result<(), ErrBox>;

    /// Creates a texture model from the provided model data
    /// 
    /// # Arguments
    /// * `settings` - Configuration settings for the model
    /// * `model_size` - Size of the model to create
    /// * `spacing` - Spacing between points in the model
    /// * `heights` - Elevation data for the model
    /// * `modelpoints` - Model points data
    /// * `elements` - Model elements (triangles)
    /// * `model_type_data` - Type-specific model data (texture coordinates)
    /// 
    /// # Returns
    /// Result containing the created model or error
    fn build_texture_model(
        settings:               &'a Settings,
        model_size:             GeoPointIndex,
        spacing:                Coord,
        heights:                Heights,
        modelpoints:            ModelPoints,
        elements:               Elements,
        model_type_data:        ModelTypeData) -> Result<Self, ErrBox> where Self:Sized;

    /// Creates a color model from the provided model data
    /// 
    /// # Arguments
    /// * `args` - Configuration settings for the model
    /// * `model_size` - Size of the model to create
    /// * `spacing` - Spacing between points in the model
    /// * `heights` - Elevation data for the model
    /// * `modelpoints` - Model points data
    /// * `elements` - Model elements (triangles)
    /// * `model_type_data` - Type-specific model data (colors)
    /// 
    /// # Returns
    /// Result containing the created model or error
    fn build_color_model(
        args:                   &'a Settings,
        model_size:             GeoPointIndex,
        spacing:                Coord,
        heights:                Heights,
        modelpoints:            ModelPoints,
        elements:               Elements,
        model_type_data:        ModelTypeData) -> Result<Self, ErrBox> where Self:Sized;

    /// Creates a texture model by processing tiles in parallel using threads
    fn create_with_texture(settings: &'a Settings) -> Result<Self, ErrBox> where Self:Sized {
        Self::options_check(settings)?;
        let data_source_name = &settings.data_source;
        let opts = make_data_source_opts(settings.nodata, settings.sea_level, data_source_name);

        let model_size = Self::make_valid_model_size(settings.model_size);
        let spacing = Self::define_spacing(model_size);

        let (modelpoints, elements) = Self::create_modelpoints(model_size, spacing);
        let geopoints_tiles = Self::create_geopoints_tiles(&opts, &modelpoints.geopoints);

        let texture_coordinates = Self::create_texture_coordinates(model_size);

        let mut heights: Heights = BTreeMap::new();
        for k in 0..Self::num_model_vertices(model_size, &modelpoints) {
            // Default height is sea level
            assert_eq!(heights.insert(k, opts.get_sea_level() as Height), None)
        }

        let tiles_limit=opts.get_max_number_of_tiles();
        let mutex=Mutex::new(heights);

        thread::scope(|scope|{
            for _job in 1..=settings.jobs { 
                scope.spawn(|| {
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
                    } 
                });
            }
        });

        let heights_ = mutex.into_inner()
            .map_err(|_| "Failed to acquire mutex lock")?;

        let model_type_data = ModelTypeData::Texture(texture_coordinates);

        Self::build_texture_model(settings, model_size, spacing, heights_, modelpoints, elements, model_type_data)
    }

    /// Creates a color model by processing tiles in parallel using threads
    fn create_with_color(settings: &'a Settings) -> Result<Self, ErrBox> where Self:Sized {
        Self::options_check(settings)?;
        let data_source_name = &settings.data_source;
        let opts = make_data_source_opts(settings.nodata, settings.sea_level, data_source_name);

        let model_size = Self::make_valid_model_size(settings.model_size);
        let spacing = Self::define_spacing(model_size);

        let (modelpoints, elements) = Self::create_modelpoints(model_size, spacing);
        let geopoints_tiles = Self::create_geopoints_tiles(&opts, &modelpoints.geopoints);

        let mut heights: Heights = BTreeMap::new();
        for k in 0..Self::num_model_vertices(model_size, &modelpoints) {
            // Default height is sea level
            assert_eq!(heights.insert(k, opts.get_sea_level() as Height), None)
        }

        let color_profile_file =
                settings.get_parameter_str("color_profile_file", DEFAULT_COLOR_PROFILE_FILE.to_string())?;

        let color_mapping =
                match get_color_mapping(&color_profile_file) {
                    Err(err) =>
                        return Err(format!("Can't find color profile file '{}': {}", &color_profile_file, err).into()),
                    Ok(func) => func
                };

        // Fill 'colors' with default color
        let default_color = match color_mapping(opts.get_sea_level()) { // the default color is the color of sea level
            Ok(c) => c,
            Err(err) =>
                return Err(format!("Can't get default color from color profile file '{}': {}", &color_profile_file, err).into())
        };
        let mut colors: Colors = BTreeMap::new();
        for k in 0..Self::num_model_vertices(model_size, &modelpoints) {
            colors.insert(k, default_color);
        };

        let tiles_limit=opts.get_max_number_of_tiles();

        /// Struct for passing data to mutex for thread-safe operations
        struct MutexStruct {
            heights: Heights,
            colors: Colors,
        }
        let mutex=Mutex::new(MutexStruct {heights: heights, colors: colors});

        thread::scope(|scope|{
            for _job in 1..=settings.jobs { 
                scope.spawn(|| {
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
                                                    match color_mapping(h.floor() as HeightInt) {
                                                        Ok(c) => {
                                                            tile_colors.insert(*k, c);
                                                            tile_heights.insert(*k, h);
                                                        },
                                                        Err(err) => eprintln!("{}", err),
                                                    };
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
                    } 
                });
            }
        });

        let MutexStruct {
            heights: heights_,
            colors: colors_,
        } = mutex.into_inner()
            .map_err(|_| "Failed to acquire mutex lock")?;

        let model_type_data = ModelTypeData::Color(colors_);

        Self::build_color_model(settings, model_size, spacing, heights_, modelpoints, elements, model_type_data)
    }

    /// Saves the model data to output files
    fn save(&self) -> Result<(), ErrBox>;
}
