//! # planet-materializer tool
//!
//! The program for 3d models generation (for web, games, and printing).
//! Uses geospatial data as input.

mod common;
mod model;
mod input;

// use std::time::Instant;
use common::args::*;
use MySubCommandEnum::*;
use common::settings::*;
use common::types::*;
use model::types::Model;
use model::x3dgeospatial::X3DGeospatial;
use model::obj::Obj;


/// Does everything that is needed to make a 3d model
fn materialize(tl_commands: &TopLevelCommands) -> Result<(), ErrBox> {
    let settings_yaml = Settings::get_settings_yaml("./settings.yaml")?;
    let settings = Settings::make_settings(&tl_commands, &settings_yaml)?;

    match &tl_commands.inner_enum {
        SubCommandX3DGeospatial(args) =>
            Ok(X3DGeospatial::create(args.model_type, &settings)?.save()?),
        SubCommandObj(args) =>
            Ok(Obj::create(args.model_type, &settings)?.save()?),
    }
}

fn main() -> Result<(), ErrBox> {
    // let now = Instant::now();

    materialize(&argh::from_env())
        .map_err(|err| format!("Error: {}", err))?;

    // let elapsed = now.elapsed();
    // println!("%%%% Elapsed: {:.2?}", elapsed);
    Ok(())
}

