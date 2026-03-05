/// Elevation as floating point number
pub type Height = f64;

/// Elevation as integer (typically used for DEM data)
pub type HeightInt = i16;

/// Coordinate as floating point number
pub type Coord = f64;

#[derive(Debug)]
/// Geopoint representing a location with longitude and latitude coordinates
/// 
/// This struct is used to represent geographic locations in the application,
/// typically for processing and calculating elevation data.
pub struct GeoPoint {
    /// Longitude coordinate in degrees
    pub lon: Coord,
    /// Latitude coordinate in degrees
    pub lat: Coord,
}

/// Index type for geopoints (used for tracking positions in collections)
pub type GeoPointIndex = usize;

/// Texture coordinate as floating point number
pub type TextureCoordinate = f64;

/// Type alias for error handling in the application
pub type ErrHandle = Box<dyn std::error::Error>;
