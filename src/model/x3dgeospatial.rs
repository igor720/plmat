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


const MIN_VALID_MODEL_SIZE: usize = 4;
const DEFAULT_TEMPLATE_FILE: &str = "./geospatial.x3d.template";
const DEFAULT_TEXTURE_URI: &str = "\"texture.png\"";


/// Model struct
pub struct X3DGeospatial<'a> {
    settings:           &'a Settings<'a>,
    model_size:         GeoPointIndex,
    spacing:            Coord,
    heights:            Heights,
    model_type_data:    ModelTypeData,
    template_file:      &'a str,
}

impl<'a> Model<'a> for X3DGeospatial<'a> {
    /// Define valid model size
    fn make_valid_model_size(model_size: Option<GeoPointIndex>) -> GeoPointIndex {
        let _model_size = model_size.unwrap_or(MIN_VALID_MODEL_SIZE);
        if _model_size<MIN_VALID_MODEL_SIZE {MIN_VALID_MODEL_SIZE} else {_model_size}
    }

    /// Define spacing parameter
    fn define_spacing(model_size: GeoPointIndex) -> Coord {
        180.0/(model_size as Coord)
    }

    /// Creates all geopoints data
    fn create_modelpoints(model_size: GeoPointIndex, spacing: Coord) -> (ModelPoints, Elements) {
        // let mut geopoints: GeoPoints = HashMap::with_capacity(2*(model_size+1)*(model_size+1));
        let mut geopoints: GeoPoints = BTreeMap::new();

        let model_size2 = 2*model_size as GeoPointIndex;
        let mut count: GeoPointIndex = 0;
        for j in 0..=model_size {
            for i in 0..=model_size2 {
                geopoints.insert(count, GeoPoint {
                    lon: (-180.0) + spacing*(if i==model_size2 {0} else {i} as Coord),
                    lat: (-90.0)  + spacing*(if j==model_size {0} else {j} as Coord)
                });
                count+=1;
            }
        }

        (ModelPoints {geopoints, points_map_opt: None}, vec!())
    }

    /// Checks files and directories
    fn options_check(settings: &'a Settings) -> Result<(), String> {
        let template_file =
                settings.get_parameter_string("template_file_x3d", DEFAULT_TEMPLATE_FILE)?;
        check_file(template_file)
    }

    /// Texture model constructor
    fn build_texture_model(
        settings:           &'a Settings,
        model_size:         GeoPointIndex,
        spacing:            Coord,
        heights:            Heights,
        _:                  ModelPoints,
        _:                  Elements,
        model_type_data:    ModelTypeData) -> Result<Self, String> where Self:Sized {

        let template_file =
                settings.get_parameter_string("template_file_x3d", DEFAULT_TEMPLATE_FILE)?;

        return Ok(X3DGeospatial{
            settings,
            model_size,
            spacing,
            heights,
            model_type_data,
            template_file,
        })
    }

    /// Color model constructor
    fn build_color_model(
        settings:           &'a Settings,
        model_size:         GeoPointIndex,
        spacing:            Coord,
        heights:            Heights,
        _:                  ModelPoints,
        _:                  Elements,
        model_type_data:    ModelTypeData) -> Result<Self, String> where Self:Sized {

        let template_file =
                settings.get_parameter_string("template_file_x3d", DEFAULT_TEMPLATE_FILE)?;
        check_file(template_file)?;

        return Ok(X3DGeospatial{
            settings,
            model_size,
            spacing,
            heights,
            model_type_data,
            template_file,
        })
    }

    /// Saves model data to resulting files
    fn save(&self) -> Result<(), String> {
        let settings = self.settings;
        let planet_name = settings.planet_name;
        let output_path = settings.output_dir;

        let mut reader = match Reader::from_file(self.template_file) {
            Ok(r) => r,
            Err(err) => return Err(format!("Can't read template: {}", err))
        };
        reader.config_mut().check_comments = true;

        let mut buf = Vec::new();

        let _result_path = Path::new(&output_path)
                .join(&planet_name)
                .with_extension("x3d");
        let result_path = match _result_path.to_str() {
            Some(fp) => fp,
            None => return Err(format!("Can't get file path for result data"))
        };

        let buffer = match File::create(result_path) {
            Ok(f) => f,
            Err(err) => return Err(format!("Can't write to output file: {}", err))
        };

        let height_values =
            self.heights
            .values().map(|v| {v.to_string()})
            .collect::<Vec<String>>()
            .join(" ");

        let color_values = match &self.model_type_data {
            ModelTypeData::Texture(_) => "".to_string(),
            ModelTypeData::Color(colors) =>
                colors
                    .values()
                    .map(|v| {v.to_string()})
                    .collect::<Vec<String>>()
                    .join(" ")
        };

        let create_height_attr = |elem: &mut BytesStart<'static>| {
                elem.push_attribute(("xDimension", (2*(self.model_size)+1).to_string().as_str()));
                elem.push_attribute(("xSpacing", (self.spacing.to_string().as_str())));
                elem.push_attribute(("zDimension", ((self.model_size)+1).to_string().as_str()));
                elem.push_attribute(("zSpacing", (self.spacing.to_string().as_str())));
                elem.push_attribute(("height", height_values.as_str()));
        };

        let mut writer = Writer::new(buffer);
        let mut in_geo_elevation_grid = false;
        loop {
            match reader.read_event_into(&mut buf) {
                Err(e) => return Err(format!("Error at position {}: {:?}", reader.error_position(), e)),
                Ok(Event::Eof) => break,
                Ok(Event::Empty(e))
                        if e.name().as_ref() == b"_GeoElevationGrid" => {
                    // println!("** {:?}", e);
                    // let mut elem = e.into_owned();
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

                        match &self.model_type_data {
                            ModelTypeData::Texture(_) => (),
                            ModelTypeData::Color(_) => {
                                elem.extend_attributes(e.attributes().map(|attr| attr.unwrap()));
                                elem.push_attribute(("color", color_values.as_str()))
                            }
                        };

                        assert!(writer.write_event(Event::Empty(elem)).is_ok());
                    },
                Ok(Event::Empty(e))
                    if e.name().as_ref() == b"_ImageTexture" => {
                        let texture_uri =
                                settings.get_parameter_string("texture_uri", DEFAULT_TEXTURE_URI)?;
                        let mut elem = BytesStart::new("ImageTexture");
                        match &self.model_type_data {
                            ModelTypeData::Texture(_) =>
                                elem.push_attribute(("url", texture_uri)),
                            ModelTypeData::Color(_) => (),
                        };

                        assert!(writer.write_event(Event::Empty(elem)).is_ok());
                    },
                Ok(e) => assert!(writer.write_event(e).is_ok()),
            }
            buf.clear();
        }

        writer.into_inner().flush().map_err(|err| {err.to_string()})
    }

}


#[cfg(test)]
mod tests {
}





