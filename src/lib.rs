use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

pub mod algorithm;
pub mod candidates;
pub mod color;
pub mod error;
pub mod label;
pub mod parse;
pub mod render;

use algorithm::{select_palette, DistanceWeights, PaletteAnchors, PaletteOptions};
use candidates::{
    generate_candidates_with_background_filter, BackgroundFilter, CandidateConstraints, GridSize,
};
use color::Rgb8;
use error::GlasbeyError;
use label::{select_label_palette, LabelPaletteOptions};
use parse::parse_hex_color;
use pyo3::types::PyBytes;
use render::{render_palette_png, render_palette_svg};

#[pyfunction]
#[pyo3(signature = (
    palette_size,
    seed_colors = None,
    avoid_colors = None,
    backgrounds = None,
    background_min_distance_squared = None,
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
    backgrounds: Option<Vec<String>>,
    background_min_distance_squared: Option<f32>,
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
            backgrounds,
            background_min_distance_squared,
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
    backgrounds: Option<Vec<String>>,
    background_min_distance_squared: Option<f32>,
    lightness: Option<(f32, f32)>,
    chroma: Option<(Option<f32>, Option<f32>)>,
    hue: Option<(f32, f32)>,
    grid_step: u8,
    lightness_weight: f32,
    chroma_weight: f32,
) -> Result<Vec<String>, GlasbeyError> {
    let seed_colors = parse_hex_colors(seed_colors.unwrap_or_default())?;
    let avoid_colors = parse_hex_colors(avoid_colors.unwrap_or_default())?;
    let backgrounds = parse_hex_colors(backgrounds.unwrap_or_default())?;
    let background_labs: Vec<_> = backgrounds.iter().map(|color| color.to_oklab()).collect();
    let constraints = CandidateConstraints {
        lightness,
        chroma,
        hue,
    };
    let weights = DistanceWeights {
        lightness: lightness_weight,
        chroma: chroma_weight,
    };
    weights.validate()?;
    let candidates = generate_candidates_with_background_filter(
        GridSize::Step(grid_step),
        constraints,
        BackgroundFilter {
            backgrounds: &background_labs,
            min_distance_squared: background_min_distance_squared,
            lightness_weight: weights.lightness,
            chroma_weight: weights.chroma,
        },
        palette_size,
    )?;
    let palette = select_palette(
        &candidates,
        PaletteOptions {
            palette_size,
            anchors: PaletteAnchors {
                seed_colors: &seed_colors,
                avoid_colors: &avoid_colors,
                backgrounds: &backgrounds,
            },
            weights,
        },
    )?;

    Ok(palette.into_iter().map(Rgb8::to_hex).collect())
}

#[pyfunction]
#[pyo3(signature = (
    coordinates,
    dimension,
    label_ids,
    label_count,
    fixed_colors,
    seed_colors = None,
    avoid_colors = None,
    backgrounds = None,
    background_min_distance_squared = None,
    lightness = None,
    chroma = None,
    hue = None,
    grid_step = 8,
    lightness_weight = 1.0,
    chroma_weight = 1.0,
    neighbors = 8,
    max_points = Some(50_000),
))]
#[allow(clippy::too_many_arguments)]
fn generate_label_palette_rs(
    py: Python<'_>,
    coordinates: Vec<f64>,
    dimension: usize,
    label_ids: Vec<usize>,
    label_count: usize,
    fixed_colors: Vec<Option<String>>,
    seed_colors: Option<Vec<String>>,
    avoid_colors: Option<Vec<String>>,
    backgrounds: Option<Vec<String>>,
    background_min_distance_squared: Option<f32>,
    lightness: Option<(f32, f32)>,
    chroma: Option<(Option<f32>, Option<f32>)>,
    hue: Option<(f32, f32)>,
    grid_step: u8,
    lightness_weight: f32,
    chroma_weight: f32,
    neighbors: usize,
    max_points: Option<usize>,
) -> PyResult<Vec<String>> {
    py.detach(move || {
        generate_label_palette_inner(
            coordinates,
            dimension,
            label_ids,
            label_count,
            fixed_colors,
            seed_colors,
            avoid_colors,
            backgrounds,
            background_min_distance_squared,
            lightness,
            chroma,
            hue,
            grid_step,
            lightness_weight,
            chroma_weight,
            neighbors,
            max_points,
        )
    })
    .map_err(to_py_value_error)
}

