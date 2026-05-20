use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

pub mod algorithm;
pub mod candidates;
pub mod color;
pub mod error;
pub mod parse;

use algorithm::{select_palette, DistanceWeights, PaletteAnchors, PaletteOptions};
use candidates::{generate_candidates, CandidateConstraints, GridSize};
use color::Rgb8;
use error::GlasbeyError;
use parse::parse_hex_color;

#[pyfunction]
#[pyo3(signature = (
    palette_size,
    seed_colors = None,
    avoid_colors = None,
    background = None,
    lightness = None,
    chroma = None,
    hue = None,
    grid_step = 8,
    lightness_weight = 1.0,
    chroma_weight = 1.0,
))]
#[allow(clippy::too_many_arguments)]
fn generate_palette_rs(
    py: Python<'_>,
    palette_size: usize,
    seed_colors: Option<Vec<String>>,
    avoid_colors: Option<Vec<String>>,
    background: Option<String>,
    lightness: Option<(f32, f32)>,
    chroma: Option<(Option<f32>, Option<f32>)>,
    hue: Option<(f32, f32)>,
    grid_step: u8,
    lightness_weight: f32,
    chroma_weight: f32,
) -> PyResult<Vec<String>> {
    py.detach(move || {
        generate_palette_inner(
            palette_size,
            seed_colors,
            avoid_colors,
            background,
            lightness,
            chroma,
            hue,
            grid_step,
            lightness_weight,
            chroma_weight,
        )
    })
    .map_err(to_py_value_error)
}

#[allow(clippy::too_many_arguments)]
fn generate_palette_inner(
    palette_size: usize,
    seed_colors: Option<Vec<String>>,
    avoid_colors: Option<Vec<String>>,
    background: Option<String>,
    lightness: Option<(f32, f32)>,
    chroma: Option<(Option<f32>, Option<f32>)>,
    hue: Option<(f32, f32)>,
    grid_step: u8,
    lightness_weight: f32,
    chroma_weight: f32,
) -> Result<Vec<String>, GlasbeyError> {
    let seed_colors = parse_hex_colors(seed_colors.unwrap_or_default())?;
    let avoid_colors = parse_hex_colors(avoid_colors.unwrap_or_default())?;
    let background = background.as_deref().map(parse_hex_color).transpose()?;
    let constraints = CandidateConstraints {
        lightness,
        chroma,
        hue,
    };
    let candidates = generate_candidates(GridSize::Step(grid_step), constraints, palette_size)?;
    let palette = select_palette(
        &candidates,
        PaletteOptions {
            palette_size,
            anchors: PaletteAnchors {
                seed_colors: &seed_colors,
                avoid_colors: &avoid_colors,
                background,
            },
            weights: DistanceWeights {
                lightness: lightness_weight,
                chroma: chroma_weight,
            },
        },
    )?;

    Ok(palette.into_iter().map(Rgb8::to_hex).collect())
}

fn parse_hex_colors(colors: Vec<String>) -> Result<Vec<Rgb8>, GlasbeyError> {
    colors
        .into_iter()
        .map(|color| parse_hex_color(&color))
        .collect()
}

fn to_py_value_error(error: GlasbeyError) -> PyErr {
    PyValueError::new_err(error.to_string())
}

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(generate_palette_rs, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_bridge_generates_canonical_hex_palette() {
        let palette = generate_palette_inner(
            3,
            Some(vec!["#f00".to_owned()]),
            None,
            Some("#fff".to_owned()),
            Some((0.2, 0.9)),
            Some((Some(0.04), None)),
            None,
            64,
            1.0,
            1.0,
        )
        .unwrap();

        assert_eq!(palette.len(), 3);
        assert!(palette.iter().all(|color| {
            color.len() == 7
                && color.starts_with('#')
                && color.chars().skip(1).all(|ch| ch.is_ascii_hexdigit())
                && color == &color.to_lowercase()
        }));
        assert!(!palette.contains(&"#ff0000".to_owned()));
        assert!(!palette.contains(&"#ffffff".to_owned()));
    }

    #[test]
    fn native_bridge_maps_engine_errors() {
        let error = generate_palette_inner(
            1,
            Some(vec!["not-a-color".to_owned()]),
            None,
            None,
            None,
            None,
            None,
            8,
            1.0,
            1.0,
        )
        .unwrap_err();

        assert!(matches!(error, GlasbeyError::InvalidHexLength { .. }));
    }

    #[test]
    fn native_bridge_reports_insufficient_candidates() {
        let error = generate_palette_inner(9, None, None, None, None, None, None, 255, 1.0, 1.0)
            .unwrap_err();

        assert_eq!(
            error,
            GlasbeyError::InsufficientCandidates {
                available: 8,
                requested: 9
            }
        );
    }
}
