//! # X3D Geospatial Model Implementation
//!
//! This module provides the implementation for creating 3D geospatial models in X3D format
//! from Digital Elevation Model (DEM) data. It supports both texture-based and color-based
//! rendering modes for visualizing elevation data on a globe.
//!
//! ## Overview
//!
//! The `X3DGeospatial` struct represents a 3D geospatial model that can be generated from
//! elevation data. It creates a grid of geographic points covering the entire globe and
//! maps elevation values to create a 3D surface. The model supports two rendering approaches:
//! - Texture-based rendering using image mapping
//! - Color-based rendering using vertex colors
//!
//! ## Key Features
//!
//! - **Geographic Coverage**: Models cover the entire globe (180° longitude, 90° latitude)
//! - **Flexible Rendering**: Supports both texture mapping and vertex coloring
//! - **Template-based Output**: Uses X3D template files for consistent output structure
//! - **Elevation Data Integration**: Properly maps elevation values to 3D coordinates
//! - **Configuration Support**: Extensive configuration options through settings
//!
//! ## Data Flow
//!
//! 1. **Input**: DEM data providing elevation values for geographic coordinates
//! 2. **Processing**: Conversion of elevation data into 3D vertex coordinates
//! 3. **Template**: Application of X3D template with elevation and color/texture data
//! 4. **Output**: Generation of complete X3D files ready for visualization
//!
//! ## Implementation Details
//!
//! The model is built on a grid where:
//! - Each vertex has geographic coordinates (longitude, latitude) and elevation
//! - The grid spans from -180° to +180° longitude and -90° to +90° latitude
//! - Vertex spacing is calculated to ensure proper geographic coverage
//! - Elevation values are interpolated and mapped to 3D coordinates
//!
//! ## Usage
//!
//! To create an X3D geospatial model:
//! 1. Configure settings with appropriate parameters
//! 2. Prepare elevation data (heights)
//! 3. Call `build_texture_model()` or `build_color_model()` to create the model
//! 4. Use `save()` method to generate the final X3D file
//!
//! ## Configuration Parameters
//!
//! - `template_file_x3d`: Path to the X3D template file (default: "./geospatial.x3d.template")
//! - `texture_uri`: URI for texture mapping (default: "\"texture.png\"")
use std::fs::File;
use std::path::Path;
use std::io::Write;
use std::collections::{BTreeMap};
use quick_xml::events::{Event, BytesEnd, BytesStart};
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;
use crate::common::types::*;
use crate::common::settings::*;
use crate::common::util::check_file;
use crate::model::types::*;


/// Minimum valid model size for X3D geospatial models
const MIN_VALID_MODEL_SIZE: usize = 4;
/// Default template file path for X3D geospatial models
const DEFAULT_TEMPLATE_FILE: &str = "./geospatial.x3d.template";
/// Default texture URI for X3D geospatial models
const DEFAULT_TEXTURE_URI: &str = "\"texture.png\"";


/// X3DGeospatial model structure
/// 
/// This struct represents a 3D geospatial model in X3D format that can be
/// created from Digital Elevation Model (DEM) data. It supports both
/// texture-based and color-based rendering modes.
/// 
/// The model is built on a grid where each vertex has geographic coordinates
/// (longitude and latitude) and elevation values.
pub struct X3DGeospatial<'a> {
    /// Model type (Color() or Texture())
    model_type:         ModelType,
    /// Size of the model grid (number of vertices along each dimension)
    model_size:         GeoPointIndex,
    /// Reference to the settings configuration
    settings:           &'a Settings<'a>,
    /// Model data
    components:         ModelComponents,
    /// Path to the X3D template file used for output generation
    template_file:      String,
}

