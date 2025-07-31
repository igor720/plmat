//! # planet-materializer tool
//!
//! The program for 3d models generation (for web, games, and printing).
//! Uses geospatial data as input.

// use std::time::Instant;

mod common;
mod model;
mod input;

use common::args::*;
use common::settings::*;
use model::types::*;
use model::x3dgeospatial::*;
use model::obj::*;
use crate::MySubCommandEnum::*;


/// Does everything that is needed to make a 3d model
fn materialize(tl_commands: &TopLevelCommands) -> Result<(), String> {
    let settings_yaml = get_settings_yaml("./settings.yaml")?;
    let settings = Settings::make_settings(&tl_commands, &settings_yaml)?;

    match &tl_commands.inner_enum {
        SubCommandX3DGeospatial(args) => {
            Ok(match &args.model_type {
                ModelType::TextureModelType => {
                    X3DGeospatial::create_with_texture(&settings)?.save()?
                },
                ModelType::ColorModelType => {
                    X3DGeospatial::create_with_color(&settings)?.save()?
                },
            })
        }
        SubCommandObj(args) => {
            Ok(match &args.model_type {
                ModelType::TextureModelType => {
                    Obj::create_with_texture(&settings)?.save()?
                }
                ModelType::ColorModelType => {
                    Obj::create_with_color(&settings)?.save()?
                }
            })
        }
    }
}

fn main() {
    // let now = Instant::now();

    materialize(&argh::from_env())
        .inspect_err(|err| eprintln!("Error: {err}"))
        .unwrap_or(());

    // let elapsed = now.elapsed();
    // println!("%%%% Elapsed: {:.2?}", elapsed);
}

