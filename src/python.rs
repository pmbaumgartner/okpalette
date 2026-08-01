use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;

use crate::{
    parse_hex_color, render_palette_png, render_palette_svg, BackgroundContrast,
    CandidateConstraints, ChromaRange, DistanceWeights, GlasbeyError, GridSize, HueRange,
    LabelPaletteRequest, LightnessRange, PaletteGenerator, Rgb8,
};

#[pyclass(name = "_PaletteGenerator")]
#[derive(Debug)]
struct PyPaletteGenerator {
    generator: PaletteGenerator,
}

#[pymethods]
impl PyPaletteGenerator {
    #[new]
    #[allow(clippy::too_many_arguments)]
    fn new(
        seed_colors: Vec<String>,
        avoid_colors: Vec<String>,
        backgrounds: Option<Vec<String>>,
        background_contrast: Option<String>,
        lightness: Option<(f32, f32)>,
        chroma: Option<(Option<f32>, Option<f32>)>,
        hue: Option<(f32, f32)>,
        grid_step: i64,
        lightness_weight: f32,
        chroma_weight: f32,
        colorblind_mode: Option<String>,
    ) -> PyResult<Self> {
        let seed_colors = parse_hex_colors(seed_colors).map_err(to_py_value_error)?;
        let avoid_colors = parse_hex_colors(avoid_colors).map_err(to_py_value_error)?;
        let backgrounds = backgrounds
            .map(parse_hex_colors)
            .transpose()
            .map_err(to_py_value_error)?;
        let background_contrast =
            parse_background_contrast(backgrounds.as_deref(), background_contrast.as_deref())?;
        let constraints = parse_constraints(lightness, chroma, hue).map_err(to_py_value_error)?;
        let grid_size = parse_grid_size(grid_step).map_err(to_py_value_error)?;
        let weights =
            DistanceWeights::new(lightness_weight, chroma_weight).map_err(to_py_value_error)?;
        let colorblind_mode = colorblind_mode
            .as_deref()
            .map(str::parse)
            .transpose()
            .map_err(to_py_value_error)?
            .unwrap_or_default();

        let mut generator = PaletteGenerator::new()
            .seed_colors(seed_colors)
            .avoid_colors(avoid_colors)
            .constraints(constraints)
            .grid_size(grid_size)
            .distance_weights(weights)
            .colorblind_mode(colorblind_mode);
        if let Some(contrast) = background_contrast {
            generator = generator.backgrounds(
                backgrounds.expect("validated backgrounds are present"),
                contrast,
            );
        }

        Ok(Self { generator })
    }

    fn generate(&self, py: Python<'_>, palette_size: usize) -> PyResult<Vec<String>> {
        let generator = self.generator.clone();
        py.detach(move || generator.generate(palette_size))
            .map(palette_to_hex)
            .map_err(to_py_value_error)
    }

    fn extend(
        &self,
        py: Python<'_>,
        colors: Vec<String>,
        target_size: usize,
    ) -> PyResult<Vec<String>> {
        let generator = self.generator.clone();
        py.detach(move || {
            let colors = parse_hex_colors(colors)?;
            generator.extend(&colors, target_size)
        })
        .map(palette_to_hex)
        .map_err(to_py_value_error)
    }

    fn generate_extension(
        &self,
        py: Python<'_>,
        colors: Vec<String>,
        count: usize,
    ) -> PyResult<Vec<String>> {
        let generator = self.generator.clone();
        py.detach(move || {
            let colors = parse_hex_colors(colors)?;
            generator.generate_extension(&colors, count)
        })
        .map(palette_to_hex)
        .map_err(to_py_value_error)
    }

    #[allow(clippy::too_many_arguments)]
    fn generate_for_labels(
        &self,
        py: Python<'_>,
        coordinates: Vec<f64>,
        dimension: usize,
        label_ids: Vec<usize>,
        label_count: usize,
        fixed_colors: Vec<Option<String>>,
        neighbors: usize,
        max_points: Option<usize>,
    ) -> PyResult<Vec<String>> {
        let generator = self.generator.clone();
        py.detach(move || {
            let fixed_colors = fixed_colors
                .iter()
                .map(|color| color.as_deref().map(parse_hex_color).transpose())
                .collect::<Result<Vec<_>, _>>()?;
            let request =
                LabelPaletteRequest::new(&coordinates, dimension, &label_ids, label_count)
                    .fixed_colors(&fixed_colors)
                    .neighbors(neighbors)
                    .max_points(max_points);
            generator.generate_for_labels(request)
        })
        .map(palette_to_hex)
        .map_err(to_py_value_error)
    }
}

fn parse_hex_colors(colors: Vec<String>) -> Result<Vec<Rgb8>, GlasbeyError> {
    colors.iter().map(|color| parse_hex_color(color)).collect()
}

fn palette_to_hex(palette: Vec<Rgb8>) -> Vec<String> {
    palette.into_iter().map(Rgb8::to_hex).collect()
}

fn parse_constraints(
    lightness: Option<(f32, f32)>,
    chroma: Option<(Option<f32>, Option<f32>)>,
    hue: Option<(f32, f32)>,
) -> Result<CandidateConstraints, GlasbeyError> {
    let mut constraints = CandidateConstraints::new();
    if let Some((minimum, maximum)) = lightness {
        constraints = constraints.with_lightness(LightnessRange::new(minimum, maximum)?);
    }
    if let Some((minimum, maximum)) = chroma {
        constraints = constraints.with_chroma(ChromaRange::new(minimum, maximum)?);
    }
    if let Some((start, end)) = hue {
        constraints = constraints.with_hue(HueRange::new(start, end)?);
    }
    Ok(constraints)
}

