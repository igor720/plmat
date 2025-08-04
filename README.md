# plmat
Planet Materializer. A tool for creating 3d planet models using geospatial elevation data

## Command Line
Usage: plmat <model_format> <model_type> <data_source> [--planet-name <planet-name>] [--model-size <model-size>] [--jobs <jobs>] [--data-source-dir <data-source-dir>] [--output-dir <output-dir>]

Positional Arguments:

    model_format      x3dgeospatial or obj  
    model_type        texture or color  
    data_source       DemArcSec3  

Options:

    --planet-name     planet name (will be used in output file names)  
    --model-size      model size (may be implicitly changed to the nearest valid value)  
    --jobs            number of thread jobs (default: min(2, available parallelism))  
    --data-source-dir data source directory (default: current directory)  
    --output-dir      output directory (default: current directory)  
    --help, help      display usage information

## Settings file
It's in a YAML format with two root sections: **DataSource** and **Model**.
The **Model** section include settings for two current model formats: **Obj** and **X3DGeospatial**.  
The **Common** section applies to both **Color** and **Texture** model types.

When launching the app, file settings.yaml must be in the current directory. Command line arguments take precedence over options in settings.yaml.

## Building and running

To go with 'dev' profile

    cargo build
    cargo test
    cargo run -- obj color DemArcSec3 --data-source-dir ../hgts --model-size 64

or to install with 'release' profile into ~/.cargo/bin/

    cargo install --path ./
    plmat obj color DemArcSec3 --data-source-dir ../hgts --model-size 64

Here command line arguments are given for example only. 

## Help

[Manual on ForgedMaps](https://forgedmaps.com/planet-materializer-manual).


