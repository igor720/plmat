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
// use std::path::Components;
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

/// Enum representing model type specific data
// #[derive(Debug)]
// pub enum ModelType {
//     /// Color type model
//     Color(),
//     /// Texture type model
//     Texture(),
// }

pub struct ModelComponents {
    /// Spacing between vertices in the grid
    pub spacing:                Coord,
    /// Elevation values for each vertex in the model
    pub heights:                Heights,
    /// Model type data (either texture or color information)
    pub colors:                 Option<Colors>,
    pub texture_coordinates:    Option<TextureCoordinates>,
    pub vertices:               Option<Vertices>,
    pub texture_mapping:        Option<PointsMapping>,
    pub faces:                  Option<Faces>,
}

impl ModelComponents {
    pub fn get_colors(&self) -> Result<&Colors, ErrBox> {
        self.colors.as_ref().ok_or("Can't get model colors".into())
    }

    pub fn get_texture_coordinates(&self) -> Result<&TextureCoordinates, ErrBox> {
        self.texture_coordinates.as_ref().ok_or("Can't get model texture coordinates".into())
    }

    pub fn get_vertices(&self) -> Result<&Vertices, ErrBox> {
        self.vertices.as_ref().ok_or("Can't get model vertices".into())
    }

    pub fn get_texture_mapping(&self) -> Result<&PointsMapping, ErrBox> {
        self.texture_mapping.as_ref().ok_or("Can't get texture points mapping".into())
    }

    pub fn get_faces(&self) -> Result<&Faces, ErrBox> {
        self.faces.as_ref().ok_or("Can't get model faces".into())
    }
}

/// Type alias for model vertices collection using BTreeMap
/// Key: GeoPointIndex, Value: GeoPoint
pub type Vertices = BTreeMap<GeoPointIndex, GeoPoint>;

/// Type alias for mapping between texture points and vertices using HashMap
/// Key: GeoPointIndex, Value: GeoPointIndex
pub type PointsMapping = HashMap<GeoPointIndex, GeoPointIndex>;

/// Type alias for mapping geographic points to tiles using HashMap
/// Key: TileID, Value: Vector of (GeoPointIndex, &GeoPoint) tuples
pub type VerticesToTilesMapping<'a> = HashMap<TileID, Vec<(GeoPointIndex, &'a GeoPoint)>>;

/// Type alias for model Faces (triangles) using vector of tuples
/// Each tuple represents a triangle defined by three GeoPointIndex values
pub type Faces = Vec<(GeoPointIndex, GeoPointIndex, GeoPointIndex)>;

/// Type alias for elevation data using BTreeMap
/// Key: GeoPointIndex, Value: Height (floating-point elevation value)
pub type Heights = BTreeMap<GeoPointIndex, Height>;

/// Type alias for color data using BTreeMap
/// Key: GeoPointIndex, Value: RGB color value
pub type Colors = BTreeMap<GeoPointIndex, RGB>;

/// Type alias for texture coordinates data using vector of tuples
/// Each tuple represents (u, v) texture coordinates
pub type TextureCoordinates = Vec<(TextureCoordinate, TextureCoordinate)>;

/// Container for model points data including vertices and optional point mapping
pub struct ModelData (pub Vertices, pub Faces, pub Option<PointsMapping>);

impl ModelData {
    pub fn create(vertices: Vertices, faces: Faces, texture_mapping: Option<PointsMapping>) -> Self {
        Self( vertices, faces, texture_mapping )
    }
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

    /// Creates geographic points and Faces for the model
    fn create_modelpoints(model_size: GeoPointIndex, spacing: Coord) -> ModelData; //(ModelPoints, Faces);

    /// Returns number of points in the model
    fn num_model_vertices(_: GeoPointIndex, vertices: &Vertices) -> GeoPointIndex {
        vertices.len() as GeoPointIndex // 2*(model_size+1)*(model_size+1)-1
    }

    /// Creates texture coordinates for the model
    fn create_texture_coordinates(_model_size: GeoPointIndex) -> TextureCoordinates {
        vec![]
    }

    /// Creates mapping between geographic points and tiles
    fn create_vertices_tiles<'b>(opts: &'b impl DataSourceOpts, vertices: &'b Vertices) -> VerticesToTilesMapping<'b> {
        let mut vertices_tiles: VerticesToTilesMapping =
            HashMap::with_capacity(opts.get_max_number_of_tiles());
        for (k, geo_point) in vertices {
            let tile_id = opts.find_tile_id(geo_point);
            match vertices_tiles.get_mut(&tile_id) {
                Some(v) => v.push((*k, geo_point)),
                None => {vertices_tiles.insert(tile_id, vec![(*k, geo_point)]);},
            }
        }
        vertices_tiles
    }

