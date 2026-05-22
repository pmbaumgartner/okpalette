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

struct CommonPaletteBridgeArgs {
    seed_colors: Option<Vec<String>>,
    avoid_colors: Option<Vec<String>>,
    backgrounds: Option<Vec<String>>,
    background_contrast: Option<String>,
    lightness: Option<(f32, f32)>,
    chroma: Option<(Option<f32>, Option<f32>)>,
    hue: Option<(f32, f32)>,
    lightness_weight: f32,
    chroma_weight: f32,
    colorblind_mode: Option<String>,
}

struct CommonPaletteRequest {
    seed_colors: Vec<Rgb8>,
    avoid_colors: Vec<Rgb8>,
    backgrounds: Vec<Rgb8>,
    background_contrast: Option<String>,
    constraints: CandidateConstraints,
    weights: DistanceWeights,
    colorblind_mode: ColorblindMode,
}

impl CommonPaletteBridgeArgs {
    fn into_request(self) -> Result<CommonPaletteRequest, GlasbeyError> {
        Ok(CommonPaletteRequest {
            seed_colors: parse_hex_colors(self.seed_colors.unwrap_or_default())?,
            avoid_colors: parse_hex_colors(self.avoid_colors.unwrap_or_default())?,
            backgrounds: parse_hex_colors(self.backgrounds.unwrap_or_default())?,
            background_contrast: self.background_contrast,
            constraints: CandidateConstraints {
                lightness: self.lightness,
                chroma: self.chroma,
                hue: self.hue,
            },
            weights: DistanceWeights {
                lightness: self.lightness_weight,
                chroma: self.chroma_weight,
            },
            colorblind_mode: ColorblindMode::parse(self.colorblind_mode.as_deref())?,
        })
    }
}

impl CommonPaletteRequest {
    fn anchors(&self) -> PaletteAnchors<'_> {
        PaletteAnchors {
            seed_colors: &self.seed_colors,
            avoid_colors: &self.avoid_colors,
            backgrounds: &self.backgrounds,
        }
    }

    fn background_filter(&self) -> Result<BackgroundFilter<'_>, GlasbeyError> {
        background_filter_from_mode(
            &self.backgrounds,
            self.background_contrast.as_deref(),
            self.weights,
            self.colorblind_mode,
        )
    }
}

struct GeneratePaletteBridgeArgs {
    palette_size: usize,
    grid_step: u8,
    common: CommonPaletteBridgeArgs,
}

struct GeneratePaletteRequest {
    palette_size: usize,
    grid_size: GridSize,
    common: CommonPaletteRequest,
}

impl GeneratePaletteBridgeArgs {
    fn into_request(self) -> Result<GeneratePaletteRequest, GlasbeyError> {
        Ok(GeneratePaletteRequest {
            palette_size: self.palette_size,
            grid_size: GridSize::Step(self.grid_step),
            common: self.common.into_request()?,
        })
    }
}

struct GenerateLabelPaletteBridgeArgs {
    coordinates: Vec<f64>,
    dimension: usize,
    label_ids: Vec<usize>,
    label_count: usize,
    fixed_colors: Vec<Option<String>>,
    grid_step: u8,
    neighbors: usize,
    max_points: Option<usize>,
    common: CommonPaletteBridgeArgs,
}

struct GenerateLabelPaletteRequest {
    coordinates: Vec<f64>,
    dimension: usize,
    label_ids: Vec<usize>,
    label_count: usize,
    fixed_colors: Vec<Option<Rgb8>>,
    grid_size: GridSize,
    neighbors: usize,
    max_points: Option<usize>,
    common: CommonPaletteRequest,
}

impl GenerateLabelPaletteBridgeArgs {
    fn into_request(self) -> Result<GenerateLabelPaletteRequest, GlasbeyError> {
        let fixed_colors = self
            .fixed_colors
            .into_iter()
            .map(|color| color.as_deref().map(parse_hex_color).transpose())
            .collect::<Result<Vec<_>, _>>()?;

        Ok(GenerateLabelPaletteRequest {
            coordinates: self.coordinates,
            dimension: self.dimension,
            label_ids: self.label_ids,
            label_count: self.label_count,
            fixed_colors,
            grid_size: GridSize::Step(self.grid_step),
            neighbors: self.neighbors,
            max_points: self.max_points,
            common: self.common.into_request()?,
        })
    }
}

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
    let args = GeneratePaletteBridgeArgs {
        palette_size,
        grid_step,
        common: CommonPaletteBridgeArgs {
            seed_colors,
            avoid_colors,
            backgrounds,
            background_contrast,
            lightness,
            chroma,
            hue,
            lightness_weight,
            chroma_weight,
            colorblind_mode,
        },
    };

    py.detach(move || generate_palette_from_bridge_args(args))
        .map_err(to_py_value_error)
}

fn generate_palette_from_bridge_args(
    args: GeneratePaletteBridgeArgs,
) -> Result<Vec<String>, GlasbeyError> {
    generate_palette_inner(args.into_request()?)
}

