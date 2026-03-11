//! Color handling utilities for elevation mapping
//! 
//! This module provides functionality for working with RGB colors in the context of
//! elevation mapping. It includes:
//! 
//! - RGB color representation with formatting
//! - Color interpolation from elevation profiles
//! - Color quantization and palette management
//! - Parsing of color profile files
//! 
//! The module is designed to map elevation values to colors for visualization purposes,
//! where different elevation ranges are assigned specific RGB colors.
//! 
//! # Color Profile Files
//! 
//! Color profile files define the mapping between elevation values and RGB colors.
//! Each line in the file should contain:
//! 
//! ```text
//! <elevation> <red> <green> <blue>
//! ```
//! 
//! Where:
//! - Elevation is a non-negative integer
//! - RGB components are floating-point values between 0.0 and 1.0
//! 
//! Lines starting with '#' are treated as comments and ignored.
//! 
//! # Examples
//! 
//! ```rust
//! use crate::common::color::{get_color_mapping, make_allowed_color_function, RGB};
//! 
//! // Create a color mapping function from a profile file
//! let color_func = get_color_mapping("color_profile.txt").unwrap();
//! 
//! // Get color for elevation 1000
//! let color = color_func(1000).unwrap();
//! 
//! // Quantize a color to a palette
//! let allowed_func = make_allowed_color_function(8);
//! let (allowed_color, position) = allowed_func(color);
//! ```
use std::fmt;
use std::fs::read_to_string;
use std::collections::HashMap;
use regex::Regex;

use crate::common::types::*;


const DEFAULT_COLOR: RGB = RGB (0.5, 0.5, 0.5);


/// Color component type for RGB values
/// 
/// Represents a single color component (red, green, or blue) with floating-point precision.
/// Values should be in the range [0.0, 1.0].
pub type ColorComponent = f32;

/// RGB Color structure
/// 
/// Represents a color in the RGB color space with three components:
/// - Red (0.0 to 1.0)
/// - Green (0.0 to 1.0)  
/// - Blue (0.0 to 1.0)
/// 
/// Implements Display trait for formatted string output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RGB (pub ColorComponent, pub ColorComponent, pub ColorComponent);

impl fmt::Display for RGB {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}",
            format!("{:.3}", self.0).trim_end_matches("0").trim_end_matches('.'),
            format!("{:.3}", self.1).trim_end_matches("0").trim_end_matches('.'),
            format!("{:.3}", self.2).trim_end_matches("0").trim_end_matches('.'),
        )
    }
}

/// Integer precision type for defining colors
/// 
/// Used to specify the precision level for color quantization, typically
/// representing the number of discrete color levels in each RGB component.
pub type ColorPrecision = u16;

/// Color position type
/// 
/// Represents a color position as three discrete values (r, g, b) that define
/// a position in the RGB color space. Each component is a ColorPrecision value.
pub type ColorPosition = (ColorPrecision, ColorPrecision, ColorPrecision);

/// Elevation to RGB Color mapping
/// 
/// A HashMap that maps elevation values (HeightInt) to RGB colors.
/// Used to determine what color should be displayed for a given elevation.
pub type ColorMappning = HashMap<HeightInt, RGB>;

/// Color profile file content type
/// 
/// Represents the content of a color profile file as a vector of strings,
/// where each string corresponds to a line in the file.
type ColorProfileFileContent = Vec<String>;

/// Color profile record
/// 
/// Represents a single record in a color profile file containing:
/// - HeightInt: Elevation value
/// - ColorComponent: Red component value (0.0 to 1.0)
/// - ColorComponent: Green component value (0.0 to 1.0)
/// - ColorComponent: Blue component value (0.0 to 1.0)
#[derive(Debug, Clone, PartialEq)]
struct ColorRecord (
    pub HeightInt,
    pub ColorComponent,
    pub ColorComponent,
    pub ColorComponent,
);

/// Read color profile file lines into a vector
/// 
/// Reads the entire content of a color profile file and splits it into lines.
fn read_lines(filepath: &str) -> Result<ColorProfileFileContent, ErrBox> {
    read_to_string(filepath)
        .map(|str| {str.lines().map(String::from).collect()})
        .map_err(|err| {err.into()})
}

