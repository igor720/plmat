use std::fs::File;
use std::path::Path;
use std::io::{BufReader, Read};
use std::io::{BufWriter, Write};
use std::collections::{BTreeMap, HashMap};

use crate::common::settings::*;
use crate::common::types::*;
use crate::common::util::check_file;
use crate::common::util::calc_point3d;
use crate::common::color::*;
use crate::model::types::*;


const MIN_VALID_MODEL_SIZE: usize = 2;
const DEFAULT_MODEL_SIZE: usize = 16;
const DEFAULT_TEMPLATE_FILE_OBJ: &str = "./obj.template";
const DEFAULT_TEMPLATE_FILE_MTL: &str = "./mtl.template";
const DEFAULT_RADIUS: f64 = 6378000.0;
const DEFAULT_SCALE: f64 = 1.0;
const DEFAULT_COLOR_PRECISION: i64 = 0;
const FRACTION_LENGHT: usize = 5;
const WRITER_BUF_STRINGS: usize = 1000;


/// Model struct
pub struct Obj<'a> {
    settings:           &'a Settings<'a>,
    heights:            Heights,
    modelpoints:        ModelPoints,
    elements:           Elements,
    model_type_data:    ModelTypeData,
    template_file_mtl:  &'a str,
    template_file_obj:  &'a str,
    scale:              Height,
    radius:             Height,
    color_precision:    ColorPrecision,
}

