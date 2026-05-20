use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum GlasbeyError {
    #[error(
        "invalid hex color length {length}; expected 3 or 6 hex digits with optional leading '#'"
    )]
    InvalidHexLength { length: usize },

    #[error("invalid hex digit '{ch}' at byte {index}; expected ASCII hexadecimal digit")]
    InvalidHexDigit { index: usize, ch: char },

    #[error("invalid grid step 0; grid_step must be greater than 0")]
    InvalidGridStep,

    #[error("invalid {constraint} constraint: {message}")]
    InvalidConstraintRange {
        constraint: &'static str,
        message: &'static str,
    },

    #[error("invalid distance weights: {message}")]
    InvalidDistanceWeights { message: &'static str },

    #[error("invalid label palette input: {message}")]
    InvalidLabelPaletteInput { message: &'static str },

    #[error(
        "only {available} candidate colors remain after applying constraints, but palette_size={requested} was requested. Try relaxing lightness, chroma, hue, background_contrast, or grid_size."
    )]
    InsufficientCandidates { available: usize, requested: usize },

    #[error(
        "{role} color {color} has WCAG contrast ratio {ratio:.2}:1 against background {background}, below required {required:.1}:1"
    )]
    InsufficientBackgroundContrast {
        role: &'static str,
        color: String,
        background: String,
        ratio: f64,
        required: f64,
    },

    #[error("invalid palette render request: {message}")]
    InvalidRenderRequest { message: &'static str },

    #[error("failed to encode PNG palette preview: {message}")]
    PngEncoding { message: String },
}

pub type Result<T> = std::result::Result<T, GlasbeyError>;
