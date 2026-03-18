//! # planet-materializer tool
//!
//! The program for 3D models generation (for web, games, and printing).
//! Uses geospatial data as input.

mod common;
mod model;
mod input;

use common::types::ErrBox;
use common::args::*;
use common::settings::Settings;
use model::types::Model;
use model::x3dgeospatial::X3DGeospatial;
use model::obj::Obj;

const HARDCODED_CONFIG_FILE: &str = "./settings.yaml";

/// Does everything that is needed to make a 3D model
///
/// This function orchestrates the creation of a 3D model based on the provided 
/// command-line arguments (`tl_commands`). It reads settings from a YAML configuration file 
/// and initializes the appropriate model type (either `X3DGeospatial` or `Obj`) using these settings. 
/// The created model is then saved to disk.
///
/// ## Arguments
/// - `tl_commands`: A reference to an instance of `TopLevelCommands`, 
/// which contains all command-line arguments parsed from the environment. 
/// This includes information about the desired model type and any other relevant parameters.

fn materialize(tl_commands: &TopLevelCommands) -> Result<(), ErrBox> {
    let settings_yaml = Settings::get_settings_yaml(HARDCODED_CONFIG_FILE)?;
    let settings = Settings::make_settings(&tl_commands, &settings_yaml)?;

    match &tl_commands.inner_enum {
        MySubCommandEnum::SubCommandX3DGeospatial(args) =>
            Ok(X3DGeospatial::create(args.model_type, &settings)?.save()?),
        MySubCommandEnum::SubCommandObj(args) =>
            Ok(Obj::create(args.model_type, &settings)?.save()?),
    }
}

fn main() -> Result<(), ErrBox> {
    materialize(&argh::from_env())
        .map_err(|err| format!("Error: {}", err))?;

    Ok(())
}