impl<'a> Model<'a> for Obj<'a> {
    /// Define valid model size
    fn make_valid_model_size(model_size: Option<GeoPointIndex>) -> GeoPointIndex {
        let _model_size =model_size.unwrap_or(DEFAULT_MODEL_SIZE);
        if _model_size<MIN_VALID_MODEL_SIZE {
            MIN_VALID_MODEL_SIZE
        } else {
            if 2*(_model_size/2)==_model_size {
                _model_size
            } else {
                _model_size-1
            }
        }
    }

    /// Define spacing parameter
    fn define_spacing(model_size: GeoPointIndex) -> Coord {
        180.0/(model_size as Coord)
    }

    /// Creates all geopoints data
    fn create_modelpoints(model_size: GeoPointIndex, j_spacing: Coord) -> (ModelPoints, Elements) {
        let gnn = model_size/2 as GeoPointIndex;
        let mut geopoints: GeoPoints = BTreeMap::new();
        let mut texture_points: PointsMapping = HashMap::new();
        let mut elements: Elements = Vec::with_capacity(4*(gnn as usize)*(gnn as usize)+2*(gnn as usize)+1);

        let mut point_index_r: GeoPointIndex = 0;
        let mut point_index_t: GeoPointIndex = 0;

        // equator points
        for i in 0..4*gnn {
            geopoints.insert(point_index_r, GeoPoint {
                lon: -180.0 + j_spacing*i as Coord,
                lat: 0.0
            });
            texture_points.insert(point_index_t, point_index_r);
            point_index_r += 1;
            point_index_t += 1;
        }
        texture_points.insert(point_index_t, 0);
        point_index_t += 1;

        // north and south hemispheres points
        for j in 1..gnn {
            let start_point_index_n = point_index_r;
            let start_point_index_s = point_index_r+1;
            let i_len = 4*(gnn-j);
            let i_spacing = 360.0/(i_len as Coord);
            // println!("*** j: {}, i_len: {}, i_spacing: {}", j, i_len, i_spacing);
            for i in 0..i_len {
                // println!("*** i: {}", i);
                // north: odd indices
                geopoints.insert(point_index_r, GeoPoint {
                    lon: -180.0 + i_spacing*i as Coord,
                    lat: j_spacing*j as Coord
                });
                texture_points.insert(point_index_t, point_index_r);
                point_index_r += 1;
                point_index_t += 1;
                // south: even indices
                geopoints.insert(point_index_r, GeoPoint {
                    lon: -180.0 + i_spacing*i as Coord,
                    lat: -j_spacing*j as Coord
                });
                texture_points.insert(point_index_t, point_index_r);
                point_index_r += 1;
                point_index_t += 1;
            }
            texture_points.insert(point_index_t, start_point_index_n);
            point_index_t += 1;
            texture_points.insert(point_index_t, start_point_index_s);
            point_index_t += 1;
        }

        // north and south poles points
        geopoints.insert(point_index_r, GeoPoint {
            lon: 0.0,
            lat: 90.0
        });
        texture_points.insert(point_index_t, point_index_r);
        point_index_r += 1;
        point_index_t += 1;
        geopoints.insert(point_index_r, GeoPoint {
            lon: 0.0,
            lat: -90.0
        });
        texture_points.insert(point_index_t, point_index_r);

        // equator triangles
        let mut index_low: GeoPointIndex = 0;
        let mut index_hi_n: GeoPointIndex = 4*gnn+1;
        let mut index_hi_s: GeoPointIndex = 4*gnn+2;
        let i_len = 4*gnn;
        if i_len>4 {
            for i in 0..i_len {
                index_hi_n += if i%gnn==0 {0} else {2};
                index_hi_s += if i%gnn==0 {0} else {2};
                elements.push((index_low, index_low+1, index_hi_n));
                elements.push((index_low+1, index_low, index_hi_s));
                if i%gnn!=0  {
                    elements.push((index_low, index_hi_n, index_hi_n-2));
                    elements.push((index_low, index_hi_s-2, index_hi_s));
                }
                index_low += 1;
            }
        }

        // north and south hemisphere triangles
        let mut index_low_n = index_low+1;
        let mut index_low_s = index_low+2;
        index_hi_n += 2;
        index_hi_s += 2;
        for j in 1..gnn-1 {
            let i_len = 4*(gnn-j);
            // println!("### j: {}", j);
            for i in 0..i_len {
                index_hi_n += if i%(gnn-j)==0 {0} else {2};
                index_hi_s += if i%(gnn-j)==0 {0} else {2};
                // println!("### i: {}, {:?}", i, (index_low_n, index_low_n+2, index_hi_n));
                elements.push((index_low_n,  index_low_n+2, index_hi_n));
                elements.push((index_low_s+2, index_low_s, index_hi_s));
                if j<gnn-1 && i%(gnn-j)!=0 {
                    elements.push((index_low_n, index_hi_n, index_hi_n-2));
                    elements.push((index_low_s, index_hi_s-2, index_hi_s));
                }
                index_low_n += 2;
                index_low_s += 2;
            }
            index_hi_n += 2;
            index_hi_s += 2;
            index_low_n += 2;
            index_low_s += 2;
        }

        // pole triangles
        if gnn!=1 {
            for _ in 0..4 {
                elements.push((index_low_n, index_low_n+2, index_hi_n));
                elements.push((index_low_s+2, index_low_s, index_hi_s));
                index_low_n += 2;
                index_low_s += 2;
                // index_hi_n += 2;
                // index_hi_s += 2;
            }
        } else {
            for _ in 0..4 {
                elements.push((index_low, index_low+1, index_hi_n-2));
                elements.push((index_low+1, index_low, index_hi_s-2));
                index_low += 1;
                // index_hi_n += 2;
                // index_hi_s += 2;
            }
        }

        (ModelPoints {geopoints, points_map_opt: Some(texture_points)}, elements)
    }

    /// Creates texture coordinates data
    fn create_texture_coordinates(model_size: GeoPointIndex) -> TextureCoordinates {
        let gnn = model_size/2 as GeoPointIndex;
        let mut texture_coordinates: TextureCoordinates =
                Vec::with_capacity((model_size*model_size+model_size+1) as usize);
        let mut point_index: GeoPointIndex = 0;

        // equator points
        let u_spacing_e = 1.0/((4*gnn) as TextureCoordinate);
        for i in 0..=4*gnn {
            texture_coordinates.insert(point_index, (u_spacing_e*i as TextureCoordinate, 0.5));
            point_index += 1;
        }

        // north and south hemispheres points
        let v_spacing = 1.0/((2*gnn) as TextureCoordinate);
        for j in 1..gnn {
            let i_len = 4*(gnn-j);
            let u_spacing = if i_len!=0 {1.0/(i_len as TextureCoordinate)} else {0.0};
            // println!("i_len: {}, i_spacing: {}"), ;
            for i in 0..=i_len {
                // north: odd indices
                texture_coordinates.insert(
                    point_index, (u_spacing*i as TextureCoordinate, 0.5+v_spacing*j as TextureCoordinate));
                point_index += 1;
                // south: even indices
                texture_coordinates.insert(
                    point_index, (u_spacing*i as TextureCoordinate, 0.5-v_spacing*j as TextureCoordinate));
                point_index += 1;
            }
        }

        // north and south pole points
        let u_spacing = 1.0/3.0;
        // println!("i_len: {}, i_spacing: {}"), ;
        for i in 0..=0 {
            // north: odd indices
            texture_coordinates.insert(
                point_index, (u_spacing*i as TextureCoordinate, 1.0 as TextureCoordinate));
            point_index += 1;
            // south: even indices
            texture_coordinates.insert(
                point_index, (u_spacing*i as TextureCoordinate, 0.0 as TextureCoordinate));
            point_index += 1;
        }

        texture_coordinates
    }

    /// Checks files and directories
    fn options_check(settings: &'a Settings) -> Result<(), String> {
        let template_file_obj =
                settings.get_parameter_string("template_file_obj", DEFAULT_TEMPLATE_FILE_OBJ)?;
        check_file(template_file_obj)?;
        let template_file_mtl =
                settings.get_parameter_string("template_file_mtl", DEFAULT_TEMPLATE_FILE_MTL)?;
        check_file(template_file_mtl)
    }

    /// Texture model constructor
    fn build_texture_model(
        settings:           &'a Settings,
        _:                  GeoPointIndex,
        _:                  Coord,
        heights:            Heights,
        modelpoints:        ModelPoints,
        elements:           Elements,
        model_type_data:    ModelTypeData) -> Result<Self, String> where Self:Sized {

        let template_file_obj =
                settings.get_parameter_string("template_file_obj", DEFAULT_TEMPLATE_FILE_OBJ)?;
        let template_file_mtl =
                settings.get_parameter_string("template_file_mtl", DEFAULT_TEMPLATE_FILE_MTL)?;
        let scale = settings.get_parameter_f64("scale", DEFAULT_SCALE)? as Height;
        let radius = settings.get_parameter_f64("radius", DEFAULT_RADIUS)? as Height;

        return Ok(Obj{
            settings,
            heights,
            modelpoints,
            elements,
            model_type_data,
            template_file_mtl,
            template_file_obj,
            scale,
            radius,
            color_precision: 0,
        })
    }

    /// Color model constructor
    fn build_color_model(
        settings:           &'a Settings,
        _:                  GeoPointIndex,
        _:                  Coord,
        heights:            Heights,
        modelpoints:        ModelPoints,
        elements:           Elements,
        model_type_data:    ModelTypeData) -> Result<Self, String> where Self:Sized {

        let template_file_obj =
                settings.get_parameter_string("template_file_obj", DEFAULT_TEMPLATE_FILE_OBJ)?;
        check_file(template_file_obj)?;
        let template_file_mtl =
                settings.get_parameter_string("template_file_mtl", DEFAULT_TEMPLATE_FILE_MTL)?;
        check_file(template_file_mtl)?;

        let scale = settings.get_parameter_f64("scale", DEFAULT_SCALE)? as Height;
        let radius = settings.get_parameter_f64("radius", DEFAULT_RADIUS)? as Height;
        let color_precision = settings.get_parameter_i64("color_precision", DEFAULT_COLOR_PRECISION)? as ColorPrecision;

        return Ok(Obj{
            settings,
            heights,
            modelpoints,
            elements,
            model_type_data,
            template_file_mtl,
            template_file_obj,
            scale,
            radius,
            color_precision,
        })
    }

    /// Saves model data to resulting files
    fn save(&self) -> Result<(), String> {
        let settings = self.settings;
        let planet_name = settings.planet_name;
        let output_path = settings.output_dir;

        // mtl file
        let create_mtl = || -> Result<(), String> {
            let mut data = match &self.model_type_data {
                ModelTypeData::Color(_) if self.color_precision==0 =>
                    String::with_capacity(2*22 * (self.color_precision+1) as usize * (self.color_precision+1) as usize),
                _ => String::with_capacity(2000),
                };

            let mtl_path_opt = Path::new(&output_path)
                    .join(&planet_name)
                    .with_extension("mtl");
            let mtl_path = match mtl_path_opt.to_str() {
                Some(fp) => fp,
                None => return Err(format!("Can't make mtl file with path {} and name {}", &output_path, &planet_name))
            };
            let f_mtl = File::create(&mtl_path)
                .map_err(|err| {format!("Can't create mtl file {}: {}", &mtl_path, err)})?;
            let mut f_mtl = BufWriter::new(f_mtl);

            // header
            data.clear();
            let f_tmpl = File::open(&self.template_file_mtl)
                .map_err(|err| {format!("Can't open mtl template file {}: {}", &self.template_file_mtl, err)})?;
            let mut br = BufReader::new(f_tmpl);
            br.read_to_string(&mut data)
                .map_err(|err| {format!("Can't read mtl template file {}: {}", &self.template_file_mtl, err)})?;
            if let ModelTypeData::Texture(_) = &self.model_type_data {
                data.push_str("map_Kd texture.png\n")
            }
            data.push_str("\n");
            f_mtl.write_all(data.as_bytes())
                .map_err(|err| {format!("Can't write to mtl file {}: {}", &mtl_path, err)})?;

            if let ModelTypeData::Color(_) = &self.model_type_data {
                if self.color_precision!=0 {
                    let interval = get_color_interval(self.color_precision);
                    for r_k in 0..=self.color_precision {
                        data.clear();
                        for g_k in 0..=self.color_precision {
                            for b_k in 0..=self.color_precision {
                                data.push_str(format!("newmtl c_{}_{}_{}\n", r_k, g_k, b_k).as_str());
                                let rgb = make_rgb_color(interval, r_k, g_k, b_k);
                                data.push_str(format!("Kd {}\n\n", rgb).as_str());
                            }
                        }
                        f_mtl.write_all(data.as_bytes())
                            .map_err(|err| {format!("Can't write to mtl file {}: {}", &mtl_path, err)})?;
                    }
                }
            };
            f_mtl.flush()
                .map_err(|err| {format!("Can't flush mtl file {}: {}", &mtl_path, err)})
        };

        // obj file
        let create_obj = || -> Result<(), String> {
            let mut data = match &self.model_type_data {
                ModelTypeData::Color(_) if self.color_precision==0 =>
                    String::with_capacity((3*(FRACTION_LENGHT+4+6)+1)*WRITER_BUF_STRINGS),
                ModelTypeData::Color(_) =>
                    String::with_capacity((2*3*(FRACTION_LENGHT+4)+1)*WRITER_BUF_STRINGS),
                ModelTypeData::Texture(_) =>
                    String::with_capacity((3*(FRACTION_LENGHT+4)+1)*WRITER_BUF_STRINGS),
                };

            let result_path_opt = Path::new(&output_path)
                    .join(&planet_name)
                    .with_extension("obj");
            let result_path = match result_path_opt.to_str() {
                Some(fp) => fp,
                None => return Err(format!("Can't make obj file with path {} and name {}", &output_path, &planet_name))
            };
            let f_obj = File::create(&result_path)
                .map_err(|err| {format!("Can't create obj file {}: {}", &result_path, err)})?;
            let mut f_obj = BufWriter::new(f_obj);

            // header
            data.clear();
            let f_tmpl = File::open(&self.template_file_obj)
                .map_err(|err| {format!("Can't open obj template file {}: {}", &self.template_file_obj, err)})?;
            let mut br = BufReader::new(f_tmpl);
            br.read_to_string(&mut data)
                .map_err(|err| {format!("Can't read obj template file {}: {}", &self.template_file_obj, err)})?;
            data.push_str(&format!("\nmtllib {}.mtl\n", planet_name));
            data.push_str("o Planet\n");
            f_obj.write_all(data.as_bytes())
                .map_err(|err| {format!("Can't write header to obj file {}: {}", &result_path, err)})?;

            // vertices
            data.clear();
            let gps = &self.modelpoints.geopoints;
            let mut vertex_count = 0;
            for (i, gp) in gps.iter() {
                let GeoPoint {lon, lat} = *gp;
                let height = match self.heights.get(i) {
                    None => 0.0,
                    Some(h) => *h
                };
                let (x, y, z) = calc_point3d(self.radius, self.scale, height, lon, lat);
                match &self.model_type_data {
                    ModelTypeData::Color(colors) if self.color_precision==0 => {
                        let rgb = colors.get(i).ok_or(format!("Missed color for point {}", i))?;
                        data.push_str(format!("v {:.5} {:.5} {:.5} {}\n", x, y, z, rgb).as_str())
                    },
                    _ =>
                        data.push_str(format!("v {:.5} {:.5} {:.5}\n", x, y, z).as_str()),  // XXX: FRACTION_LENGHT=5
                }
                vertex_count += 1;
                if i%WRITER_BUF_STRINGS==WRITER_BUF_STRINGS-1 {
                    f_obj.write_all(data.as_bytes())
                        .map_err(|err| {
                            format!("Can't write chunk of vertices to obj file {}: {}", &result_path, err)})?;
                    data.clear();
                }
            }

            data.push_str(format!("# {} vertices\n\n", vertex_count).as_str());
            f_obj.write_all(data.as_bytes())
                .map_err(|err| {format!("Can't write vertices to obj file {}: {}", &result_path, err)})?;

            // texture coordinates
            if let ModelTypeData::Texture(texture_coordinates) = &self.model_type_data {
                data.clear();
                let mut coord_count = 0;
                for (u, v) in texture_coordinates.iter() {
                    data.push_str(format!("vt {:.6} {:.6}\n", u, v).as_str());

                    if coord_count%WRITER_BUF_STRINGS==WRITER_BUF_STRINGS-1 {
                        f_obj.write_all(data.as_bytes())
                            .map_err(|err| {
                                format!("Can't write chunk of texture coordinates to obj file {}: {}", &result_path, err)})?;
                        data.clear();
                    }
                    coord_count += 1;
                }

                data.push_str(format!("# {} texture coordinates\n\n", &texture_coordinates.len()).as_str());
                f_obj.write_all(data.as_bytes())
                    .map_err(|err| {
                        format!("Can't write texture coordinates to obj file {}: {}", &result_path, err)})?;
            };

            // elements
            data.clear();
            data.push_str("usemtl Material\n");
            let pmap = match &self.modelpoints.points_map_opt {
                    None => return Err("Critical: Texture Appearance must use points mapping".to_string()),
                    Some(a) => a
            };
            let allowed_color_func = make_allowed_color_function(self.color_precision);
            let mut prev_color_id = None;
            let mut elements_count = 1;
            for (tvt0, tvt1, tvt2) in self.elements.iter() {
                let vt0 = match pmap.get(tvt0) {
                    Some(vt) => vt,
                    None => return Err(format!("Point tv0={} isn't found in points mapping", tvt0))
                };
                let vt1 = match pmap.get(tvt1) {
                    Some(vt) => vt,
                    None => return Err(format!("Point tv1={} isn't found in points mapping", tvt1))
                };
                let vt2 = match pmap.get(tvt2) {
                    Some(vt) => vt,
                    None => return Err(format!("Point tv2={} isn't found in points mapping", tvt2))
                };

                match &self.model_type_data {
                    ModelTypeData::Texture(_) =>
                        data.push_str(format!("f {}/{} {}/{} {}/{}\n", vt0+1, tvt0+1, vt1+1, tvt1+1, vt2+1, tvt2+1).as_str()),
                    ModelTypeData::Color(_) if self.color_precision==0 =>
                        data.push_str(format!("f {} {} {}\n", vt0+1, vt1+1, vt2+1).as_str()),
                    ModelTypeData::Color(colors) => {
                        let color = colors.get(vt0).ok_or(format!("Missed color for vertex {}", vt0))?;
                        let (_, color_id@(r_k, g_k, b_k)) = allowed_color_func(*color);
                        if prev_color_id.is_none() || Some(color_id)!=prev_color_id {
                            data.push_str(format!("usemtl c_{}_{}_{}\n", r_k, g_k, b_k).as_str());
                            prev_color_id = Some(color_id);
                        }
                        data.push_str(format!("f {} {} {}\n", vt0+1, vt1+1, vt2+1).as_str());
                    },
                }

                if elements_count%WRITER_BUF_STRINGS==WRITER_BUF_STRINGS-1 {
                    f_obj.write_all(data.as_bytes())
                        .map_err(|err| {
                            format!("Can't write chunk of elements to obj file {}: {}", &result_path, err)})?;
                    data.clear();
                }
                elements_count += 1;
            }

            data.push_str(format!("# {} elements\n\n", self.elements.len()).as_str());
            f_obj.write_all(data.as_bytes())
                .map_err(|err| {format!("Can't write elements to obj file {}: {}", &result_path, err)})?;

            f_obj.flush()
                .map_err(|err| {format!("Can't flush obj file {}: {}", &result_path, err)})
        };

        create_mtl()?;
        create_obj()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_geopoints_t0() {
        let model_size = Obj::make_valid_model_size(Some(3));
        let j_spacing = Obj::define_spacing(model_size);
        let (ModelPoints {geopoints, points_map_opt: pmap_opt}, elms) =
                Obj::create_modelpoints(model_size, j_spacing);
        let pmap = pmap_opt.unwrap();
        // println!("{:?}", geopoints);
        // println!("{:?}", pmap);
        // println!("{:?}", elms);
        assert_eq!(geopoints.len(), 6);
        assert_eq!(pmap.len(), 7);
        let elms_res = [
            (0, 1, 5), (1, 0, 6), (1, 2, 5), (2, 1, 6),
            (2, 3, 5), (3, 2, 6), (3, 4, 5), (4, 3, 6)
            ];
        assert_eq!(elms.len(), 8);
        assert_eq!(elms, elms_res);
    }

    #[test]
    fn create_geopoints_t1() {
        let model_size = Obj::make_valid_model_size(Some(4));
        let j_spacing = Obj::define_spacing(model_size);
        let (ModelPoints {geopoints, points_map_opt: pmap_opt}, elms) =
                Obj::create_modelpoints(model_size, j_spacing);
        let pmap = pmap_opt.unwrap();
        // println!("{:?}", geopoints);
        // println!("{:?}", pmap);
        // println!("{:?}", elms);
        assert_eq!(geopoints.len(), 18);
        assert_eq!(pmap.len(), 21);
        let elms_res = [
            (0, 1, 9), (1, 0, 10), (1, 2, 11), (2, 1, 12), (1, 11, 9),
            (1, 10, 12), (2, 3, 11), (3, 2, 12), (3, 4, 13), (4, 3, 14),
            (3, 13, 11), (3, 12, 14), (4, 5, 13), (5, 4, 14), (5, 6, 15),
            (6, 5, 16), (5, 15, 13), (5, 14, 16), (6, 7, 15), (7, 6, 16),
            (7, 8, 17), (8, 7, 18), (7, 17, 15), (7, 16, 18), (9, 11, 19),
            (12, 10, 20), (11, 13, 19), (14, 12, 20), (13, 15, 19), (16, 14, 20), (15, 17, 19), (18, 16, 20)
        ];
        assert_eq!(elms.len(), 32);
        assert_eq!(elms, elms_res);
    }

    #[test]
    fn create_geopoints_t2() {
        let model_size = Obj::make_valid_model_size(Some(8));
        let j_spacing = Obj::define_spacing(model_size);
        let (ModelPoints {geopoints, points_map_opt: pmap_opt}, elms) =
                Obj::create_modelpoints(model_size, j_spacing);
        let pmap = pmap_opt.unwrap();
        // println!("{:?}", geopoints);
        // println!("{:?}", pmap);
        // println!("{:?}", elms);
        assert_eq!(geopoints.len(), 66);
        assert_eq!(pmap.len(), 73);
        let elms_res = [
            (0, 1, 17), (1, 0, 18), (1, 2, 19), (2, 1, 20), (1, 19, 17), (1, 18, 20), (2, 3, 21),
            (3, 2, 22), (2, 21, 19), (2, 20, 22), (3, 4, 23), (4, 3, 24), (3, 23, 21), (3, 22, 24),
            (4, 5, 23), (5, 4, 24), (5, 6, 25), (6, 5, 26), (5, 25, 23), (5, 24, 26), (6, 7, 27),
            (7, 6, 28), (6, 27, 25), (6, 26, 28), (7, 8, 29), (8, 7, 30), (7, 29, 27), (7, 28, 30),
            (8, 9, 29), (9, 8, 30), (9, 10, 31), (10, 9, 32), (9, 31, 29), (9, 30, 32), (10, 11, 33),
            (11, 10, 34), (10, 33, 31), (10, 32, 34), (11, 12, 35), (12, 11, 36), (11, 35, 33), (11, 34, 36),
            (12, 13, 35), (13, 12, 36), (13, 14, 37), (14, 13, 38), (13, 37, 35), (13, 36, 38), (14, 15, 39),
            (15, 14, 40), (14, 39, 37), (14, 38, 40), (15, 16, 41), (16, 15, 42), (15, 41, 39), (15, 40, 42),

            (17, 19, 43), (20, 18, 44), (19, 21, 45), (22, 20, 46), (19, 45, 43),
            (20, 44, 46), (21, 23, 47), (24, 22, 48), (21, 47, 45), (22, 46, 48),
            (23, 25, 47), (26, 24, 48), (25, 27, 49), (28, 26, 50), (25, 49, 47),
            (26, 48, 50), (27, 29, 51), (30, 28, 52), (27, 51, 49), (28, 50, 52),
            (29, 31, 51), (32, 30, 52), (31, 33, 53), (34, 32, 54), (31, 53, 51),
            (32, 52, 54), (33, 35, 55), (36, 34, 56), (33, 55, 53), (34, 54, 56),
            (35, 37, 55), (38, 36, 56), (37, 39, 57), (40, 38, 58), (37, 57, 55),
            (38, 56, 58), (39, 41, 59), (42, 40, 60), (39, 59, 57), (40, 58, 60),

            (43, 45, 61), (46, 44, 62), (45, 47, 63), (48, 46, 64), (45, 63, 61), (46, 62, 64),
            (47, 49, 63), (50, 48, 64), (49, 51, 65), (52, 50, 66), (49, 65, 63), (50, 64, 66),
            (51, 53, 65), (54, 52, 66), (53, 55, 67), (56, 54, 68), (53, 67, 65), (54, 66, 68),
            (55, 57, 67), (58, 56, 68), (57, 59, 69), (60, 58, 70), (57, 69, 67), (58, 68, 70),

            (61, 63, 71), (64, 62, 72), (63, 65, 71), (66, 64, 72),
            (65, 67, 71), (68, 66, 72), (67, 69, 71), (70, 68, 72)
        ];
        assert_eq!(elms.len(), 128);
        assert_eq!(elms, elms_res);
    }

    #[test]
    fn create_texture_coordinates_t0() {
        let model_size = Obj::make_valid_model_size(Some(3));
        let tcs = Obj::create_texture_coordinates(model_size);
        // println!("{:?}", tcs);
        assert_eq!(tcs.len(), 7);
        let tcs_res = [
            (0.0, 0.5), (0.25, 0.5), (0.5, 0.5), (0.75, 0.5), (1.0, 0.5),
            (0.0, 1.0), (0.0, 0.0)
            ];
        assert_eq!(tcs, tcs_res);
    }

    #[test]
    fn create_texture_coordinates_t1() {
        let model_size = Obj::make_valid_model_size(Some(4));
        let tcs = Obj::create_texture_coordinates(model_size);
        // println!("{:?}", tcs);
        assert_eq!(tcs.len(), 21);
        let tcs_res = [
            (0.0, 0.5), (0.125, 0.5), (0.25, 0.5), (0.375, 0.5), (0.5, 0.5), (0.625, 0.5), (0.75, 0.5), (0.875, 0.5), (1.0, 0.5),
            (0.0, 0.75), (0.0, 0.25), (0.25, 0.75), (0.25, 0.25), (0.5, 0.75), (0.5, 0.25), (0.75, 0.75), (0.75, 0.25), (1.0, 0.75), (1.0, 0.25),
            (0.0, 1.0), (0.0, 0.0)
            ];
        assert_eq!(tcs, tcs_res);
    }
}





