use pyo3::exceptions::PyNotImplementedError;
use pyo3::prelude::*;

const SCAFFOLD_MESSAGE: &str =
    "generate_palette_rs is scaffolded; palette generation is implemented in follow-up issues";

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
    let _ = (
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
    );

    Err(PyNotImplementedError::new_err(SCAFFOLD_MESSAGE))
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
    fn scaffold_message_names_follow_up_work() {
        assert!(SCAFFOLD_MESSAGE.contains("scaffolded"));
        assert!(SCAFFOLD_MESSAGE.contains("follow-up issues"));
    }
}