/// Build color table from file content
/// 
/// Parses color profile file content into a structured color table.
/// Each line should contain: height red green blue values separated by whitespace.
/// Lines starting with '#' are treated as comments and ignored.
fn build_color_table(file_content: ColorProfileFileContent) -> Result<Vec<ColorRecord>, ErrBox> {
    let re_line_ = Regex::new(r"^\s*(\d+)\s+(0(?:\.\d+)?|(?:\.\d+)|(?:1(?:\.0)?))\s+(0(?:\.\d+)?|(?:\.\d+)|(?:1(?:\.0)?))\s+(0(?:\.\d+)?|(?:\.\d+)|(?:1(?:\.0)?))\s*$");
    let re_line = match re_line_ {
        Ok(re_) => re_,
        Err(_) => return Err("RegExp error".into()),

    };  
    let re_comment_ = Regex::new(r"^#.*");
    let re_comment = match re_comment_ {
        Ok(re_) => re_,
        Err(_) => return Err("RegExp error".into()),

    };  
    let mut l: usize = 0;
    let mut prev_h: HeightInt = -32767;

    let mut color_table = vec![];
    for line in file_content {
        l += 1;
        match re_line.captures(&line) {
            Some(caps) => {
                let h = caps[1].parse::<HeightInt>()
                    .map_err(|err| -> ErrBox {format!("Can't parse height value at line {}: {}", l, err).into()})?;
                let r = caps[2].parse::<ColorComponent>()
                    .map_err(|err| -> ErrBox {format!("Can't parse Red component at line {}: {}", l, err).into()})?;
                let g = caps[3].parse::<ColorComponent>()
                    .map_err(|err| -> ErrBox {format!("Can't parse Green component at line {}: {}", l, err).into()})?;
                let b = caps[4].parse::<ColorComponent>()
                    .map_err(|err| -> ErrBox {format!("Can't parse Blue component at line {}: {}", l, err).into()})?;

                if h<=prev_h {
                    return Err(format!("Heights in color profile must be strictly incremental").into());
                }
                prev_h = h;

                color_table.push(ColorRecord (h, r, g, b));
            }
            None => {
                match re_comment.captures(&line) {
                    Some(_) => {
                    },
                    None => {
                        return Err(format!("Invalid line {} in color profile", l).into());
                    },
                }
            }
        }
    }

    if color_table.is_empty() {
        return Err("Color table is empty".into());
    } else {
        return Ok(color_table);
    }
}

/// Build color mapping from color table
/// 
/// Creates a mapping from elevation values to RGB colors by interpolating
/// between color records in the table. For heights between records, linear
/// interpolation is used to determine the color.
fn build_color_mapping(color_table: &Vec<ColorRecord>) -> Result<ColorMappning, ErrBox> {
    let mut color_mapping = HashMap::new();

    let ColorRecord (hl, rl, gl, bl) = &color_table.last()
        .ok_or_else(|| -> ErrBox { "Color table is empty".into() })?;
    color_mapping.insert(*hl, RGB (*rl, *gl, *bl));    // biggest height

    let mut color_table_copy = color_table.clone();
    color_table_copy.remove(0);

    let color_table_bounds = color_table.into_iter().zip(color_table_copy);

    for (
        ColorRecord (h0, r0, g0, b0),
        ColorRecord (h1, r1, g1, b1)
    ) in color_table_bounds {
        for h in *h0..h1 {
            let delta_h = (h-h0) as ColorComponent;
            let span_h = (h1-h0) as ColorComponent;
            color_mapping.insert(h, RGB (
                r0 + (r1-r0)*delta_h/span_h,
                g0 + (g1-g0)*delta_h/span_h,
                b0 + (b1-b0)*delta_h/span_h,
            ));
        }
    }

    return Ok(color_mapping);
}

/// Get color mapping function from color profile file
/// 
/// Creates a closure that maps elevation values to RGB colors by reading
/// a color profile file and building an interpolation mapping.
pub fn get_color_mapping<'a>(filepath: &'a str) -> Result<impl Fn(HeightInt) -> Result<RGB, ErrBox>, ErrBox> {
    let file_content = read_lines(&filepath)?;
    let color_table = build_color_table(file_content)?;

    let color_mapping = build_color_mapping(&color_table)?;

    let ColorRecord (h0, r0, g0, b0) = color_table.first()
            .ok_or_else(|| -> ErrBox { "Can't get first element in color table".into() })?
            .clone();
    let ColorRecord (h1, r1, g1, b1) = color_table.last()
            .ok_or_else(|| -> ErrBox { "Can't get last element in color table".into() })?
            .clone();

    Ok(move |h| {
        match color_mapping.get(&h) {
            Some(c) => Ok(*c),
            None =>
                if h<h0 { Ok(RGB (r0, g0, b0)) }
                else if h>h1 { Ok(RGB (r1, g1, b1)) }
                else { Err(format!("Missing color for elevation {}", h).into()) }
        }
    })
}

