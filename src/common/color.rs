use std::fmt;
use std::fs::read_to_string;
use std::collections::HashMap;
use regex::Regex;

use crate::common::types::*;


const DEFAULT_COLOR: RGB = RGB (0.5, 0.5, 0.5);


/// Color componenent
pub type ColorComponent = f32;

#[derive(Debug, Clone, Copy, PartialEq)]
/// RGB Color
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

/// Integer precision for defining colors
pub type ColorPrecision = u16;

/// Color specification with three numbers defining positions in rgb color intervals
pub type ColorPosition = (ColorPrecision, ColorPrecision, ColorPrecision);

/// Elevation to RGB Color mapping
pub type ColorMappning = HashMap<HeightInt, RGB>;

/// Color profile file content
type ColorProfileFileContent = Vec<String>;

#[derive(Debug, Clone, PartialEq)]
/// Color profile record (element of color table)
struct ColorRecord (
    pub HeightInt,
    pub ColorComponent,
    pub ColorComponent,
    pub ColorComponent,
);

/// Read color profile file
fn read_lines(filepath: &str) -> Result<ColorProfileFileContent, String> {
    read_to_string(filepath)
        .map(|str| {str.lines().map(String::from).collect()})
        .map_err(|err| {err.to_string()})
}

/// Build color table
fn build_color_table(file_content: ColorProfileFileContent) -> Result<Vec<ColorRecord>, String> {
    let re = Regex::new(r"^([0-9]+)\s+([0-9.]+)\s+([0-9.]+)\s+([0-9.]+)\s*$").unwrap();
    let mut l: usize = 0;
    let mut prev_h: HeightInt = -32767;

    let mut color_table = vec![];
    for line in file_content {
        l += 1;
        match re.captures(&line) {
            Some(caps) => {
                let h = caps[1].parse::<HeightInt>().unwrap();
                let r = caps[2].parse::<ColorComponent>().unwrap();
                let g = caps[3].parse::<ColorComponent>().unwrap();
                let b = caps[4].parse::<ColorComponent>().unwrap();

                if r>1.0 || r<0.0 {
                    return Err(format!("Invalid number in second column of line {} in color profile", l));
                }
                if g>1.0 || g<0.0 {
                    return Err(format!("Invalid number in third column of line {} in color profile", l));
                }
                if b>1.0 || b<0.0 {
                    return Err(format!("Invalid number in fourth column of line {} in color profile", l));
                }

                if h<=prev_h {
                    return Err(format!("Heights in color profile must be strictly incremental"));
                }
                prev_h = h;

                color_table.push(ColorRecord (h, r, g, b));
            }
            None => {}
        }
    }

    if color_table.is_empty() {
        return Err("Color table is empty".to_string());
    } else {
        return Ok(color_table);
    }
}

/// Builds color mapping data (HeightInt -> RGB)
fn build_color_mapping(color_table: Vec<ColorRecord>) -> ColorMappning {
    let mut color_mapping = HashMap::new();

    let ColorRecord (hl, rl, gl, bl) = &color_table.last().unwrap();
    color_mapping.insert(*hl, RGB (*rl, *gl, *bl));    // biggest height

    let mut color_table_copy = color_table.clone();
    color_table_copy.remove(0);

    let color_table_bounds = color_table.into_iter().zip(color_table_copy);

    for (
        ColorRecord (h0, r0, g0, b0),
        ColorRecord (h1, r1, g1, b1)
    ) in color_table_bounds {
        for h in h0..h1 {
            let delta_h = (h-h0) as ColorComponent;
            let span_h = (h1-h0) as ColorComponent;
            color_mapping.insert(h, RGB (
                r0 + (r1-r0)*delta_h/span_h,
                g0 + (g1-g0)*delta_h/span_h,
                b0 + (b1-b0)*delta_h/span_h,
            ));
        }
    }

    return color_mapping;
}

/// Returns function to mapping values of HeightInt type to RGB values
pub fn get_color_mapping(filepath: &str) -> Result<impl Fn(HeightInt) -> RGB, String> {
    let file_content = read_lines(&filepath)?;
    let color_table = build_color_table(file_content)?;

    let ColorRecord (h0, r0, g0, b0) = color_table.first().unwrap().clone();
    let ColorRecord (h1, r1, g1, b1) = color_table.last().unwrap().clone();

    let color_mapping = build_color_mapping(color_table);

    Ok(move |h| {
        match color_mapping.get(&h) {
            Some(c) => *c,
            None =>
                if h<h0 {RGB (r0, g0, b0)}
                else if h>h1 {RGB (r1, g1, b1)}
                else {panic!("Missing color for elevation {}", h)}
        }
    })
}

/// Returns function which makes allowed color from arbitrary color
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

/// Calculates interval within rgb intervals which separates individual color component values
pub fn get_color_interval(prec: ColorPrecision) -> ColorComponent {
    1.0/(prec as ColorComponent)
}