    /// Checks that required files and directories exist for the model
    fn options_check(settings: &'a Settings) -> Result<(), ErrBox>;

    /// Creates a texture model from the provided model data
    /// 
    /// # Arguments
    /// * `model_type` - Type of the model to create (ModelType enum)
    /// * `model_size` - Size of the model to create
    /// * `settings` - Configuration settings for the model
    /// * `components` - Model data
    fn build_model(
        model_type:             ModelType,
        model_size:             GeoPointIndex,
        settings:               &'a Settings,
        components:             ModelComponents,
    ) -> Result<Self, ErrBox> where Self:Sized;

    fn calc() {}


    fn calc1(
            model_type: ModelType,
            color_mapping: impl Fn(HeightInt) -> Result<RGB, ErrBox>,
            tile_heights: &mut Heights, 
            tile_colors: &mut Colors, 
            h_res: Option<Height>,
            k: usize
        ) {
        match h_res {
            None => (), // Geopoint not in the tile
            Some(h) => {
                match model_type {
                    ModelType::Texture => {
                        (*tile_heights).insert(k, h);
                    },
                    ModelType::Color => {
                        match color_mapping(h.floor() as HeightInt) {
                            Ok(c) => {
                                (*tile_colors).insert(k, c);
                                (*tile_heights).insert(k, h);
                            },
                            Err(err) => eprintln!("{}", err),
                        }
                    }
                }
            }
        }
    }


    /// Creates a color model by processing tiles in parallel using threads
    fn create(model_type: ModelType, settings: &'a Settings) -> Result<Self, ErrBox> where Self:Sized {
        Self::options_check(settings)?;
        let data_source_name = &settings.data_source;
        let opts = make_data_source_opts(settings.nodata, settings.sea_level, data_source_name);

        let model_size = Self::make_valid_model_size(settings.model_size);
        let spacing = Self::define_spacing(model_size);

        let ModelData(vertices, faces, texture_mapping) = Self::create_modelpoints(model_size, spacing);
        let vertices_tiles = Self::create_vertices_tiles(&opts, &vertices);

        let texture_coordinates = 
            match model_type {
                ModelType::Texture =>
                    Some(Self::create_texture_coordinates(model_size)),
                ModelType::Color => None,
            };

        let mut heights: Heights = BTreeMap::new();
        for k in 0..Self::num_model_vertices(model_size, &vertices) {
            // Default height is sea level
            heights.insert(k, opts.get_sea_level() as Height);
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
        for k in 0..Self::num_model_vertices(model_size, &vertices) {
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
                        match vertices_tiles.get(&tile_id) {
                            Some(tile_vertices) => {
                                let load_result =
                                    load_tile_data(&settings.data_source_dir, data_source_name, &opts, &tile_id);
                                match load_result {
                                    Err(err) => eprintln!("{}", err),
                                    Ok(None) => (),
                                    Ok(Some(dem_tile)) => {
                                        Self::calc();
                                        for (k, geo_point) in tile_vertices {
                                            let h_res = dem_tile.calc_height(geo_point); 
                                            Self::calc1(model_type, color_mapping, &mut tile_heights, &mut tile_colors, h_res, *k);
                                            // match dem_tile.calc_height(geo_point) {
                                    //             None => (), // Geopoint not in the tile
                                    //             Some(h) => {
                                    //                 match model_type {
                                    //                     ModelType::Texture => {
                                    //                         tile_heights.insert(*k, h);
                                    //                     },
                                    //                     ModelType::Color => {
                                    //                         match color_mapping(h.floor() as HeightInt) {
                                    //                             Ok(c) => {
                                    //                                 tile_colors.insert(*k, c);
                                    //                                 tile_heights.insert(*k, h);
                                    //                             },
                                    //                             Err(err) => eprintln!("{}", err),
                                    //                         }
                                    //                     }
                                    //                 }
                                    //             }
                                            // }
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
            heights: heights_ready,
            colors: colors_,
        } = mutex.into_inner()
            .map_err(|_| "Failed to acquire mutex lock")?;

        let components = ModelComponents {
            spacing,
            heights: heights_ready,
            colors: Some(colors_),
            texture_coordinates,
            vertices: Some(vertices),
            texture_mapping,
            faces: Some(faces),
        };

        Self::build_model(model_type, model_size, settings, components)
    }

    /// Saves the model data to output files
    fn save(&self) -> Result<(), ErrBox>;
}
