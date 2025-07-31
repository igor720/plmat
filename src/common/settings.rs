use std::fs::read_to_string;
use yaml_rust2::{Yaml, YamlLoader};

use crate::common::types::*;
use crate::common::util::*;
use crate::common::args::*;
use crate::common::args::MySubCommandEnum::*;


const DEFAULT_DATA_SOURCE_DIR: &str = "./";
const DEFAULT_OUTPUT_DIR: &str = "./";


/// Reads the settings file
fn read_settings_file(filepath: &str) -> Result<String, String> {
    read_to_string(filepath)
        .map_err(|err| {err.to_string()})
}

/// Reads the color profile file
pub fn get_settings_yaml (filepath: &str) -> Result<Yaml, String> {
    match read_settings_file(filepath) {
        Ok(s) => {
            let docs = YamlLoader::load_from_str(&s).unwrap();
            let doc = &docs[0];
            Ok(doc.clone())
        },
        Err(err) =>
            Err(format!("Can't load settings.yaml from the current directory: {}", err))
    }
}

/// Main settings structure
pub struct Settings<'a> {
    /// planet name
    pub planet_name: &'a str,
    /// model size
    pub model_size: Option<GeoPointIndex>,
    /// number of jobs
    pub jobs: usize,
    /// source data type
    pub data_source: DataSourceName,
    /// path to data tiles
    pub data_source_dir: &'a str,
    /// output_path
    pub output_dir: &'a str,
    /// nodata value
    pub nodata: Option<HeightInt>,
    /// default sea level
    pub sea_level: Option<HeightInt>,
    /// specific settings
    pub specific: (&'a Yaml, &'a Yaml)
}

impl<'a> Settings<'a> {
    pub fn make_settings (tl_commands: &'a TopLevelCommands, settings: &'a Yaml) -> Result<Self, String> {
        let make = |args: &'a dyn Args, model_name: &str| {
            let data_source = args.data_source();
            let planet_name = args.planet_name().as_str();
            let model_size = args.model_size();
            let jobs = args.jobs();

            let y_ds = match data_source {
                DataSourceName::DemArcSec3 => &settings["DataSource"]["DemArcSec3"],
            };
            if y_ds.is_badvalue() {
                return Err(format!("Common section for '{}' is missed in settings file", model_name))
            };

            let nodata = y_ds["data_source_path"].as_i64().map(|i| {i as HeightInt});
            let sea_level = y_ds["sea_level"].as_i64().map(|i| {i as HeightInt});

            let data_source_dir = args.data_source_dir()
                    .map(|s| {s.as_str()})
                    .or_else(|| {y_ds["data_source_path"].as_str()})
                    .unwrap_or(DEFAULT_DATA_SOURCE_DIR);
            check_dir(data_source_dir)?;

            let y0 = &settings["Model"][model_name]["Common"];
            if y0.is_badvalue() {
                return Err(format!("Common section for '{}' is missed in settings file", model_name))
            };

            let y1 = match &args.model_type() {
                ModelType::TextureModelType => &settings["Model"][model_name]["Texture"],
                ModelType::ColorModelType => &settings["Model"][model_name]["Color"],
            };
            if y1.is_badvalue() {
                return Err(format!("The model type section for '{}' is missed in settings file", model_name))
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
                    .unwrap_or(DEFAULT_OUTPUT_DIR);
            check_dir(output_dir)?;

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

// Returns parameter value as Yaml struct
fn get_parameter_value(&'a self, parameter: &str) -> Result<&'a Yaml, String> {
    let (y0, y1) = self.specific;
    if y1[parameter].is_badvalue() {
        if y0[parameter].is_badvalue() {
            return Err(format!("Parameter '{}' can't be found in settings file", parameter))
        } else {
            return Ok(&y0[parameter])
        }
    } else {
        return Ok(&y1[parameter])
    }
}

// Returns string parameter value
pub fn get_parameter_string(&'a self, parameter: &str, default: &'a str) -> Result<&'a str, String> {
    self.get_parameter_value(parameter)
    .map_or_else(
        |_| {Some(default)},
        |y| {y.as_str()}
        )
    .ok_or_else(|| {format!("invalid '{}' parameter in the settings file", parameter)})
}

#[allow(dead_code)]
// Returns i64 parameter value
pub fn get_parameter_i64(&self, parameter: &str, default: i64) -> Result<i64, String> {
    self.get_parameter_value(parameter)
    .map_or_else(
        |_| {Some(default)},
        |y| {y.as_i64()}
        )
    .ok_or_else(|| {format!("invalid '{}' parameter in the settings file", parameter)})
}

// Returns i64 parameter value
pub fn get_parameter_f64(&self, parameter: &str, default: f64) -> Result<f64, String> {
    self.get_parameter_value(parameter)
    .map_or_else(
        |_| {Some(default)},
        |y| {y.as_f64()}
        )
    .ok_or_else(|| {format!("invalid '{}' parameter in the settings file", parameter)})
}

}