fn parse_grid_size(grid_step: i64) -> Result<GridSize, GlasbeyError> {
    u8::try_from(grid_step)
        .ok()
        .filter(|&step| step > 0)
        .map(GridSize::Step)
        .ok_or(GlasbeyError::InvalidConstraintRange {
            constraint: "grid_size",
            message: "must be an integer in 1..=255",
        })
}

fn parse_background_contrast(
    backgrounds: Option<&[Rgb8]>,
    value: Option<&str>,
) -> PyResult<Option<BackgroundContrast>> {
    match (backgrounds, value) {
        (None, None) => Ok(None),
        (Some(_), None) => Err(PyValueError::new_err(
            "background_contrast must be provided when background is set",
        )),
        (None, Some(_)) => Err(PyValueError::new_err(
            "background must be provided when background_contrast is set",
        )),
        (Some([]), Some(_)) => Err(PyValueError::new_err(
            "background must contain at least one color",
        )),
        (Some(_), Some("normal")) => Ok(Some(BackgroundContrast::Normal)),
        (Some(_), Some("high" | "wcag")) => Ok(Some(BackgroundContrast::Wcag)),
        (Some(_), Some(_)) => Err(PyValueError::new_err(
            "background_contrast must be 'normal', 'high', 'wcag', or None",
        )),
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
#[pyo3(name = "_core")]
fn python_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyPaletteGenerator>()?;
    m.add_function(wrap_pyfunction!(palette_svg_rs, m)?)?;
    m.add_function(wrap_pyfunction!(palette_png_rs, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{assert_canonical_hex_palette, assert_png_dimensions};

    fn run_with_python<T>(operation: impl for<'py> FnOnce(Python<'py>) -> T) -> T {
        Python::initialize();
        Python::attach(operation)
    }

    fn native_generator(
        seed_colors: Vec<String>,
        backgrounds: Option<Vec<String>>,
        background_contrast: Option<String>,
        grid_step: i64,
        colorblind_mode: Option<String>,
    ) -> PyResult<PyPaletteGenerator> {
        PyPaletteGenerator::new(
            seed_colors,
            Vec::new(),
            backgrounds,
            background_contrast,
            None,
            None,
            None,
            grid_step,
            1.0,
            1.0,
            colorblind_mode,
        )
    }

    #[test]
    fn native_bridge_generates_canonical_hex_palette() {
        let generator = native_generator(
            vec!["#f00".to_owned()],
            Some(vec!["#fff".to_owned()]),
            Some("normal".to_owned()),
            64,
            None,
        )
        .unwrap();
        let palette = run_with_python(|py| generator.generate(py, 3)).unwrap();

        assert_canonical_hex_palette(&palette, 3);
        assert!(!palette.contains(&"#ff0000".to_owned()));
        assert!(!palette.contains(&"#ffffff".to_owned()));
    }

    #[test]
    fn native_bridge_maps_engine_errors() {
        let error =
            native_generator(vec!["not-a-color".to_owned()], None, None, 64, None).unwrap_err();

        assert!(error.to_string().contains("invalid hex color length"));
    }

    #[test]
    fn native_bridge_reports_insufficient_candidates() {
        let generator = native_generator(Vec::new(), None, None, 255, None).unwrap();
        let error = run_with_python(|py| generator.generate(py, 9)).unwrap_err();

        assert!(error.to_string().contains("only 8 candidate colors"));
    }

    #[test]
    fn native_bridge_rejects_invalid_colorblind_mode() {
        let error = native_generator(Vec::new(), None, None, 64, Some("protanopia".to_owned()))
            .unwrap_err();

        assert!(error.to_string().contains("colorblind_mode"));
    }

    #[test]
    fn native_bridge_rejects_high_contrast_seed_failures() {
        let generator = native_generator(
            vec!["#ffffff".to_owned()],
            Some(vec!["#ffffff".to_owned()]),
            Some("high".to_owned()),
            255,
            None,
        )
        .unwrap();
        let error = run_with_python(|py| generator.generate(py, 1)).unwrap_err();

        assert!(error.to_string().contains("#ffffff"));
    }

    #[test]
    fn native_bridge_generates_label_palette_with_fixed_colors() {
        let generator = native_generator(Vec::new(), None, None, 64, None).unwrap();
        let palette = run_with_python(|py| {
            generator.generate_for_labels(
                py,
                vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0],
                2,
                vec![0, 1, 2],
                3,
                vec![None, Some("#ff0000".to_owned()), None],
                2,
                Some(10),
            )
        })
        .unwrap();

        assert_canonical_hex_palette(&palette, 3);
        assert_eq!(palette[1], "#ff0000");
    }

    #[test]
    fn native_bridge_uses_canonical_extension_methods() {
        let generator = native_generator(Vec::new(), None, None, 64, None).unwrap();
        let existing = vec!["#ff0000".to_owned()];

        let extended = run_with_python(|py| generator.extend(py, existing.clone(), 3)).unwrap();
        let generated =
            run_with_python(|py| generator.generate_extension(py, existing, 2)).unwrap();

        assert_eq!(extended[0], "#ff0000");
        assert_eq!(&extended[1..], generated);
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
