#![warn(missing_docs)]

//! Fast, deterministic categorical color palette generation.
//!
//! Use [`generate_palette`] and [`extend_palette`] for the common defaults, or
//! configure a reusable [`PaletteGenerator`]. Colors are returned as [`Rgb8`]
//! values so callers can choose their own serialization or rendering format.
//!
//! ```no_run
//! use okpalette::{BackgroundContrast, PaletteGenerator, Rgb8};
//!
//! let palette = PaletteGenerator::new()
//!     .backgrounds([Rgb8::new(255, 255, 255)], BackgroundContrast::Normal)
//!     ?
//!     .generate(8)?;
//! # Ok::<(), okpalette::OkPaletteError>(())
//! ```

mod algorithm;
mod api;
mod candidates;
mod color;
mod distance;
mod error;
mod label;
mod parse;
#[cfg(feature = "python")]
mod python;
mod render;
#[cfg(test)]
pub(crate) mod test_support;

pub use api::{
    extend_palette, generate_palette, BackgroundContrast, LabelPaletteRequest, PaletteGenerator,
};
pub use candidates::{CandidateConstraints, ChromaRange, GridSize, HueRange, LightnessRange};
pub use color::{ColorblindMode, Oklab, Oklch, Rgb8};
pub use distance::DistanceWeights;
pub use error::{GlasbeyError, OkPaletteError, Result};
pub use parse::parse_hex_color;
pub use render::{render_palette_png, render_palette_svg};