/// Create function to make allowed color from arbitrary color
/// 
/// Creates a closure that quantizes an RGB color to the nearest allowed color
/// based on the specified precision level. This is useful for color palette
/// restrictions or reducing color depth.
pub fn make_allowed_color_function(prec: ColorPrecision) -> impl Fn(RGB)-> (RGB, ColorPosition) {
    let interval = 1.0/(prec as ColorComponent);
    move |color| {
        if prec>0 {
            let RGB (r, g, b) = color;
            let r_k = ((r as ColorComponent)/interval).round();
            let g_k = ((g as ColorComponent)/interval).round();
            let b_k = ((b as ColorComponent)/interval).round();

            (RGB (
                (r_k*interval) as ColorComponent,
                (g_k*interval) as ColorComponent,
                (b_k*interval) as ColorComponent
            ), (r_k as ColorPrecision, g_k as ColorPrecision, b_k as ColorPrecision))
        } else {
            (DEFAULT_COLOR, (0, 0, 0))
        }
    }
}

/// Calculate color interval for given precision
/// 
/// Computes the interval size between discrete color values for a given precision.
/// This represents the step size in the color space for each component.
pub fn get_color_interval(prec: ColorPrecision) -> ColorComponent {
    1.0/(prec as ColorComponent)
}

/// Restore RGB color from discrete components
/// 
/// Creates an RGB color from discrete color position components and interval size.
/// This is the inverse operation of quantization.
pub fn make_rgb_color(interval: ColorComponent, r_k: ColorPrecision, g_k: ColorPrecision, b_k: ColorPrecision) -> RGB {
    RGB (r_k as ColorComponent * interval,
        g_k as ColorComponent * interval,
        b_k as ColorComponent * interval)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_color_table_t0() -> Result<(), ErrBox> {
        let file_content: ColorProfileFileContent = vec![
            String::from("0       0       0       0.7"),
            String::from("1       0       0.7     0  "),
            String::from("500     0.5     .7      0  "),
            String::from("1500    0.5     0.5     0  "),
            String::from("4000    0.6     0.4     0.1"),
            String::from("8000    0.8     0.2     0  "),
        ];

        match build_color_table(file_content) {
            Err(err) => Err(err),
            Ok(color_table) => {
                let color_table0 = vec![
                    ColorRecord (0, 0.0, 0.0, 0.7),
                    ColorRecord (1, 0.0, 0.7, 0.0),
                    ColorRecord (500, 0.5, 0.7, 0.0),
                    ColorRecord (1500, 0.5, 0.5, 0.0),
                    ColorRecord (4000, 0.6, 0.4, 0.1),
                    ColorRecord (8000, 0.8, 0.2, 0.0),
                ];
                // assert_eq!(color_table, color_table0)
                if color_table==color_table0 {
                    Ok(())
                } else {
                    Err(format!("wrong color_table: {:?}", color_table).into())
                }
            }
        }
    }

    #[test]
    fn build_color_table_t1() -> Result<(), ErrBox> {
        let file_content: ColorProfileFileContent = vec![
            String::from("0    0.5     0.5     0  "),
            String::from(" 1   0.6     0.4     0.1"),
            String::from("2    0.8     0.2     0  "),
        ];

        match build_color_table(file_content) {
            Err(err) => Err(err),
            Ok(color_table) => {
                let color_table0 = vec![
                    ColorRecord (0, 0.5, 0.5, 0.0),
                    ColorRecord (1, 0.6, 0.4, 0.1),
                    ColorRecord (2, 0.8, 0.2, 0.0),
                ];
                if color_table==color_table0 {
                    Ok(())
                } else {
                    Err(format!("wrong color_table: {:?}", color_table).into())
                }
            }
        }
    }

    #[test]
    fn build_color_table_t2() -> Result<(), ErrBox> {
        let file_content: ColorProfileFileContent = vec![
            String::from("0       0.0       0       0.7"),
            String::from("#1      1.0       0.7     0  "),
            String::from("3       .333      0.7     0  "),
        ];

        match build_color_table(file_content) {
            Err(err) => Err(err),
            Ok(color_table) => {
                let color_table0 = vec![
                    ColorRecord (0, 0.0, 0.0, 0.7),
                    ColorRecord (3, 0.333, 0.7, 0.0),
                ];
                if color_table==color_table0 {
                    Ok(())
                } else {
                    Err(format!("wrong color_table: {:?}", color_table).into())
                }
            }
        }
    }

    #[test]
    fn build_color_table_t3() -> Result<(), ErrBox> {
        let file_content: ColorProfileFileContent = vec![
            String::from("0       0       0       0.7"),
            String::from("1       0       0.7     .3"),
            String::from("500     0.5     0.7     0  "),
        ];

        match build_color_table(file_content) {
            Err(err) => Err(err),
            Ok(color_table) => {
                let color_table0 = vec![
                    ColorRecord (0, 0.0, 0.0, 0.7),
                    ColorRecord (1, 0.0, 0.7, 0.0),
                    ColorRecord (500, 0.5, 0.7, 0.0),
                ];
                if color_table==color_table0 {
                    Err(format!("wrong color_table: {:?}", color_table).into())
                } else {
                    Ok(())
                }
            }
        }
    }

    #[test]
    fn build_color_table_t4() -> Result<(), String> {
        let file_content: ColorProfileFileContent = vec![
            String::from("0       0       0       0.7"),
            String::from("1500    0.5     0.5     0  "),
            String::from("500     0.5     0.7     0  "),
            String::from("4000    0.6     0.4     0.1"),
        ];

        match build_color_table(file_content) {
            Err(err) => {
                if err.to_string()==format!("Heights in color profile must be strictly incremental") {
                    Ok(())
                } else {
                    Err(format!("Got: {}", err))
                }

            },
            Ok(color_table) => Err(format!("wrong color_table: {:?}", color_table))
        }
    }

    #[test]
    fn build_color_table_t5() -> Result<(), ErrBox> {
        let file_content: ColorProfileFileContent = vec![
            String::from("0       0       0       0.7"),
            String::from("500     1.5     0.7     0  "),
            String::from("1500    0.5     0.5     0  "),
            String::from("4000    0.6     0.4     0.1"),
        ];

        match build_color_table(file_content) {
            Err(err) => {
                if err.to_string()==format!("Invalid line {} in color profile", 2) {
                    Ok(())
                } else {
                    Err(format!("Got: {}", err).into())
                }
            },
            Ok(color_table) => Err(format!("wrong color_table: {:?}", color_table).into())
        }
    }

    #[test]
    fn build_color_table_t6() -> Result<(), ErrBox> {
        let file_content: ColorProfileFileContent = vec![
            String::from("0       0       0       0.7"),
            String::from("500     0.5     -0.7    0  "),
            String::from("1500    0.5     0.5     0  "),
            String::from("4000    0.6     0.4     0.1"),
        ];

        match build_color_table(file_content) {
            Err(err) => {
                if err.to_string()==format!("Invalid line {} in color profile", 2) {
                    Ok(())
                } else {
                    Err(format!("Got: {}", err).into())
                }
            },
            Ok(color_table) => Err(format!("wrong color_table: {:?}", color_table).into())
        }
    }

    #[test]
    fn build_color_table_t7() -> Result<(), ErrBox> {
        let file_content: ColorProfileFileContent = vec![
            String::from("0       0       0       0.7"),
            String::from("500     0.5     0.7     1.0001  "),
            String::from("1500    0.5     0.5     0  "),
            String::from("4000    0.6     0.4     0.1"),
        ];

        match build_color_table(file_content) {
            Err(err) => {
                if err.to_string()==format!("Invalid line {} in color profile", 2) {
                    Ok(())
                } else {
                    Err(format!("Got: {}", err).into())
                }
            },
            Ok(color_table) => Err(format!("wrong color_table: {:?}", color_table).into())
        }
    }

    #[test]
    fn build_color_table_t8() -> Result<(), ErrBox> {
        let file_content: ColorProfileFileContent = vec![
        ];

        match build_color_table(file_content) {
            Err(err) => {
                if err.to_string()==format!("Color table is empty") {
                    Ok(())
                } else {
                    Err(format!("Got: {}", err).into())
                }
            },
            Ok(color_table) => Err(format!("wrong color_table: {:?}", color_table).into())
        }
    }

    #[test]
    fn allowed_color_function_test() {
        // Test that the function works correctly
        let func = make_allowed_color_function(4);
        let result = func(RGB(0.3, 0.7, 0.9));
        assert_eq!(result.0, RGB(0.25, 0.75, 1.0));
        assert_eq!(result.1, (1, 3, 4));
    }

    #[test]
    fn make_rgb_color_test() {
        let interval = 0.25;
        let color = make_rgb_color(interval, 1, 2, 3);
        assert_eq!(color, RGB(0.25, 0.5, 0.75));
    }
}
