use std::thread::available_parallelism;
use std::cmp::min;
use argh::FromArgs;

use crate::common::types::*;


#[derive(Debug, PartialEq, Clone)]
/// 3d model type that determines its appearance
pub enum ModelType {
    TextureModelType,
    ColorModelType,
}

/// Get model type identificator depending on specific command argument
fn get_model_type(value: &str) -> Result<ModelType, String> {
    match value {
        "texture" => Ok(ModelType::TextureModelType),
        "color" => Ok(ModelType::ColorModelType),
        _ => Err("Unknown model type".to_string())
    }
}

#[derive(Debug, PartialEq, Clone)]
/// Source data type
pub enum DataSourceName {
    DemArcSec3,
}

/// Get source data type identificator depending on specific command argument
fn get_data_source_name(value: &str) -> Result<DataSourceName, String> {
    match value {
        "DemArcSec3" => Ok(DataSourceName::DemArcSec3),
        _ => Err("Unknown data source".to_string())

    }
}

/// Default planet name
fn default_planet_name() -> String {
    "Unknown_planet".to_string()
}

/// Default number of worker threads
fn default_jobs() -> usize {
    let parallelism = available_parallelism().unwrap().get();
    min(2, parallelism)
}

/// Top-level commands
#[derive(FromArgs, PartialEq, Debug)]
pub struct TopLevelCommands {
    #[argh(subcommand)]
    pub inner_enum: MySubCommandEnum,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
pub enum MySubCommandEnum {
    SubCommandX3DGeospatial(CLIArgsX3DGeospatial),
    SubCommandObj(CLIArgsObj),
}

/// Common arguments getters trait
pub trait Args {
    fn data_source(&self) -> DataSourceName;
    fn model_type(&self) -> ModelType;
    fn planet_name(&self) -> &String;
    fn model_size(&self) -> Option<GeoPointIndex>;
    fn jobs(&self) -> usize;
    fn data_source_dir(&self) -> Option<&String>;
    fn output_dir(&self) -> Option<&String>;
}

/// Subcommand for X3DGeospatial mode
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "x3dgeospatial")]
pub struct CLIArgsX3DGeospatial {
    /// model type (Texture or Color)
    #[argh(positional, from_str_fn(get_model_type))]
    pub model_type: ModelType,

    /// data source type
    #[argh(positional, from_str_fn(get_data_source_name))]
    pub data_source: DataSourceName,

    /// planet name (will be used in output file names)
    #[argh(option, default = "default_planet_name()")]
    pub planet_name: String,

    /// model size (may be implicitly changed to the nearest valid value)
    #[argh(option)]
    pub model_size: Option<GeoPointIndex>,

    /// number of jobs (default: min(2, available parallelism))
    #[argh(option, default = "default_jobs()")]
    pub jobs: usize,

    /// data source path (default: current directory)
    #[argh(option)]
    pub data_source_dir: Option<String>,

    /// output path (default: current directory)
    #[argh(option)]
    pub output_dir: Option<String>,
}

impl Args for CLIArgsX3DGeospatial {
    fn data_source(&self) -> DataSourceName {
        self.data_source.clone()
    }
    fn model_type(&self) -> ModelType {
        self.model_type.clone()
    }
    fn planet_name(&self) -> &String {
        &self.planet_name
    }
    fn model_size(&self) -> Option<GeoPointIndex> {
        self.model_size
    }
    fn jobs(&self) -> usize {
        self.jobs
    }
    fn data_source_dir(&self) -> Option<&String> {
        self.data_source_dir.as_ref()
    }
    fn output_dir(&self) -> Option<&String> {
        self.output_dir.as_ref()
    }
}

/// Subcommand for color mode
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "obj")]
pub struct CLIArgsObj {
    /// model type (Texture or Color)
    #[argh(positional, from_str_fn(get_model_type))]
    pub model_type: ModelType,

    /// data source type
    #[argh(positional, from_str_fn(get_data_source_name))]
    pub data_source: DataSourceName,

    /// planet name (will be used in output file names)
    #[argh(option, default = "default_planet_name()")]
    pub planet_name: String,

    /// model size (may be implicitly changed to the nearest valid value)
    #[argh(option)]
    pub model_size: Option<GeoPointIndex>,

    /// number of jobs (default: min(2, available parallelism))
    #[argh(option, default = "default_jobs()")]
    pub jobs: usize,

    /// data source path (default: current directory)
    #[argh(option)]
    pub data_source_dir: Option<String>,

    /// output path (default: current directory)
    #[argh(option)]
    pub output_dir: Option<String>,
}

impl Args for CLIArgsObj {
    fn data_source(&self) -> DataSourceName {
        self.data_source.clone()
    }
    fn model_type(&self) -> ModelType {
        self.model_type.clone()
    }
    fn planet_name(&self) -> &String {
        &self.planet_name
    }
    fn model_size(&self) -> Option<GeoPointIndex> {
        self.model_size
    }
    fn jobs(&self) -> usize {
        self.jobs
    }
    fn data_source_dir(&self) -> Option<&String> {
        self.data_source_dir.as_ref()
    }
    fn output_dir(&self) -> Option<&String> {
        self.output_dir.as_ref()
    }
}

