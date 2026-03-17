//! Module for handling application settings and configuration
//! 
//! This module provides functionality for reading and parsing YAML configuration files,
//! as well as managing application settings for different model types and data sources.
#[allow(dead_code)]

use std::fs::read_to_string;
use yaml_rust2::{Yaml, YamlLoader};
use num_traits::FromPrimitive;
use std::path::Path;
use crate::common::types::*;
use crate::common::util::*;
use crate::common::args::*;
use crate::common::args::MySubCommandEnum::*;


const DEFAULT_DATA_SOURCE_DIR: &str = "./";
const DEFAULT_OUTPUT_DIR: &str = "./";


/// Main settings structure for the application
/// 
/// This structure holds all the configuration parameters needed for processing
/// geographic data and generating 3D models. It includes both global settings
/// and model-specific configurations.
#[derive(Debug)]
pub struct Settings<'a> {
    /// Name of the planet being processed
    pub planet_name: String,
    /// Size of the model in terms of geopoints
    pub model_size: Option<GeoPointIndex>,
    /// Number of parallel jobs to run during processing
    pub jobs: usize,
    /// Type of data source being used
    pub data_source: DataSourceName,
    /// Directory path where source data tiles are located
    pub data_source_dir: &'a Path,
    /// Directory path where output files will be written
    pub output_dir: &'a Path,
    /// Value representing no data in the elevation data
    pub nodata: Option<HeightInt>,
    /// Default sea level for the model
    pub sea_level: Option<HeightInt>,
    /// Common settings for the model
    pub common: &'a Yaml,
    /// Specific settings for the model
    pub specific: &'a Yaml,
}

impl<'a> Settings<'a> {
    /// Reads the settings file from the specified filepath
    fn read_settings_file(filepath: &str) -> Result<String, ErrBox> {
        Ok(read_to_string(filepath)?)
    }

    /// Reads and parses the settings YAML file
    pub fn get_settings_yaml (filepath: &str) -> Result<Yaml, ErrBox> {
        match Settings::read_settings_file(filepath) {
            Ok(s) => {
                match YamlLoader::load_from_str(&s) {
                    Ok(docs) => Ok(docs[0].clone()),
                    Err(err) => Err(format!("Malformed YAML settings file: {}", err).into()),
                }
            },
            Err(err) =>
                Err(format!("Can't read YAML settings file: {}", err).into())
        }
    }

