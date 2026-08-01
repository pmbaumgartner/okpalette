use thiserror::Error;

/// Errors returned while validating inputs, generating palettes, or rendering previews.
#[non_exhaustive]
#[derive(Debug, Error, PartialEq)]
pub enum OkPaletteError {
    /// A hexadecimal color had neither three nor six digits.
    #[error(
        "invalid hex color length {length}; expected 3 or 6 hex digits with optional leading '#'"
    )]
    InvalidHexLength {
        /// Number of hexadecimal digits supplied after an optional `#`.
        length: usize,
    },

    /// A hexadecimal color contained a non-ASCII-hexadecimal character.
    #[error("invalid hex digit '{ch}' at byte {index}; expected ASCII hexadecimal digit")]
    InvalidHexDigit {
        /// Byte index of the invalid character after an optional `#`.
        index: usize,
        /// Invalid character.
        ch: char,
    },

    /// A custom RGB grid step was zero.
    #[error("invalid grid step 0; grid_step must be greater than 0")]
    InvalidGridStep,

    /// A palette constraint was malformed or outside its valid range.
    #[error("invalid {constraint} constraint: {message}")]
    InvalidConstraintRange {
        /// Name of the invalid constraint.
        constraint: &'static str,
        /// Explanation of the required value.
        message: &'static str,
    },

    /// The lightness or chroma distance weights were invalid.
    #[error("invalid distance weights: {message}")]
    InvalidDistanceWeights {
        /// Explanation of the required weights.
        message: &'static str,
    },

    /// Position-aware label input was inconsistent or malformed.
    #[error("invalid label palette input: {message}")]
    InvalidLabelPaletteInput {
        /// Explanation of the required input.
        message: &'static str,
    },

    /// Too few candidate colors remained after constraints and exclusions.
    #[error(
        "only {available} candidate colors remain after applying constraints, but palette_size={requested} was requested. Try relaxing lightness, chroma, hue, background_contrast, or grid_size."
    )]
    InsufficientCandidates {
        /// Number of candidate colors that remained.
        available: usize,
        /// Number of new colors requested.
        requested: usize,
    },

    /// A user-supplied color failed the configured background contrast requirement.
    #[error(
        "{role} color {color} has WCAG contrast ratio {ratio:.2}:1 against background {background}, below required {required:.1}:1"
    )]
    InsufficientBackgroundContrast {
        /// Input collection containing the failing color.
        role: &'static str,
        /// Failing color in hexadecimal notation.
        color: String,
        /// Background color in hexadecimal notation.
        background: String,
        /// Observed WCAG contrast ratio.
        ratio: f64,
        /// Required WCAG contrast ratio.
        required: f64,
    },

    /// Palette preview dimensions or colors were invalid.
    #[error("invalid palette render request: {message}")]
    InvalidRenderRequest {
        /// Explanation of the required render input.
        message: &'static str,
    },

    /// The PNG encoder failed while writing a preview.
    #[error("failed to encode PNG palette preview: {message}")]
    PngEncoding {
        /// Encoder error message.
        message: String,
    },
}

/// Result type returned by `okpalette` operations.
pub type Result<T> = std::result::Result<T, OkPaletteError>;

/// Backward-compatible name for [`OkPaletteError`].
///
/// New code should use [`OkPaletteError`], which matches the crate name.
pub type GlasbeyError = OkPaletteError;
