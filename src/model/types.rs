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
use std::path::Path;
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

/// Container for the components that make up a 3D model
/// 
/// This struct holds all the essential data elements that constitute a 3D model,
/// including geometric information, texture data, and optional color information.
/// It serves as a central container for model data during the creation and processing
/// of 3D models from DEM (Digital Elevation Model) data.
/// 
/// The struct is designed to be flexible, allowing for different model types (texture
/// vs color) while maintaining a consistent interface for accessing model components.
pub struct ModelComponents {
    /// The distance between vertices in the model grid, determining the
    /// resolution and density of the 3D surface
    pub spacing:                Coord,
    /// A mapping of vertex indices to elevation values, forming the
    /// foundation of the 3D terrain geometry
    pub heights:                Heights,
    /// Optional color data for each vertex, used in color models to
    /// provide visual representation of elevation values
    pub colors:                 Option<Colors>,
    /// Optional UV coordinates for texture mapping, used
    /// in texture models to map texture images onto the 3D surface    
    pub texture_coordinates:    Option<TextureCoordinates>,
    /// Optional collection of geographic points that define the 3D
    /// model geometry, mapping vertex indices to actual geographic coordinates
    pub vertices:               Option<Vertices>,
    /// Optional mapping between texture points and vertices,
    /// used to associate texture coordinates with specific model vertices
    pub texture_mapping:        Option<PointsMapping>,
    /// Optional triangular faces that define the connectivity of vertices
    /// in the 3D mesh, creating the surface structure of the model
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
pub fn load_tile_data<'a>(data_source_path: &Path, data_source_name: &DataSourceName, opts: &'a dyn DataSourceOpts,
        tile_id: &'a TileID) -> Result<Option<impl TileData<'a>>, String> {

    match data_source_name {
        DataSourceName::DemArcSec3 =>
            arcsec3::DemArc3SecData::load(data_source_path, opts, &tile_id)
    }
}

/// A wrapper struct for thread-safe access to model data during parallel processing
/// 
/// This struct encapsulates the shared elevation and color data that needs to be
/// accessed by multiple threads during the model creation process. It's wrapped in
/// a `Mutex` to ensure thread-safe access and prevent data races when multiple
/// threads update the model's elevation and color information simultaneously.
pub struct MutexStruct {
    heights: Heights,
    colors: Colors,
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
    fn create_modeldata(model_size: GeoPointIndex, spacing: Coord) -> ModelData;

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

