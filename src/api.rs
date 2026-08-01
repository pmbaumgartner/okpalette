use crate::algorithm::{select_palette, PaletteAnchors, PaletteOptions};
use crate::candidates::{
    generate_candidates_with_background_filter, BackgroundFilter, CandidateConstraints, GridSize,
    NORMAL_BACKGROUND_DISTANCE_SQUARED, WCAG_NON_TEXT_CONTRAST_RATIO,
};
use crate::color::{ColorblindMode, Rgb8};
use crate::distance::DistanceWeights;
use crate::error::{GlasbeyError, Result};
use crate::label::{select_label_palette, LabelPaletteOptions};

/// The strategy used to keep generated colors distinct from a background.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundContrast {
    /// Use the same perceptual OKLab separation heuristic as Python's `"normal"` mode.
    Normal,
    /// Require the WCAG 2.2 non-text contrast ratio of at least 3:1.
    Wcag,
}

/// Configuration for generating palettes.
///
/// The defaults match `okpalette.create_palette()` in Python: a medium RGB grid,
/// lightness from 0.20 to 0.90, and chroma of at least 0.04.
#[derive(Debug, Clone, PartialEq)]
pub struct PaletteGenerator {
    seed_colors: Vec<Rgb8>,
    avoid_colors: Vec<Rgb8>,
    background: Option<(Vec<Rgb8>, BackgroundContrast)>,
    constraints: CandidateConstraints,
    grid_size: GridSize,
    weights: DistanceWeights,
    colorblind_mode: ColorblindMode,
}

/// Input for position-aware label palette generation.
///
/// `coordinates` is a flattened array containing one `dimension`-wide point per
/// label ID. Label IDs must be in `0..label_count`.
#[derive(Debug, Clone, Copy)]
pub struct LabelPaletteRequest<'a> {
    coordinates: &'a [f64],
    dimension: usize,
    label_ids: &'a [usize],
    label_count: usize,
    fixed_colors: Option<&'a [Option<Rgb8>]>,
    neighbors: usize,
    max_points: Option<usize>,
}

impl Default for PaletteGenerator {
    fn default() -> Self {
        Self {
            seed_colors: Vec::new(),
            avoid_colors: Vec::new(),
            background: None,
            constraints: CandidateConstraints::palette_defaults(),
            grid_size: GridSize::Medium,
            weights: DistanceWeights::default(),
            colorblind_mode: ColorblindMode::None,
        }
    }
}

impl PaletteGenerator {
    /// Create a generator with the same defaults as the Python API and CLI.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the seed colors used as distance anchors.
    #[must_use]
    pub fn seed_colors(mut self, colors: impl Into<Vec<Rgb8>>) -> Self {
        self.seed_colors = colors.into();
        self
    }

    /// Replace the colors that must be excluded and used as distance anchors.
    #[must_use]
    pub fn avoid_colors(mut self, colors: impl Into<Vec<Rgb8>>) -> Self {
        self.avoid_colors = colors.into();
        self
    }

    /// Keep generated colors distinct from every supplied background color.
    #[must_use]
    pub fn backgrounds(
        mut self,
        colors: impl Into<Vec<Rgb8>>,
        contrast: BackgroundContrast,
    ) -> Self {
        self.background = Some((colors.into(), contrast));
        self
    }

    /// Replace all candidate color constraints.
    #[must_use]
    pub fn constraints(mut self, constraints: CandidateConstraints) -> Self {
        self.constraints = constraints;
        self
    }

    /// Set the RGB candidate grid resolution.
    #[must_use]
    pub fn grid_size(mut self, grid_size: GridSize) -> Self {
        self.grid_size = grid_size;
        self
    }

    /// Set the relative OKLab lightness and chroma distance weights.
    #[must_use]
    pub fn distance_weights(mut self, weights: DistanceWeights) -> Self {
        self.weights = weights;
        self
    }

    /// Score colors under the selected color-vision-deficiency simulations.
    #[must_use]
    pub fn colorblind_mode(mut self, mode: ColorblindMode) -> Self {
        self.colorblind_mode = mode;
        self
    }

    /// Generate `palette_size` new colors.
    pub fn generate(&self, palette_size: usize) -> Result<Vec<Rgb8>> {
        self.generate_with_extra_seeds(palette_size, &[])
    }

    /// Extend `colors` to `target_size`, returning the existing colors first.
    pub fn extend(&self, colors: &[Rgb8], target_size: usize) -> Result<Vec<Rgb8>> {
        let generated_count =
            target_size
                .checked_sub(colors.len())
                .ok_or(GlasbeyError::InvalidConstraintRange {
                    constraint: "target_size",
                    message: "must be greater than or equal to the existing palette length",
                })?;
        let generated = self.generate_with_extra_seeds(generated_count, colors)?;
        let mut palette = Vec::with_capacity(target_size);
        palette.extend_from_slice(colors);
        palette.extend(generated);
        Ok(palette)
    }