#[allow(clippy::too_many_arguments)]
fn generate_label_palette_inner(
    coordinates: Vec<f64>,
    dimension: usize,
    label_ids: Vec<usize>,
    label_count: usize,
    fixed_colors: Vec<Option<String>>,
    seed_colors: Option<Vec<String>>,
    avoid_colors: Option<Vec<String>>,
    backgrounds: Option<Vec<String>>,
    background_min_distance_squared: Option<f32>,
    lightness: Option<(f32, f32)>,
    chroma: Option<(Option<f32>, Option<f32>)>,
    hue: Option<(f32, f32)>,
    grid_step: u8,
    lightness_weight: f32,
    chroma_weight: f32,
    neighbors: usize,
    max_points: Option<usize>,
) -> Result<Vec<String>, GlasbeyError> {
    let fixed_colors = fixed_colors
        .into_iter()
        .map(|color| color.as_deref().map(parse_hex_color).transpose())
        .collect::<Result<Vec<_>, _>>()?;
    let seed_colors = parse_hex_colors(seed_colors.unwrap_or_default())?;
    let avoid_colors = parse_hex_colors(avoid_colors.unwrap_or_default())?;
    let backgrounds = parse_hex_colors(backgrounds.unwrap_or_default())?;
    let background_labs: Vec<_> = backgrounds.iter().map(|color| color.to_oklab()).collect();
    let constraints = CandidateConstraints {
        lightness,
        chroma,
        hue,
    };
    let weights = DistanceWeights {
        lightness: lightness_weight,
        chroma: chroma_weight,
    };

    let palette = select_label_palette(LabelPaletteOptions {
        coordinates: &coordinates,
        dimension,
        label_ids: &label_ids,
        label_count,
        fixed_colors: &fixed_colors,
        constraints,
        background_filter: BackgroundFilter {
            backgrounds: &background_labs,
            min_distance_squared: background_min_distance_squared,
            lightness_weight: weights.lightness,
            chroma_weight: weights.chroma,
        },
        grid_size: GridSize::Step(grid_step),
        anchors: PaletteAnchors {
            seed_colors: &seed_colors,
            avoid_colors: &avoid_colors,
            backgrounds: &backgrounds,
        },
        weights,
        neighbors,
        max_points,
    })?;

    Ok(palette.into_iter().map(Rgb8::to_hex).collect())
}

fn parse_hex_colors(colors: Vec<String>) -> Result<Vec<Rgb8>, GlasbeyError> {
    colors
        .into_iter()
        .map(|color| parse_hex_color(&color))
        .collect()
}

#[pyfunction]
#[pyo3(signature = (colors, width = 1246, height = 154))]
fn palette_svg_rs(colors: Vec<String>, width: u32, height: u32) -> PyResult<String> {
    let colors = parse_hex_colors(colors).map_err(to_py_value_error)?;
    render_palette_svg(&colors, width, height).map_err(to_py_value_error)
}

#[pyfunction]
#[pyo3(signature = (colors, width = 1246, height = 154))]
fn palette_png_rs(
    py: Python<'_>,
    colors: Vec<String>,
    width: u32,
    height: u32,
) -> PyResult<Py<PyBytes>> {
    let colors = parse_hex_colors(colors).map_err(to_py_value_error)?;
    let png = render_palette_png(&colors, width, height).map_err(to_py_value_error)?;
    Ok(PyBytes::new(py, &png).unbind())
}

fn to_py_value_error(error: GlasbeyError) -> PyErr {
    PyValueError::new_err(error.to_string())
}

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(generate_palette_rs, m)?)?;
    m.add_function(wrap_pyfunction!(generate_label_palette_rs, m)?)?;
    m.add_function(wrap_pyfunction!(palette_svg_rs, m)?)?;
    m.add_function(wrap_pyfunction!(palette_png_rs, m)?)?;
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
            Some(vec!["#fff".to_owned()]),
            Some(0.006),
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
        let error =
            generate_palette_inner(9, None, None, None, None, None, None, None, 255, 1.0, 1.0)
                .unwrap_err();

        assert_eq!(
            error,
            GlasbeyError::InsufficientCandidates {
                available: 8,
                requested: 9
            }
        );
    }

    #[test]
    fn native_bridge_renders_svg_and_png_previews() {
        let colors = vec!["#ff0000".to_owned(), "#00ff00".to_owned()];

        let svg = palette_svg_rs(colors.clone(), 20, 6).unwrap();
        assert!(svg.contains(r#"width="20" height="6""#));
        assert!(svg.contains(r##"fill="#ff0000""##));

        Python::initialize();
        Python::attach(|py| {
            let png = palette_png_rs(py, colors, 20, 6).unwrap();
            let bytes = png.bind(py).as_bytes();
            assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
            assert_eq!(u32::from_be_bytes(bytes[16..20].try_into().unwrap()), 20);
            assert_eq!(u32::from_be_bytes(bytes[20..24].try_into().unwrap()), 6);
        });
    }
}
