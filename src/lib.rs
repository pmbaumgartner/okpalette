use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

pub mod algorithm;
pub mod candidates;
pub mod color;
pub mod error;
pub mod label;
pub mod parse;
pub mod render;
#[cfg(test)]
pub(crate) mod test_support;

use algorithm::{select_palette, DistanceWeights, PaletteAnchors, PaletteOptions};
use candidates::{
    generate_candidates_with_background_filter, BackgroundFilter, CandidateConstraints, GridSize,
    NORMAL_BACKGROUND_DISTANCE_SQUARED, WCAG_NON_TEXT_CONTRAST_RATIO,
};
use color::{ColorblindMode, Rgb8};
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
    background_contrast = None,
    lightness = None,
    chroma = None,
    hue = None,
    grid_step = 8,
    lightness_weight = 1.0,
    chroma_weight = 1.0,
    colorblind_mode = None,
))]
#[allow(clippy::too_many_arguments)]
fn generate_palette_rs(
    py: Python<'_>,
    palette_size: usize,
    seed_colors: Option<Vec<String>>,
    avoid_colors: Option<Vec<String>>,
    backgrounds: Option<Vec<String>>,
    background_contrast: Option<String>,
    lightness: Option<(f32, f32)>,
    chroma: Option<(Option<f32>, Option<f32>)>,
    hue: Option<(f32, f32)>,
    grid_step: u8,
    lightness_weight: f32,
    chroma_weight: f32,
    colorblind_mode: Option<String>,
) -> PyResult<Vec<String>> {
    py.detach(move || {
        generate_palette_inner(
            palette_size,
            seed_colors,
            avoid_colors,
            backgrounds,
            background_contrast,
            lightness,
            chroma,
            hue,
            grid_step,
            lightness_weight,
            chroma_weight,
            colorblind_mode,
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
    background_contrast: Option<String>,
    lightness: Option<(f32, f32)>,
    chroma: Option<(Option<f32>, Option<f32>)>,
    hue: Option<(f32, f32)>,
    grid_step: u8,
    lightness_weight: f32,
    chroma_weight: f32,
    colorblind_mode: Option<String>,
) -> Result<Vec<String>, GlasbeyError> {
    let seed_colors = parse_hex_colors(seed_colors.unwrap_or_default())?;
    let avoid_colors = parse_hex_colors(avoid_colors.unwrap_or_default())?;
    let backgrounds = parse_hex_colors(backgrounds.unwrap_or_default())?;
    let colorblind_mode = ColorblindMode::parse(colorblind_mode.as_deref())?;
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
    let background_filter = background_filter_from_mode(
        &backgrounds,
        background_contrast.as_deref(),
        weights,
        colorblind_mode,
    )?;
    background_filter.validate_user_colors("seed_colors", &seed_colors)?;
    let candidates = generate_candidates_with_background_filter(
        GridSize::Step(grid_step),
        constraints,
        background_filter,
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
            colorblind_mode,
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
    background_contrast = None,
    lightness = None,
    chroma = None,
    hue = None,
    grid_step = 8,
    lightness_weight = 1.0,
    chroma_weight = 1.0,
    colorblind_mode = None,
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
    background_contrast: Option<String>,
    lightness: Option<(f32, f32)>,
    chroma: Option<(Option<f32>, Option<f32>)>,
    hue: Option<(f32, f32)>,
    grid_step: u8,
    lightness_weight: f32,
    chroma_weight: f32,
    colorblind_mode: Option<String>,
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
            background_contrast,
            lightness,
            chroma,
            hue,
            grid_step,
            lightness_weight,
            chroma_weight,
            colorblind_mode,
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
    background_contrast: Option<String>,
    lightness: Option<(f32, f32)>,
    chroma: Option<(Option<f32>, Option<f32>)>,
    hue: Option<(f32, f32)>,
    grid_step: u8,
    lightness_weight: f32,
    chroma_weight: f32,
    colorblind_mode: Option<String>,
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
    let colorblind_mode = ColorblindMode::parse(colorblind_mode.as_deref())?;
    let constraints = CandidateConstraints {
        lightness,
        chroma,
        hue,
    };
    let weights = DistanceWeights {
        lightness: lightness_weight,
        chroma: chroma_weight,
    };
    let background_filter = background_filter_from_mode(
        &backgrounds,
        background_contrast.as_deref(),
        weights,
        colorblind_mode,
    )?;
    background_filter.validate_user_colors("seed_colors", &seed_colors)?;
    let fixed_anchor_colors: Vec<Rgb8> = fixed_colors.iter().flatten().copied().collect();
    background_filter.validate_user_colors("fixed_colors", &fixed_anchor_colors)?;

    let palette = select_label_palette(LabelPaletteOptions {
        coordinates: &coordinates,
        dimension,
        label_ids: &label_ids,
        label_count,
        fixed_colors: &fixed_colors,
        constraints,
        background_filter,
        grid_size: GridSize::Step(grid_step),
        anchors: PaletteAnchors {
            seed_colors: &seed_colors,
            avoid_colors: &avoid_colors,
            backgrounds: &backgrounds,
        },
        weights,
        colorblind_mode,
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

fn background_filter_from_mode<'a>(
    backgrounds: &'a [Rgb8],
    background_contrast: Option<&str>,
    weights: DistanceWeights,
    colorblind_mode: ColorblindMode,
) -> Result<BackgroundFilter<'a>, GlasbeyError> {
    match (backgrounds.is_empty(), background_contrast) {
        (true, None) => Ok(BackgroundFilter::None),
        (false, None) => Err(GlasbeyError::InvalidConstraintRange {
            constraint: "background_contrast",
            message: "must be provided when background is set",
        }),
        (true, Some(_)) => Err(GlasbeyError::InvalidConstraintRange {
            constraint: "background",
            message: "must contain at least one color when background_contrast is set",
        }),
        (false, Some("normal")) => Ok(BackgroundFilter::NormalOklabDistance {
            backgrounds,
            min_distance_squared: NORMAL_BACKGROUND_DISTANCE_SQUARED,
            weights,
            colorblind_mode,
        }),
        (false, Some("high" | "wcag")) => Ok(BackgroundFilter::WcagNonTextContrast {
            backgrounds,
            min_ratio: WCAG_NON_TEXT_CONTRAST_RATIO,
        }),
        (false, Some(_)) => Err(GlasbeyError::InvalidConstraintRange {
            constraint: "background_contrast",
            message: "must be 'normal', 'high', 'wcag', or None",
        }),
    }
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
    use crate::test_support::{assert_canonical_hex_palette, assert_png_dimensions};

    #[test]
    fn native_bridge_generates_canonical_hex_palette() {
        let palette = generate_palette_inner(
            3,
            Some(vec!["#f00".to_owned()]),
            None,
            Some(vec!["#fff".to_owned()]),
            Some("normal".to_owned()),
            Some((0.2, 0.9)),
            Some((Some(0.04), None)),
            None,
            64,
            1.0,
            1.0,
            None,
        )
        .unwrap();

        assert_canonical_hex_palette(&palette, 3);
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
            None,
        )
        .unwrap_err();

        assert!(matches!(error, GlasbeyError::InvalidHexLength { .. }));
    }

    #[test]
    fn native_bridge_reports_insufficient_candidates() {
        let error = generate_palette_inner(
            9, None, None, None, None, None, None, None, 255, 1.0, 1.0, None,
        )
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
    fn native_bridge_rejects_invalid_colorblind_mode() {
        let error = generate_palette_inner(
            1,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            255,
            1.0,
            1.0,
            Some("protanopia".to_owned()),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            GlasbeyError::InvalidConstraintRange {
                constraint: "colorblind_mode",
                ..
            }
        ));
    }

    #[test]
    fn native_bridge_rejects_high_contrast_seed_failures() {
        let error = generate_palette_inner(
            1,
            Some(vec!["#ffffff".to_owned()]),
            None,
            Some(vec!["#ffffff".to_owned()]),
            Some("high".to_owned()),
            None,
            None,
            None,
            255,
            1.0,
            1.0,
            None,
        )
        .unwrap_err();

        assert!(matches!(
            error,
            GlasbeyError::InsufficientBackgroundContrast {
                role: "seed_colors",
                ..
            }
        ));
        assert!(error.to_string().contains("#ffffff"));
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
            assert_png_dimensions(bytes, 20, 6);
        });
    }
}