    /// Creates a Settings instance from command line arguments and configuration YAML
    pub fn make_settings (tl_commands: &'a TopLevelCommands, settings: &'a Yaml) -> Result<Self, ErrBox> {
        let make_for_model_name = |args: &'a dyn Args, model_name: &str| {
            let data_source = args.data_source();
            let planet_name = args.planet_name().clone();
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

            let data_source_dir = Path::new(
                    args.data_source_dir()
                    .map(|s| {s.as_str()})
                    .or_else(|| {y_ds["data_source_path"].as_str()})
                    .unwrap_or(DEFAULT_DATA_SOURCE_DIR)
                );
            check_dir(&data_source_dir)?;

            let y0 = &settings["Model"][model_name]["Common"];
            if y0.is_badvalue() {
                return Err(format!("Common section for '{}' is missed in settings file", model_name).into())
            };

            let y1 = match &args.model_type() {
                ModelType::Texture => &settings["Model"][model_name]["Texture"],
                ModelType::Color => &settings["Model"][model_name]["Color"],
            };
            if y1.is_badvalue() {
                return Err(format!("The model type section for '{}' is missed in settings file", model_name).into())
            };

            let output_dir = Path::new(
                    args.output_dir()
                    .map(|s| {s.as_str()})
                    .or_else(|| {
                        if y1["output_dir"].is_badvalue() {
                            if y0["output_dir"].is_badvalue() {None} else {y0["output_dir"].as_str()}
                        } else {
                            y1["output_dir"].as_str()
                        }
                    })
                    .unwrap_or(DEFAULT_OUTPUT_DIR)
                );
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
                common: &y0,
                specific: &y1,
            })
        };

        match &tl_commands.inner_enum {
            SubCommandX3DGeospatial(args) =>
                make_for_model_name(args, "X3DGeospatial"),
            SubCommandObj(args) =>
                make_for_model_name(args, "Obj"),
        }
    }

    /// Returns parameter value as Yaml struct
    fn get_parameter_yaml(&'a self, parameter: &str) -> Result<&'a Yaml, ErrBox> {
        let y0 = self.common;
        let y1 = self.specific;
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

    /// Returns number type parameter value
    pub fn get_parameter_num<T: FromPrimitive>(&'a self, parameter: &str, default: T) -> Result<T, ErrBox> {
        let err_str = format!("Invalid '{}' parameter in the settings file", parameter);

        match self.get_parameter_yaml(parameter) {
            Ok(y) =>
                return match y {
                    Yaml::Real(_) => y.as_f64().map_or_else(
                       || { Err(err_str.into()) },
                       |a| {<T>::from_f64(a).ok_or_else(|| {"".into()})}),
                    Yaml::Integer(i) => <T>::from_i64(*i).ok_or_else(|| {err_str.into()}),
                    _ => Err(format!("'{}' parameter must have numeric type in the settings file", parameter).into()),
                },
            Err(_) => Ok(default),
        }
    }

    /// Returns string type parameter value
    pub fn get_parameter_str(&'a self, parameter: &str, default: &str) -> Result<String, ErrBox> {
        match self.get_parameter_yaml(parameter) {
            Ok(y) =>
                return match y {
                    Yaml::String(s) => Ok(s.to_string()),
                    _ => Err(format!("'{}' parameter must have string type in the settings file", parameter).into()),
                },
            Err(_) => Ok(default.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_read_settings_file() {
        let filepath = "tests/fixtures/valid_settings.yaml";
        assert!(Settings::read_settings_file(filepath).is_ok());
    }

    #[test]
    fn test_get_settings_yaml() {
        let filepath = "tests/fixtures/valid_settings.yaml";
        assert!(Settings::get_settings_yaml(filepath).is_ok());
    }

    #[test]
    fn test_make_settings() {
        // Assuming you have a valid settings.yaml file and top-level commands setup for testing
        let filepath = "tests/fixtures/valid_settings.yaml";
        let content = fs::read_to_string(filepath).expect("Failed to read file");
        let yaml = YamlLoader::load_from_str(&content).unwrap()[0].clone();
        // You need to set up a TopLevelCommands and Yaml for this test
        let args = CLIArgsX3DGeospatial {
            model_type: ModelType::Texture,
            data_source: DataSourceName::DemArcSec3,
            planet_name: "test-planet".to_string(),
            model_size: Some(8),
            jobs: 2,
            data_source_dir: Some("./".to_string()),
            output_dir: Some("./".to_string()),
        };
        let tl_command = TopLevelCommands { inner_enum: SubCommandX3DGeospatial(args) };
        assert!(Settings::make_settings(&tl_command, &yaml).is_ok());
    }

    #[test]
    fn test_get_parameter() {
        let filepath = "tests/fixtures/valid_settings.yaml";
        let content = fs::read_to_string(filepath).expect("Failed to read file");
        let yaml = YamlLoader::load_from_str(&content).unwrap();
        let args = CLIArgsObj {
            model_type: ModelType::Color,
            data_source: DataSourceName::DemArcSec3,
            jobs: 4,
            data_source_dir: Some("./".to_string()),
            output_dir: None,
            planet_name: "some name".to_string(),
            model_size: None,
        };
        let tl_command = TopLevelCommands {
            inner_enum: MySubCommandEnum::SubCommandObj(args)
        };
        let settings = Settings::make_settings(&tl_command, &yaml[0]).unwrap();
        assert_eq!(settings.get_parameter_num("jobs", 4).unwrap(), 4);
        assert_eq!(
            settings.get_parameter_str("output_dir", "***").unwrap(), 
            "./".to_string()
        );
        // from cmd args
        assert_eq!(
            settings.data_source_dir.to_str(), 
            Some("./")
        );
        // from settings file
        assert_eq!(
            settings.get_parameter_str("data_source_dir", "***").unwrap(), 
            "./some-dir".to_string()
        );
    }
    
    #[test]
    fn test_get_parameter_default() {
        let filepath = "tests/fixtures/valid_settings.yaml";
        let content = fs::read_to_string(filepath).expect("Failed to read file");
        let yaml = YamlLoader::load_from_str(&content).unwrap();
        let settings = Settings {
            planet_name: "Earth".to_string(),
            model_size: None,
            jobs: 4,
            data_source: DataSourceName::DemArcSec3,
            data_source_dir: Path::new(DEFAULT_DATA_SOURCE_DIR),
            output_dir: Path::new(DEFAULT_OUTPUT_DIR),
            nodata: None,
            sea_level: None,
            common: &yaml[0],
            specific: &yaml[0]["Model"]["X3DGeospatial"],
        };
        assert_eq!(settings.get_parameter_num("unknown_param", 123).unwrap(), 123);
        assert_eq!(settings.get_parameter_str("unknown_param", "123").unwrap(), "123".to_string());
    }
}
