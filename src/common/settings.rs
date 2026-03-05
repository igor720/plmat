//! Module for handling application settings and configuration
//! 
//! This module provides functionality for reading and parsing YAML configuration files,
//! as well as managing application settings for different model types and data sources.
#[allow(dead_code)]

use std::fs::read_to_string;
use yaml_rust2::{Yaml, YamlLoader};
use std::str::FromStr;

use crate::common::types::*;
use crate::common::util::*;
use crate::common::args::*;
use crate::common::args::MySubCommandEnum::*;


const DEFAULT_DATA_SOURCE_DIR: &str = "./";
const DEFAULT_OUTPUT_DIR: &str = "./";


/// Reads the settings file from the specified filepath
fn read_settings_file(filepath: &str) -> Result<String, ErrHandle> {
    Ok(read_to_string(filepath)?)
}

/// Reads and parses the settings YAML file
pub fn get_settings_yaml (filepath: &str) -> Result<Yaml, ErrHandle> {
    match read_settings_file(filepath) {
        Ok(s) => {
            let docs = YamlLoader::load_from_str(&s)
                .map_err(|err| {format!("Malformed YAML settings file: {}", err)})?;
            let doc = &docs[0];
            Ok(doc.clone())
        },
        Err(err) =>
            Err(format!("Can't read YAML settings file from the current directory: {}", err).into())
    }
}

/// Main settings structure for the application
/// 
/// This structure holds all the configuration parameters needed for processing
/// geographic data and generating 3D models. It includes both global settings
/// and model-specific configurations.
pub struct Settings<'a> {
    /// Name of the planet being processed
    pub planet_name: &'a str,
    /// Size of the model in terms of geopoints
    pub model_size: Option<GeoPointIndex>,
    /// Number of parallel jobs to run during processing
    pub jobs: usize,
    /// Type of data source being used
    pub data_source: DataSourceName,
    /// Directory path where source data tiles are located
    pub data_source_dir: String,
    /// Directory path where output files will be written
    pub output_dir: String,
    /// Value representing no data in the elevation data
    pub nodata: Option<HeightInt>,
    /// Default sea level for the model
    pub sea_level: Option<HeightInt>,
    /// Tuple containing common and specific settings for the model
    pub specific: (&'a Yaml, &'a Yaml)
}

impl<'a> Settings<'a> {
    /// Creates a Settings instance from command line arguments and configuration YAML
    pub fn make_settings (tl_commands: &'a TopLevelCommands, settings: &'a Yaml) -> Result<Self, ErrHandle> {
        let make = |args: &'a dyn Args, model_name: &str| {
            let data_source = args.data_source();
            let planet_name = args.planet_name().as_str();
            let model_size = args.model_size();
            let jobs = args.jobs();

            let y_ds = match data_source {
                DataSourceName::DemArcSec3 => &settings["DataSource"]["DemArcSec3"],
            };
            if y_ds.is_badvalue() {
                return Err(format!("Common section for '{}' is missed in settings file", model_name).into())
            };

            let nodata = y_ds["data_source_path"].as_i64().map(|i| {i as HeightInt});
            let sea_level = y_ds["sea_level"].as_i64().map(|i| {i as HeightInt});

            let data_source_dir = args.data_source_dir()
                    .map(|s| {s.as_str()})
                    .or_else(|| {y_ds["data_source_path"].as_str()})
                    .unwrap_or(DEFAULT_DATA_SOURCE_DIR)
                    .to_string();
            check_dir(&data_source_dir)?;

            let y0 = &settings["Model"][model_name]["Common"];
            if y0.is_badvalue() {
                return Err(format!("Common section for '{}' is missed in settings file", model_name).into())
            };

            let y1 = match &args.model_type() {
                ModelType::TextureModelType => &settings["Model"][model_name]["Texture"],
                ModelType::ColorModelType => &settings["Model"][model_name]["Color"],
            };
            if y1.is_badvalue() {
                return Err(format!("The model type section for '{}' is missed in settings file", model_name).into())
            };

            let output_dir = args.output_dir()
                    .map(|s| {s.as_str()})
                    .or_else(|| {
                        if y1["output_dir"].is_badvalue() {
                            if y0["output_dir"].is_badvalue() {None} else {y0["output_dir"].as_str()}
                        } else {
                            y1["output_dir"].as_str()
                        }
                    })
                    .unwrap_or(DEFAULT_OUTPUT_DIR)
                    .to_string();
            check_dir(&output_dir)?;

            Ok(Settings{
                planet_name,
                model_size,
                jobs,
                data_source,
                data_source_dir,
                output_dir,
                nodata,
                sea_level,
                specific: (&y0, &y1),
            })
        };

        match &tl_commands.inner_enum {
            SubCommandX3DGeospatial(args) =>
                make(args, "X3DGeospatial"),
            SubCommandObj(args) =>
                make(args, "Obj"),
        }
    }

    /// Returns parameter value as Yaml struct
    fn get_parameter_value(&'a self, parameter: &str) -> Result<&'a Yaml, ErrHandle> {
        let (y0, y1) = self.specific;
        if y1[parameter].is_badvalue() {
            if y0[parameter].is_badvalue() {
                return Err(format!("Parameter '{}' can't be found in settings file", parameter).into())
            } else {
                return Ok(&y0[parameter])
            }
        } else {
            return Ok(&y1[parameter])
        }
    }

    /// Returns parameter value
    pub fn get_parameter<T: FromStr>(&'a self, parameter: &str, default: T) -> Result<T, ErrHandle> {
        self.get_parameter_value(parameter)
        .map_or_else(
            |_| {Some(default)},
            |y| {
                // y.as_str()
                // .unwrap_or("")
                // .parse::<&T>()
                // .ok()
                y.as_str()
                .unwrap_or("")
                .parse::<T>()
                .ok()
            }
            )
        .ok_or_else(|| {format!("invalid '{}' parameter in the settings file", parameter).into()})
    }

    // /// Returns string parameter value
    // pub fn get_parameter_string(&'a self, parameter: &str, default: &'a str) -> Result<&'a str, ErrHandle> {
    //     self.get_parameter_value(parameter)
    //     .map_or_else(
    //         |_| {Some(default)},
    //         |y| {y.as_str()},
    //         )
    //     .ok_or_else(|| {format!("invalid '{}' parameter in the settings file", parameter).into()})
    // }

    // /// Returns i64 parameter value
    // pub fn get_parameter_i64(&self, parameter: &str, default: i64) -> Result<i64, ErrHandle> {
    //     self.get_parameter_value(parameter)
    //     .map_or_else(
    //         |_| {Some(default)},
    //         |y| {y.as_i64()},
    //         )
    //     .ok_or_else(|| {format!("invalid '{}' parameter in the settings file", parameter).into()})
    // }

    // /// Returns f64 parameter value
    // pub fn get_parameter_f64(&self, parameter: &str, default: f64) -> Result<f64, ErrHandle> {
    //     self.get_parameter_value(parameter)
    //     .map_or_else(
    //         |_| {Some(default)},
    //         |y| {y.as_f64()},
    //         )
    //     .ok_or_else(|| {format!("invalid '{}' parameter in the settings file", parameter).into()})
    // }
}