/// Restores rgb color from components
pub fn make_rgb_color(interval: ColorComponent, r_k: ColorPrecision, g_k: ColorPrecision, b_k: ColorPrecision) -> RGB {
    RGB (r_k as ColorComponent * interval,
        g_k as ColorComponent * interval,
        b_k as ColorComponent * interval)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_color_table_t0() -> Result<(), String> {
        let file_content: ColorProfileFileContent = vec![
            String::from("0       0       0       0.7"),
            String::from("1       0       0.7     0  "),
            String::from("500     0.5     0.7     0  "),
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
                    Err(format!("wrong color_table: {:?}", color_table))
                }
            }
        }
    }

    #[test]
    fn build_color_table_t1() -> Result<(), String> {
        let file_content: ColorProfileFileContent = vec![
            String::from("1500    0.5     0.5     0  "),
            String::from(" 4000    0.6     0.4     0.1"),
            String::from("8000    0.8     0.2     0  "),
        ];

        match build_color_table(file_content) {
            Err(err) => Err(err),
            Ok(color_table) => {
                let color_table0 = vec![
                    ColorRecord (1500, 0.5, 0.5, 0.0),
                    ColorRecord (8000, 0.8, 0.2, 0.0),
                ];
                if color_table==color_table0 {
                    Ok(())
                } else {
                    Err(format!("wrong color_table: {:?}", color_table))
                }
            }
        }
    }

    #[test]
    fn build_color_table_t2() -> Result<(), String> {
        let file_content: ColorProfileFileContent = vec![
            String::from("0       0       0       0.7"),
            String::from("#1       0       0.7     0  "),
            String::from("500     0.5     0.7     0  "),
        ];

        match build_color_table(file_content) {
            Err(err) => Err(err),
            Ok(color_table) => {
                let color_table0 = vec![
                    ColorRecord (0, 0.0, 0.0, 0.7),
                    ColorRecord (500, 0.5, 0.7, 0.0),
                ];
                if color_table==color_table0 {
                    Ok(())
                } else {
                    Err(format!("wrong color_table: {:?}", color_table))
                }
            }
        }
    }

    #[test]
    fn build_color_table_t3() -> Result<(), String> {
        let file_content: ColorProfileFileContent = vec![
            String::from("0       0       0       0.7"),
            String::from("1       0       0.7   "),
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
                    Err(format!("wrong color_table: {:?}", color_table))
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
                if err==format!("Heights in color profile must be strictly incremental") {
                    Ok(())
                } else {
                    Err(format!("Got: {}", err))
                }

            },
            Ok(color_table) => Err(format!("wrong color_table: {:?}", color_table))
        }
    }

    #[test]
    fn build_color_table_t5() -> Result<(), String> {
        let file_content: ColorProfileFileContent = vec![
            String::from("0       0       0       0.7"),
            String::from("500     1.5     0.7     0  "),
            String::from("1500    0.5     0.5     0  "),
            String::from("4000    0.6     0.4     0.1"),
        ];

        match build_color_table(file_content) {
            Err(err) => {
                if err==format!("Invalid number in second column of line {} in color profile", 2) {
                    Ok(())
                } else {
                    Err(format!("Got: {}", err))
                }
            },
            Ok(color_table) => Err(format!("wrong color_table: {:?}", color_table))
        }
    }

    #[test]
    fn build_color_table_t6() -> Result<(), String> {
        let file_content: ColorProfileFileContent = vec![
            String::from("0       0       0       0.7"),
            String::from("500     0.5     2.7     0  "),
            String::from("1500    0.5     0.5     0  "),
            String::from("4000    0.6     0.4     0.1"),
        ];

        match build_color_table(file_content) {
            Err(err) => {
                if err==format!("Invalid number in third column of line {} in color profile", 2) {
                    Ok(())
                } else {
                    Err(format!("Got: {}", err))
                }
            },
            Ok(color_table) => Err(format!("wrong color_table: {:?}", color_table))
        }
    }

    #[test]
    fn build_color_table_t7() -> Result<(), String> {
        let file_content: ColorProfileFileContent = vec![
            String::from("0       0       0       0.7"),
            String::from("500     0.5     0.7     1.0001  "),
            String::from("1500    0.5     0.5     0  "),
            String::from("4000    0.6     0.4     0.1"),
        ];

        match build_color_table(file_content) {
            Err(err) => {
                if err==format!("Invalid number in fourth column of line {} in color profile", 2) {
                    Ok(())
                } else {
                    Err(format!("Got: {}", err))
                }
            },
            Ok(color_table) => Err(format!("wrong color_table: {:?}", color_table))
        }
    }

    #[test]
    fn build_color_table_t8() -> Result<(), String> {
        let file_content: ColorProfileFileContent = vec![
            String::from(" 0       0       0       0.7"),
            String::from("#500     0.5     0.7     0.1  "),
        ];

        match build_color_table(file_content) {
            Err(err) => {
                if err==format!("Color table is empty") {
                    Ok(())
                } else {
                    Err(format!("Got: {}", err))
                }
            },
            Ok(color_table) => Err(format!("wrong color_table: {:?}", color_table))
        }
    }

    #[test]
    fn allowed_color_function_t0() {
        let color0 = RGB (0.0, 0.35, 1.0);

        let f1 = make_allowed_color_function(1);
        assert_eq!(f1(color0), (RGB (0.0, 0.0, 1.0), (0, 0, 1)));

        let f2 = make_allowed_color_function(2);
        assert_eq!(f2(color0), (RGB (0.0, 0.5, 1.0), (0, 1, 2)));

        let f4 = make_allowed_color_function(4);
        assert_eq!(f4(color0), (RGB (0.0, 0.25, 1.0), (0, 1, 4)));

        let f8 = make_allowed_color_function(8);
        assert_eq!(f8(color0), (RGB (0.0, 0.375, 1.0), (0, 3, 8)));

        let f0 = make_allowed_color_function(0);
        assert_eq!(f0(color0), (DEFAULT_COLOR, (0, 0, 0)));
    }
}