fn generate_palette_inner(request: GeneratePaletteRequest) -> Result<Vec<String>, GlasbeyError> {
    let common = &request.common;
    common.weights.validate()?;
    let background_filter = common.background_filter()?;
    background_filter.validate_user_colors("seed_colors", &common.seed_colors)?;
    let candidates = generate_candidates_with_background_filter(
        request.grid_size,
        common.constraints,
        background_filter,
        request.palette_size,
    )?;
    let palette = select_palette(
        &candidates,
        PaletteOptions {
            palette_size: request.palette_size,
            anchors: common.anchors(),
            weights: common.weights,
            colorblind_mode: common.colorblind_mode,
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
    let args = GenerateLabelPaletteBridgeArgs {
        coordinates,
        dimension,
        label_ids,
        label_count,
        fixed_colors,
        grid_step,
        neighbors,
        max_points,
        common: CommonPaletteBridgeArgs {
            seed_colors,
            avoid_colors,
            backgrounds,
            background_contrast,
            lightness,
            chroma,
            hue,
            lightness_weight,
            chroma_weight,
            colorblind_mode,
        },
    };

    py.detach(move || generate_label_palette_from_bridge_args(args))
        .map_err(to_py_value_error)
}

fn generate_label_palette_from_bridge_args(
    args: GenerateLabelPaletteBridgeArgs,
) -> Result<Vec<String>, GlasbeyError> {
    generate_label_palette_inner(args.into_request()?)
}

fn generate_label_palette_inner(
    request: GenerateLabelPaletteRequest,
) -> Result<Vec<String>, GlasbeyError> {
    let common = &request.common;
    let background_filter = common.background_filter()?;
    background_filter.validate_user_colors("seed_colors", &common.seed_colors)?;
    let fixed_anchor_colors: Vec<Rgb8> = request.fixed_colors.iter().flatten().copied().collect();
    background_filter.validate_user_colors("fixed_colors", &fixed_anchor_colors)?;

    let palette = select_label_palette(LabelPaletteOptions {
        coordinates: &request.coordinates,
        dimension: request.dimension,
        label_ids: &request.label_ids,
        label_count: request.label_count,
        fixed_colors: &request.fixed_colors,
        constraints: common.constraints,
        background_filter,
        grid_size: request.grid_size,
        anchors: common.anchors(),
        weights: common.weights,
        colorblind_mode: common.colorblind_mode,
        neighbors: request.neighbors,
        max_points: request.max_points,
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

    fn common_args() -> CommonPaletteBridgeArgs {
        CommonPaletteBridgeArgs {
            seed_colors: None,
            avoid_colors: None,
            backgrounds: None,
            background_contrast: None,
            lightness: None,
            chroma: None,
            hue: None,
            lightness_weight: 1.0,
            chroma_weight: 1.0,
            colorblind_mode: None,
        }
    }

    fn palette_args(palette_size: usize) -> GeneratePaletteBridgeArgs {
        GeneratePaletteBridgeArgs {
            palette_size,
            grid_step: 8,
            common: common_args(),
        }
    }

    #[test]
    fn native_bridge_generates_canonical_hex_palette() {
        let palette = generate_palette_from_bridge_args(GeneratePaletteBridgeArgs {
            palette_size: 3,
            grid_step: 64,
            common: CommonPaletteBridgeArgs {
                seed_colors: Some(vec!["#f00".to_owned()]),
                backgrounds: Some(vec!["#fff".to_owned()]),
                background_contrast: Some("normal".to_owned()),
                lightness: Some((0.2, 0.9)),
                chroma: Some((Some(0.04), None)),
                ..common_args()
            },
        })
        .unwrap();

        assert_canonical_hex_palette(&palette, 3);
        assert!(!palette.contains(&"#ff0000".to_owned()));
        assert!(!palette.contains(&"#ffffff".to_owned()));
    }

    #[test]
    fn native_bridge_maps_engine_errors() {
        let error = generate_palette_from_bridge_args(GeneratePaletteBridgeArgs {
            common: CommonPaletteBridgeArgs {
                seed_colors: Some(vec!["not-a-color".to_owned()]),
                ..common_args()
            },
            ..palette_args(1)
        })
        .unwrap_err();

        assert!(matches!(error, GlasbeyError::InvalidHexLength { .. }));
    }

    #[test]
    fn native_bridge_reports_insufficient_candidates() {
        let error = generate_palette_from_bridge_args(GeneratePaletteBridgeArgs {
            palette_size: 9,
            grid_step: 255,
            common: common_args(),
        })
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
        let error = generate_palette_from_bridge_args(GeneratePaletteBridgeArgs {
            grid_step: 255,
            common: CommonPaletteBridgeArgs {
                colorblind_mode: Some("protanopia".to_owned()),
                ..common_args()
            },
            ..palette_args(1)
        })
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
        let error = generate_palette_from_bridge_args(GeneratePaletteBridgeArgs {
            grid_step: 255,
            common: CommonPaletteBridgeArgs {
                seed_colors: Some(vec!["#ffffff".to_owned()]),
                backgrounds: Some(vec!["#ffffff".to_owned()]),
                background_contrast: Some("high".to_owned()),
                ..common_args()
            },
            ..palette_args(1)
        })
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
    fn native_bridge_generates_label_palette_with_fixed_colors() {
        let palette = generate_label_palette_from_bridge_args(GenerateLabelPaletteBridgeArgs {
            coordinates: vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0],
            dimension: 2,
            label_ids: vec![0, 1, 2],
            label_count: 3,
            fixed_colors: vec![None, Some("#ff0000".to_owned()), None],
            grid_step: 64,
            neighbors: 2,
            max_points: Some(10),
            common: common_args(),
        })
        .unwrap();

        assert_canonical_hex_palette(&palette, 3);
        assert_eq!(palette[1], "#ff0000");
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