    /// Generate `count` new colors using `colors` as anchors without returning them.
    pub fn generate_extension(&self, colors: &[Rgb8], count: usize) -> Result<Vec<Rgb8>> {
        self.generate_with_extra_seeds(count, colors)
    }

    /// Generate a position-aware color for every label ID in `request`.
    pub fn generate_for_labels(&self, request: LabelPaletteRequest<'_>) -> Result<Vec<Rgb8>> {
        let default_fixed_colors;
        let fixed_colors = match request.fixed_colors {
            Some(colors) => colors,
            None => {
                default_fixed_colors = vec![None; request.label_count];
                &default_fixed_colors
            }
        };
        let backgrounds = self.backgrounds_slice();

        select_label_palette(LabelPaletteOptions {
            coordinates: request.coordinates,
            dimension: request.dimension,
            label_ids: request.label_ids,
            label_count: request.label_count,
            fixed_colors,
            constraints: self.constraints,
            background_filter: self.background_filter()?,
            grid_size: self.grid_size,
            anchors: PaletteAnchors {
                seed_colors: &self.seed_colors,
                avoid_colors: &self.avoid_colors,
                backgrounds,
            },
            weights: self.weights,
            colorblind_mode: self.colorblind_mode,
            neighbors: request.neighbors,
            max_points: request.max_points,
        })
    }

    fn generate_with_extra_seeds(
        &self,
        palette_size: usize,
        extra_seed_colors: &[Rgb8],
    ) -> Result<Vec<Rgb8>> {
        let seed_colors;
        let seeds = if extra_seed_colors.is_empty() {
            self.seed_colors.as_slice()
        } else {
            seed_colors = self
                .seed_colors
                .iter()
                .chain(extra_seed_colors)
                .copied()
                .collect::<Vec<_>>();
            seed_colors.as_slice()
        };
        let background_filter = self.background_filter()?;
        background_filter.validate_user_colors("seed_colors", seeds)?;
        let candidates = generate_candidates_with_background_filter(
            self.grid_size,
            self.constraints,
            background_filter,
            palette_size,
        )?;
        let palette = select_palette(
            &candidates,
            PaletteOptions {
                palette_size,
                anchors: PaletteAnchors {
                    seed_colors: seeds,
                    avoid_colors: &self.avoid_colors,
                    backgrounds: self.backgrounds_slice(),
                },
                weights: self.weights,
                colorblind_mode: self.colorblind_mode,
            },
        )?;

        Ok(palette)
    }

    fn backgrounds_slice(&self) -> &[Rgb8] {
        self.background
            .as_ref()
            .map_or(&[], |(colors, _)| colors.as_slice())
    }

    fn background_filter(&self) -> Result<BackgroundFilter<'_>> {
        let Some((backgrounds, contrast)) = &self.background else {
            return Ok(BackgroundFilter::None);
        };
        if backgrounds.is_empty() {
            return Err(GlasbeyError::InvalidConstraintRange {
                constraint: "background",
                message: "must contain at least one color",
            });
        }

        Ok(match contrast {
            BackgroundContrast::Normal => BackgroundFilter::NormalOklabDistance {
                backgrounds,
                min_distance_squared: NORMAL_BACKGROUND_DISTANCE_SQUARED,
                weights: self.weights,
                colorblind_mode: self.colorblind_mode,
            },
            BackgroundContrast::Wcag => BackgroundFilter::WcagNonTextContrast {
                backgrounds,
                min_ratio: WCAG_NON_TEXT_CONTRAST_RATIO,
            },
        })
    }
}

impl<'a> LabelPaletteRequest<'a> {
    /// Create a label request using eight neighbors and at most 50,000 points.
    #[must_use]
    pub fn new(
        coordinates: &'a [f64],
        dimension: usize,
        label_ids: &'a [usize],
        label_count: usize,
    ) -> Self {
        Self {
            coordinates,
            dimension,
            label_ids,
            label_count,
            fixed_colors: None,
            neighbors: 8,
            max_points: Some(50_000),
        }
    }

    /// Keep the supplied colors fixed by label ID; the slice must have `label_count` items.
    #[must_use]
    pub fn fixed_colors(mut self, colors: &'a [Option<Rgb8>]) -> Self {
        self.fixed_colors = Some(colors);
        self
    }

    /// Set the number of nearest spatial neighbors considered for each point.
    #[must_use]
    pub fn neighbors(mut self, neighbors: usize) -> Self {
        self.neighbors = neighbors;
        self
    }

    /// Limit deterministic spatial sampling, or pass `None` to use every point.
    #[must_use]
    pub fn max_points(mut self, max_points: Option<usize>) -> Self {
        self.max_points = max_points;
        self
    }
}

/// Generate a palette with the same defaults as the Python API and CLI.
pub fn generate_palette(palette_size: usize) -> Result<Vec<Rgb8>> {
    PaletteGenerator::new().generate(palette_size)
}

/// Extend a palette with the same defaults as the Python API and CLI.
pub fn extend_palette(colors: &[Rgb8], target_size: usize) -> Result<Vec<Rgb8>> {
    PaletteGenerator::new().extend(colors, target_size)
}