    /// Do computations of model vertices for a given tile
    /// 
    /// This function processes elevation data for a specific DEM tile and updates
    /// the model's height and color information for vertices that fall within this tile.
    /// It handles both texture and color model types by either storing elevation values
    /// directly (for texture models) or mapping elevation values to colors (for color models).
    /// 
    /// # Arguments
    /// * `model_type` - The type of model being created (Texture or Color)
    /// * `settings` - Configuration settings for the model creation process
    /// * `data_source_name` - The name of the data source being used (e.g., SRTM 3-arcsecond)
    /// * `opts` - Data source options for the current processing context
    /// * `color_mapping` - Color mapping object used to convert elevation values to colors
    /// * `tile_id` - Identifier for the DEM tile being processed
    /// * `tile_vertices` - Vector of vertex indices and geographic points that fall within this tile
    /// * `mutex` - Thread-safe mutex protecting shared model data (elevations and colors)
    /// * `tile_heights` - Mutable reference to store elevation values for vertices in this tile
    /// * `tile_colors` - Mutable reference to store color values for vertices in this tile
    /// 
    /// # Processing Flow
    /// 1. Loads the DEM tile data for the given tile_id
    /// 2. For each vertex in the tile:
    ///    - Calculates the elevation value at that geographic point
    ///    - For texture models: stores the elevation value directly
    ///    - For color models: converts elevation to color using the color mapping
    /// 3. Updates the shared model data through the mutex
    /// 
    /// # Error Handling
    /// - If tile loading fails, an error message is printed to stderr
    /// - If elevation calculation fails, the vertex is skipped
    /// - If color mapping fails for color models, an error message is printed to stderr
    /// 
    /// # Thread Safety
    /// This function is designed to be called from multiple threads in parallel.
    /// It uses a mutex to safely update shared model data structures.
    fn calc_tile(
            model_type: ModelType,
            settings: &Settings, 
            data_source_name: &DataSourceName, 
            opts: &impl DataSourceOpts, 
            color_mapping: &ColorMapping,
            tile_id: TileID,
            tile_vertices: &Vec<(usize, &GeoPoint)>,
            mutex: &Mutex<MutexStruct>,
            tile_heights: &mut Heights, 
            tile_colors: &mut Colors, 
        ) {
        let load_result =
            load_tile_data(&settings.data_source_dir, &data_source_name, opts, &tile_id);
        match load_result {
            Err(err) => eprintln!("{}", err),
            Ok(None) => (), // Missed data for this tile: default elevations and colors
            Ok(Some(dem_tile)) => {
                for (k, geo_point) in tile_vertices {
                    match dem_tile.calc_height(geo_point) {
                        None => (), // Geopoint is not in the tile
                        Some(h) => {
                            match model_type {
                                ModelType::Texture => {
                                    (*tile_heights).insert(*k, h);
                                },
                                ModelType::Color => {
                                    match color_mapping.get_color(h.floor() as HeightInt) {
                                        Ok(c) => {
                                            (*tile_colors).insert(*k, c);
                                            (*tile_heights).insert(*k, h);
                                        },
                                        Err(err) => eprintln!("{}", err),
                                    }
                                }
                            }
                        }
                    }
                }
                let mut ms = mutex.lock().unwrap();
                let MutexStruct {heights, colors } = ms.deref_mut();
                heights.append(tile_heights);
                colors.append(tile_colors);
                drop(ms);
            }
        } 
    }

