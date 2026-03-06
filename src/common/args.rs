//! This module contains the definitions for command line argument parsing 
//! and related types used throughout the application. 
//! It includes enums, functions, traits, and subcommands tailored for different modes of operation 
//! such as `X3DGeospatial` and `Obj`.
use std::thread::available_parallelism;
use std::cmp::min;
use argh::FromArgs;

use crate::common::types::*;

/// Enum representing different types of 3D model
#[derive(Debug, PartialEq, Clone)]
/// 3d model type that determines its appearance
pub enum ModelType {
    TextureModelType,
    ColorModelType,
}

/// Get the model type based on a string value.
/// # Arguments
/// * `value` - A string slice that represents the model type.
/// # Returns
/// Result containing either the ModelType or an error message if the type is unknown.
fn get_model_type(value: &str) -> Result<ModelType, String> {
    match value {
        "texture" => Ok(ModelType::TextureModelType),
        "color" => Ok(ModelType::ColorModelType),
        _ => Err("Unknown model type".to_string())
    }
}

/// Enum representing different data source types.
#[derive(Debug, PartialEq, Clone)]
pub enum DataSourceName {
    /// Represents the DEM (Digital Elevation Model) ArcSec3 data source.
    DemArcSec3,
}

/// Get the data source name based on a string value.
/// # Arguments
/// * `value` - A string slice that represents the data source name.
/// # Returns
/// Result containing either the DataSourceName or an error message if the type is unknown.
fn get_data_source_name(value: &str) -> Result<DataSourceName, String> {
    match value {
        "DemArcSec3" => Ok(DataSourceName::DemArcSec3),
        _ => Err("Unknown data source".to_string())
    }
}

/// Default planet name.
fn default_planet_name() -> String {
    "Unnamed".to_string()
}

/// Default number of worker threads.
fn default_jobs() -> usize {
    let parallelism = available_parallelism().unwrap().get();
    min(2, parallelism)
}

/// Top-level commands structure that can be parsed from command line arguments.
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

/// Common arguments getter trait.
pub trait Args {
    fn data_source(&self) -> DataSourceName;
    fn model_type(&self) -> ModelType;
    fn planet_name(&self) -> &String;
    fn model_size(&self) -> Option<GeoPointIndex>;
    fn jobs(&self) -> usize;
    fn data_source_dir(&self) -> Option<&String>;
    fn output_dir(&self) -> Option<&String>;
}

/// Subcommand for X3DGeospatial mode.
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "x3dgeospatial")]
pub struct CLIArgsX3DGeospatial {
    /// model type (Texture or Color)
    #[argh(positional, from_str_fn(get_model_type))]
    pub model_type: ModelType,

    /// data source type
    #[argh(positional, from_str_fn(get_data_source_name))]
    pub data_source: DataSourceName,

    /// planet name (will be used in output file names).
    #[argh(option, default = "default_planet_name()")]
    pub planet_name: String,

    /// model size (may be implicitly changed to the nearest valid value)
    #[argh(option)]
    pub model_size: Option<GeoPointIndex>,

    /// number of jobs (default: min(2, available parallelism))
    #[argh(option, default = "default_jobs()")]
    pub jobs: usize,

    /// data source directory (default: current directory)
    #[argh(option)]
    pub data_source_dir: Option<String>,

    /// output directory (default: current directory)
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

/// Subcommand for color mode.
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "obj")]
pub struct CLIArgsObj {
    /// model type (Texture or Color).
    #[argh(positional, from_str_fn(get_model_type))]
    pub model_type: ModelType,

    /// data source type.
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

    /// data source directory (default: current directory)
    #[argh(option)]
    pub data_source_dir: Option<String>,

    /// output directory (default: current directory).
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
