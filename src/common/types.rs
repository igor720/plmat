/// Elevation
pub type Height = f64;

/// Elevation as integer
pub type HeightInt = i16;

/// Coordinate
pub type Coord = f64;

#[derive(Debug)]
/// Geopoint as longitude and latitude
pub struct GeoPoint {
    pub lon: Coord,
    pub lat: Coord,
}

/// Geopoints counter
pub type GeoPointIndex = usize;

// Texture coordinate
pub type TextureCoordinate = f64;