    /// Creates a model by processing tiles in parallel using threads
    /// 
    /// This is the main entry point for model creation that orchestrates the complete
    /// workflow of generating 3D models from DEM data. It handles configuration validation,
    /// model point generation, tile distribution, parallel processing of DEM tiles, and
    /// final model construction.
    /// 
    /// # Arguments
    /// * `model_type` - The type of model to create (Texture or Color)
    /// * `settings` - Configuration settings for the model creation process
    /// 
    /// # Processing Workflow
    /// 1. **Configuration Validation**: Validates that required files and directories exist
    ///    using the `options_check` method
    /// 2. **Model Size and Spacing Calculation**: Determines the appropriate model size and
    ///    vertex spacing based on input parameters
    /// 3. **Model Point Generation**: Creates the geographic points and triangular faces that
    ///    define the 3D model structure
    /// 4. **Tile Mapping**: Associates geographic points with DEM tiles for efficient processing
    /// 5. **Parallel Tile Processing**: Distributes tile processing across multiple threads:
    ///    - Each thread processes tiles in a work-stealing pattern
    ///    - For each tile, loads DEM data and calculates elevation values for vertices
    ///    - Updates shared model data structures through thread-safe mutex operations
    /// 6. **Model Construction**: Builds the final model using the `build_model` method
    /// 
    /// # Thread Safety
    /// The function uses a `Mutex` to protect shared model data structures (elevations and colors)
    /// during parallel processing. Each thread processes its assigned tiles independently,
    /// but safely updates the shared data through the mutex.
    /// 
    /// # Error Handling
    /// - Configuration validation errors are propagated as `ErrBox` results
    /// - Tile loading failures are logged to stderr but don't stop processing
    /// - Elevation calculation failures skip individual vertices
    /// - Color mapping failures are logged to stderr but don't stop processing
    /// - Mutex acquisition failures are converted to `ErrBox` results
    /// 
    /// # Performance Considerations
    /// - Uses multi-threading to parallelize tile processing across available CPU cores
    /// - Implements work-stealing pattern for efficient load balancing
    /// - Minimizes data copying by using references and mutable borrows
    /// - Processes tiles in batches to reduce thread synchronization overhead
    /// 
    /// # Data Flow
    /// 1. Initial configuration and model setup
    /// 2. Generation of geographic points and triangular faces
    /// 3. Distribution of points to tiles for processing
    /// 4. Parallel processing of tiles with elevation/color calculations
    /// 5. Aggregation of results from all threads into final model data
    /// 6. Final model construction and return
    /// 
    /// # Panics
    /// This function will panic if:
    /// - Mutex lock acquisition fails (indicating a serious system error)
    fn create(model_type: ModelType, settings: &'a Settings) -> Result<Self, ErrBox> where Self:Sized {
        // Check here before long calculation times
        Self::options_check(settings)?;
        let data_source_name = &settings.data_source;
        let opts = make_data_source_opts(settings.nodata, settings.sea_level, data_source_name);

        let model_size = Self::make_valid_model_size(settings.model_size);
        let spacing = Self::define_spacing(model_size);

        let ModelData(vertices, faces, texture_mapping) = Self::create_modeldata(model_size, spacing);
        let vertices_tiles = Self::create_vertices_tiles(&opts, &vertices);

        let texture_coordinates = 
            match model_type {
                ModelType::Texture =>
                    Some(Self::create_texture_coordinates(model_size)),
                ModelType::Color => None,
            };

        let mut heights: Heights = BTreeMap::new();
        for k in 0..Self::num_model_vertices(model_size, &vertices) {
            // The default height is the sea level
            heights.insert(k, opts.get_sea_level() as Height);
        }

        let color_profile_file =
                settings.get_parameter_str("color_profile_file", DEFAULT_COLOR_PROFILE_FILE)?;

        let color_mapping =
                ColorMapping::create(Path::new(&color_profile_file))
                .map_err(|err| {
                    format!("Can't create color mapping from file '{}': {}", &color_profile_file, err)
                })?;

        // Fill 'colors' with default color
        // The default color is the color of sea level
        let default_color = color_mapping.get_color(opts.get_sea_level())
                .map_err(|err| {
                    format!("Can't get default color from color profile file '{}': {}", &color_profile_file, err)
                })?;
        let mut colors: Colors = BTreeMap::new();
        for k in 0..Self::num_model_vertices(model_size, &vertices) {
            colors.insert(k, default_color);
        };

        let tiles_limit=opts.get_max_number_of_tiles();

        let mutex=Mutex::new(MutexStruct {heights: heights, colors: colors});

        thread::scope(|scope|{
            for _job in 1..=settings.jobs { 
                scope.spawn(|| {
                    while let Some(tile_id) = TileID::next(tiles_limit) {
                        let mut tile_heights: Heights = BTreeMap::new();
                        let mut tile_colors: Colors = BTreeMap::new();
                        match vertices_tiles.get(&tile_id) {
                            Some(tile_vertices) => {
                                Self::calc_tile(
                                    model_type,
                                    settings, 
                                    data_source_name,
                                    &opts, 
                                    &color_mapping,
                                    tile_id,
                                    tile_vertices,
                                    &mutex,
                                    &mut tile_heights, 
                                    &mut tile_colors, 
                                )
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
            .map_err(|err| format!("Failed to acquire mutex lock: {}", err))?;

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
    /// 
    /// This function is responsible for persisting the created 3D model data to 
    /// disk in the specified output format. The function handles writing all 
    /// components of the model (geometry, texture coordinates, faces, etc.) 
    /// to appropriate files based on the model type and user configuration.
    /// 
    /// # File Generation
    /// The function generates output files in the directory specified by 
    /// `settings.output_dir` with filenames based on the `settings.output_file` 
    /// parameter. The exact format and naming convention depends on the 
    /// output format specified in the settings.
    /// 
    /// # Thread Safety
    /// This function is designed to be called on a single-threaded context 
    /// after all model processing is complete. It does not perform any 
    /// thread-safe operations as it's meant to be called from the main thread 
    /// after parallel processing is finished.
    /// 
    /// # Performance Considerations
    /// - File I/O operations are performed synchronously
    /// - Large models may take considerable time to write to disk
    /// - Memory usage is minimal as data is written incrementally
    fn save(&self) -> Result<(), ErrBox>;
}
