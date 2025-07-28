use std::mem;
use std::i16;
use std::f64::consts::PI;
use std::vec::Vec;
use std::path::Path;

use crate::common::types::*;


/// Converts a vector of u8 values to a vector of i16 values
pub fn vec_u8_to_i16 (mut vec_u8:Vec<u8>) -> Vec<i16> {
    if vec_u8.len() % 2 != 0 {
        panic!("Odd length of vector data, reminder: {}", vec_u8.len() % 2);
    }

    let mut vec_i16: Vec<i16>;
    unsafe {
        let length = vec_u8.len() / 2;
        let capacity = vec_u8.capacity() / 2;
        let mutptr = vec_u8.as_mut_ptr() as *mut i16;
        mem::forget(vec_u8); // don't run the destructor for vec_u8

        // construct new vec
        vec_i16 = Vec::from_raw_parts(mutptr, length, capacity);
        for value in &mut vec_i16 {
            *value = i16::from_be(*value);
        }
    };

    return vec_i16;
}

/// Calculates coordinates of a model vertex
pub fn calc_point3d(radius: Height, scale: Height, height: Height, lon: Coord, lat: Coord) -> (Coord, Coord, Coord) {
    let r = 1.0 + scale as f64*height as f64/radius as f64;
    let phi = lon as f64*PI/180.0;
    let theta = lat as f64*PI/180.0;
    let x = -r*phi.sin() * theta.cos();
    let y = r*phi.cos() * theta.cos();
    let z = r*theta.sin();
    (x as Coord, y as Coord, z as Coord)
}

/// Check validity of directory path specification
pub fn check_dir(value: &str) -> Result<(), String> {
    let p = Path::new(&value);
    if p.exists() && p.is_dir() {
        match p.to_str() {
            Some(_) => Ok(()),
            None => Err(format!("Invalid path: {}", value))
        }
    } else {
        Err(format!("Directory path is invalid or can't be read: {}", value))
    }
}

/// Check validity of file path specification
pub fn check_file(value: &str) -> Result<(), String> {
    let p = Path::new(&value);
    if p.exists() && p.is_file() {
        match p.to_str() {
            Some(_) => Ok(()),
            None => Err(format!("Invalid file: {}", value))
        }
    } else {
        Err(format!("File path is invalid or can't be read: {}", value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vec_u8_to_i16_t0() {
        let vec_u8 = vec![1u8; 16];
        let vec_i16 = vec_u8_to_i16(vec_u8);

        assert_eq!(vec_i16, vec![257; 8])
    }
}

