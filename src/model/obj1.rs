//! This module provides functionality to generate 3D models using OBJ format, specifically tailored for planetary data visualization.

use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

/// Represents a geographic point on the surface of a planet, specified by latitude and longitude.
#[derive(Debug, Clone, Copy)]
pub struct GeoPoint {
    /// Latitude in degrees. Positive values are north of the equator, negative values are south.
    pub lat: f64,
    /// Longitude in degrees. Positive values are east of the prime meridian, negative values are west.
    pub lon: f64,
}

impl GeoPoint {
    /// Creates a new `GeoPoint` from latitude and longitude values.
    /// 
    /// # Arguments
    /// 
    /// * `lat`: The latitude in degrees.
    /// * `lon`: The longitude in degrees.
    pub fn new(lat: f64, lon: f64) -> Self {
        GeoPoint { lat, lon }
    }
}

/// Represents a vertex in the 3D model with texture coordinates and its corresponding index in the points mapping.
#[derive(Debug)]
pub struct VertexTextureIndex {
    /// Index of the vertex in the points mapping.
    pub index: usize,
    /// Texture coordinate for this vertex.
    pub tex_coord: (f64, f64),
}

/// Struct representing a 3D model with its geographic points and texture coordinates.
pub struct ModelPoints {
    /// A vector of `GeoPoint`s representing the vertices of the model.
    pub geopoints: Vec<GeoPoint>,
    /// An optional mapping from vertex indices to their corresponding texture coordinates.
    pub points_map_opt: Option<HashMap<usize, VertexTextureIndex>>,
}

/// Struct representing a 3D model in OBJ format with options for color appearance and element indexing.
pub struct Obj {
    /// The size of the model grid (e.g., 4 for a 4x4 grid).
    pub size: usize,
    /// A vector containing tuples of indices representing triangular faces of the model.
    pub elements: Vec<(usize, usize, usize)>,
    /// Precision level for color appearance in the OBJ file.
    pub color_precision: i32,
}

impl Obj {
    /// Calculates a valid size for the model grid based on the input size or defaults to 4 if none is provided.
    /// 
    /// # Arguments
    /// 
    /// * `model_size`: Optional size of the model grid. If None, defaults to 4.
    pub fn make_valid_model_size(model_size: Option<usize>) -> usize {
        model_size.unwrap_or(4)
    }

    /// Defines the spacing between points in the model based on its size.
    /// 
    /// # Arguments
    /// 
    /// * `model_size`: The size of the model grid.
    pub fn define_spacing(model_size: usize) -> f64 {
        1.0 / (model_size - 1) as f64
    }

    /// Creates a vector of `GeoPoint`s representing the vertices of the model and an optional mapping for texture coordinates.
    /// 
    /// # Arguments
    /// 
    /// * `model_size`: The size of the model grid.
    /// * `j_spacing`: The spacing between points in the model.
    pub fn create_modelpoints(model_size: usize, j_spacing: f64) -> (ModelPoints, Vec<(usize, usize, usize)>) {
        let mut geopoints = Vec::new();
        for i in 0..model_size {
            for j in 0..model_size {
                let lat = -90.0 + i as f64 * j_spacing * 180.0;
                let lon = -180.0 + j as f64 * j_spacing * 360.0;
                geopoints.push(GeoPoint::new(lat, lon));
            }
        }

        let mut points_map = HashMap::new();
        for (idx, point) in geopoints.iter().enumerate() {
            points_map.insert(idx, VertexTextureIndex { index: idx, tex_coord: ((idx % model_size) as f64 / (model_size - 1) as f64, (idx / model_size) as f64 / (model_size - 1) as f64) });
        }

        let mut elements = Vec::new();
        for i in 0..(model_size * model_size - model_size) {
            if i % model_size != model_size - 1 {
                elements.push((i, i + 1, i + model_size));
            } else {
                elements.push((i, i + 1 - model_size, i + model_size));
            }
        }

        (ModelPoints { geopoints, points_map_opt: Some(points_map) }, elements)
    }

    /// Creates a vector of texture coordinates for the vertices of the model.
    /// 
    /// # Arguments
    /// 
    /// * `model_size`: The size of the model grid.
    pub fn create_texture_coordinates(model_size: usize) -> Vec<(f64, f64)> {
        let mut texture_coords = Vec::new();
        for i in 0..model_size {
            for j in 0..model_size {
                texture_coords.push(((i as f64 / (model_size - 1) as f64), (j as f64 / (model_size - 1) as f64)));
            }
        }
        texture_coords
    }
}