impl<'a> Model<'a> for X3DGeospatial<'a> {
    /// Validates and returns a valid model size
    /// 
    /// Ensures the model size is at least the minimum valid size.
    /// If no size is provided, returns the minimum valid size.
    fn make_valid_model_size(model_size: Option<GeoPointIndex>) -> GeoPointIndex {
        let _model_size = model_size.unwrap_or(MIN_VALID_MODEL_SIZE);
        if _model_size<MIN_VALID_MODEL_SIZE {MIN_VALID_MODEL_SIZE} else {_model_size}
    }

    /// Defines the spacing between vertices in the model grid
    /// 
    /// Calculates the spacing based on the model size to ensure proper
    /// geographic coverage of the entire globe (180 degrees in longitude).
    fn define_spacing(model_size: GeoPointIndex) -> Coord {
        180.0/(model_size as Coord)
    }

    /// Creates all geographic points (vertices) for the model
    /// 
    /// Generates a grid of geographic points covering the entire globe.
    /// The points are arranged in a specific pattern to create the 3D surface.
    fn create_modelpoints(model_size: GeoPointIndex, spacing: Coord) -> ModelData {
        // let mut vertices: Vertices = HashMap::with_capacity(2*(model_size+1)*(model_size+1));
        let mut vertices: Vertices = BTreeMap::new();

        let model_size2 = 2*model_size as GeoPointIndex;
        let mut count: GeoPointIndex = 0;
        for j in 0..=model_size {
            for i in 0..=model_size2 {
                vertices.insert(count, GeoPoint {
                    // wraping around the global dateline
                    lon: (-180.0) + spacing*(if i==model_size2 {0} else {i} as Coord),
                    lat: (-90.0)  + spacing*(j as Coord)
                });
                count+=1;
            }
        }

        ModelData::create( vertices, vec!(), None )
    }

    /// Checks that required files and directories exist
    /// 
    /// Validates that the X3D template file exists and is accessible.
    /// This is necessary for generating the final X3D output file.
    fn options_check(settings: &'a Settings) -> Result<(), ErrBox> {
        let template_file: String =
                settings.get_parameter_str("template_file_x3d", DEFAULT_TEMPLATE_FILE.to_string())?;
        check_file(&template_file)
    }

    fn build_model(
        model_type:         ModelType,
        model_size:         GeoPointIndex,
        settings:           &'a Settings,
        components:         ModelComponents,
    ) -> Result<Self, ErrBox> where Self:Sized {
        let template_file: String =
                settings.get_parameter_str("template_file_x3d", DEFAULT_TEMPLATE_FILE.to_string())?;
        check_file(&template_file)?;

        return Ok(X3DGeospatial{
            model_type,
            model_size,
            settings,
            components,
            template_file
        })
    }

    /// Saves the model data to X3D output files
    /// 
    /// Writes the final X3D file by processing the template file and
    /// inserting the elevation data and color/texture information.
    fn save(&self) -> Result<(), ErrBox> {
        let settings = self.settings;
        let planet_name = &settings.planet_name;
        let output_path = &settings.output_dir;

        let mut reader = match Reader::from_file(&self.template_file) {
            Ok(r) => r,
            Err(err) => return Err(format!("Can't read template: {}", err).into())
        };
        reader.config_mut().check_comments = true;

        let mut buf = Vec::new();

        let _result_path = Path::new(&output_path)
                .join(&planet_name)
                .with_extension("x3d");
        let result_path = match _result_path.to_str() {
            Some(fp) => fp,
            None => return Err("Can't get file path for result data".into())
        };

        let buffer = match File::create(result_path) {
            Ok(f) => f,
            Err(err) => return Err(format!("Can't write to output file: {}", err).into())
        };

        let height_values =
            self.components.heights
            .values().map(|v| {v.to_string()})
            .collect::<Vec<String>>()
            .join(" ");

        let color_values = match &self.model_type {
            ModelType::Texture => "".to_string(),
            ModelType::Color =>
                self.components.get_colors()?
                    .values()
                    .map(|v| {v.to_string()})
                    .collect::<Vec<String>>()
                    .join(" ")
        };

        let create_height_attr = |elem: &mut BytesStart<'static>| {
                elem.push_attribute(("xDimension", (2*(self.model_size)+1).to_string().as_str()));
                elem.push_attribute(("xSpacing", (self.components.spacing.to_string().as_str())));
                elem.push_attribute(("zDimension", ((self.model_size)+1).to_string().as_str()));
                elem.push_attribute(("zSpacing", (self.components.spacing.to_string().as_str())));
                elem.push_attribute(("height", height_values.as_str()));
        };

        let mut writer = Writer::new(buffer);
        let mut in_geo_elevation_grid = false;
        loop {
            match reader.read_event_into(&mut buf) {
                Err(e) => return Err(format!("Error at position {}: {:?}", reader.buffer_position(), e).into()),
                Ok(Event::Eof) => break,
                Ok(Event::Empty(e))
                        if e.name().as_ref() == b"_GeoElevationGrid" => {
                    let mut elem = BytesStart::new("GeoElevationGrid");
                    elem.extend_attributes(e.attributes().map(|attr| attr.unwrap()));
                    create_height_attr(&mut elem);

                    assert!(writer.write_event(Event::Empty(elem)).is_ok());
                }
                Ok(Event::Start(e))
                        if e.name().as_ref() == b"_GeoElevationGrid" => {
                    in_geo_elevation_grid = true;
                    let mut elem = BytesStart::new("GeoElevationGrid");
                    elem.extend_attributes(e.attributes().map(|attr| attr.unwrap()));
                    create_height_attr(&mut elem);

                    assert!(writer.write_event(Event::Start(elem)).is_ok());
                },
                Ok(Event::End(e))
                        if e.name().as_ref() == b"_GeoElevationGrid" => {
                    in_geo_elevation_grid = false;
                    let elem = BytesEnd::new("GeoElevationGrid");

                    assert!(writer.write_event(Event::End(elem)).is_ok());
                },
                Ok(Event::Empty(e))
                    if e.name().as_ref() == b"_Color" && in_geo_elevation_grid => {
                        let mut elem = BytesStart::new("Color");

                        match &self.model_type {
                            ModelType::Texture => (),
                            ModelType::Color => {
                                elem.extend_attributes(e.attributes().map(|attr| attr.unwrap()));
                                elem.push_attribute(("color", color_values.as_str()))
                            }
                        };

                        assert!(writer.write_event(Event::Empty(elem)).is_ok());
                    },
                Ok(Event::Empty(e))
                    if e.name().as_ref() == b"_ImageTexture" => {
                        let texture_uri =
                                settings.get_parameter_str("texture_uri", DEFAULT_TEXTURE_URI.to_string())?;
                        let mut elem = BytesStart::new("ImageTexture");
                        match &self.model_type {
                            ModelType::Texture => {
                                elem.push_attribute(("url", &texture_uri[..]))
                            },
                            ModelType::Color => (),
                        };

                        assert!(writer.write_event(Event::Empty(elem)).is_ok());
                    },
                Ok(e) => assert!(writer.write_event(e).is_ok()),
            }
            buf.clear();
        }

        Ok(writer.into_inner().flush()?)
    }

}


#[cfg(test)]
mod tests {
    // Test cases would go here
}
