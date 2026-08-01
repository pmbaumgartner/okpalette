use crate::color::{
    relative_luminance_srgb, wcag_contrast_ratio, wcag_contrast_ratio_from_luminance, ColorProfile,
    ColorblindMode, Oklab, Rgb8,
};
use crate::distance::DistanceWeights;
use crate::error::{GlasbeyError, Result};

pub(crate) const NORMAL_BACKGROUND_DISTANCE_SQUARED: f32 = 0.006;
pub(crate) const WCAG_NON_TEXT_CONTRAST_RATIO: f64 = 3.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Candidate {
    pub(crate) rgb: Rgb8,
    pub(crate) lab: Oklab,
    pub(crate) chroma: f32,
    pub(crate) hue: f32,
}

impl Candidate {
    pub(crate) fn from_rgb(rgb: Rgb8) -> Self {
        let lab = rgb.to_oklab();
        let oklch = lab.to_oklch();
        Self {
            rgb,
            lab,
            chroma: oklch.c,
            hue: oklch.h,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
/// Resolution of the RGB candidate grid searched by a [`PaletteGenerator`](crate::PaletteGenerator).
pub enum GridSize {
    /// Search channels in steps of 16 for fast, lower-resolution generation.
    Coarse,
    /// Search channels in steps of 8; this is the default.
    Medium,
    /// Search channels in steps of 4 for a larger, slower search.
    Fine,
    /// Search channels using the given nonzero step in `1..=255`.
    Step(u8),
}

/// Validated inclusive OKLab lightness bounds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LightnessRange {
    minimum: f32,
    maximum: f32,
}

/// Validated inclusive OKLCH chroma bounds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChromaRange {
    minimum: Option<f32>,
    maximum: Option<f32>,
}

/// Validated inclusive OKLCH hue bounds, which may wrap around zero.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HueRange {
    start: f32,
    end: f32,
}

/// Optional validated bounds applied when constructing candidate colors.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CandidateConstraints {
    lightness: Option<LightnessRange>,
    chroma: Option<ChromaRange>,
    hue: Option<HueRange>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) enum BackgroundFilter<'a> {
    #[default]
    None,
    NormalOklabDistance {
        backgrounds: &'a [Rgb8],
        min_distance_squared: f32,
        weights: DistanceWeights,
        colorblind_mode: ColorblindMode,
    },
    WcagNonTextContrast {
        backgrounds: &'a [Rgb8],
        min_ratio: f64,
    },
}

#[derive(Debug, Clone, PartialEq)]
enum PreparedBackgroundFilter {
    None,
    NormalOklabDistance {
        background_profiles: Vec<ColorProfile>,
        min_distance_squared: f32,
        weights: DistanceWeights,
        colorblind_mode: ColorblindMode,
    },
    WcagNonTextContrast {
        background_luminances: Vec<f64>,
        min_ratio: f64,
    },
}

impl GridSize {
    pub(crate) fn step(self) -> Result<u8> {
        match self {
            Self::Coarse => Ok(16),
            Self::Medium => Ok(8),
            Self::Fine => Ok(4),
            Self::Step(0) => Err(GlasbeyError::InvalidGridStep),
            Self::Step(step) => Ok(step),
        }
    }
}

impl LightnessRange {
    /// Create inclusive lightness bounds in `0.0..=1.0`.
    pub fn new(minimum: f32, maximum: f32) -> Result<Self> {
        validate_finite_bounds("lightness", minimum, maximum)?;
        if minimum < 0.0 || maximum > 1.0 {
            return Err(GlasbeyError::InvalidConstraintRange {
                constraint: "lightness",
                message: "bounds are outside the allowed range",
            });
        }
        Ok(Self { minimum, maximum })
    }

    /// Return the inclusive minimum lightness.
    pub const fn minimum(self) -> f32 {
        self.minimum
    }

    /// Return the inclusive maximum lightness.
    pub const fn maximum(self) -> f32 {
        self.maximum
    }

    fn contains(self, value: f32) -> bool {
        value >= self.minimum && value <= self.maximum
    }
}

impl ChromaRange {
    /// Create inclusive chroma bounds with at least one finite, non-negative endpoint.
    pub fn new(minimum: Option<f32>, maximum: Option<f32>) -> Result<Self> {
        if minimum.is_none() && maximum.is_none() {
            return Err(GlasbeyError::InvalidConstraintRange {
                constraint: "chroma",
                message: "at least one bound must be provided",
            });
        }
        validate_non_negative_bound("chroma", "minimum", minimum)?;
        validate_non_negative_bound("chroma", "maximum", maximum)?;
        if let (Some(minimum), Some(maximum)) = (minimum, maximum) {
            if minimum > maximum {
                return Err(GlasbeyError::InvalidConstraintRange {
                    constraint: "chroma",
                    message: "minimum must be less than or equal to maximum",
                });
            }
        }
        Ok(Self { minimum, maximum })
    }

    /// Return the optional inclusive minimum chroma.
    pub const fn minimum(self) -> Option<f32> {
        self.minimum
    }

    /// Return the optional inclusive maximum chroma.
    pub const fn maximum(self) -> Option<f32> {
        self.maximum
    }

    fn contains(self, value: f32) -> bool {
        self.minimum.is_none_or(|minimum| value >= minimum)
            && self.maximum.is_none_or(|maximum| value <= maximum)
    }
}

impl HueRange {
    /// Create inclusive hue bounds in degrees; `start > end` wraps around zero.
    pub fn new(start: f32, end: f32) -> Result<Self> {
        validate_hue_bound(start)?;
        validate_hue_bound(end)?;
        Ok(Self { start, end })
    }

    /// Return the inclusive starting hue in degrees.
    pub const fn start(self) -> f32 {
        self.start
    }

    /// Return the inclusive ending hue in degrees.
    pub const fn end(self) -> f32 {
        self.end
    }

    fn contains(self, hue: f32) -> bool {
        if self.start <= self.end {
            hue >= self.start && hue <= self.end
        } else {
            hue >= self.start || hue <= self.end
        }
    }
}

impl CandidateConstraints {
    /// Create constraints with no lightness, chroma, or hue restrictions.
    pub const fn new() -> Self {
        Self {
            lightness: None,
            chroma: None,
            hue: None,
        }
    }

    /// Apply validated lightness bounds.
    #[must_use]
    pub const fn with_lightness(mut self, range: LightnessRange) -> Self {
        self.lightness = Some(range);
        self
    }

    /// Apply validated chroma bounds.
    #[must_use]
    pub const fn with_chroma(mut self, range: ChromaRange) -> Self {
        self.chroma = Some(range);
        self
    }

    /// Apply validated hue bounds.
    #[must_use]
    pub const fn with_hue(mut self, range: HueRange) -> Self {
        self.hue = Some(range);
        self
    }

    /// Return the configured lightness bounds.
    pub const fn lightness(self) -> Option<LightnessRange> {
        self.lightness
    }

    /// Return the configured chroma bounds.
    pub const fn chroma(self) -> Option<ChromaRange> {
        self.chroma
    }

    /// Return the configured hue bounds.
    pub const fn hue(self) -> Option<HueRange> {
        self.hue
    }

    pub(crate) fn palette_defaults() -> Self {
        Self::new()
            .with_lightness(LightnessRange::new(0.20, 0.90).expect("valid default lightness"))
            .with_chroma(ChromaRange::new(Some(0.04), None).expect("valid default minimum chroma"))
    }

    fn allows(self, candidate: Candidate) -> bool {
        self.lightness
            .is_none_or(|range| range.contains(candidate.lab.l))
            && self
                .chroma
                .is_none_or(|range| range.contains(candidate.chroma))
            && self.hue.is_none_or(|range| range.contains(candidate.hue))
    }
}

#[cfg(test)]
pub(crate) fn generate_candidates(
    grid_size: GridSize,
    constraints: CandidateConstraints,
    requested_palette_size: usize,
) -> Result<Vec<Candidate>> {
    generate_candidates_with_background_filter(
        grid_size,
        constraints,
        BackgroundFilter::default(),
        requested_palette_size,
    )
}

pub(crate) fn generate_candidates_with_background_filter(
    grid_size: GridSize,
    constraints: CandidateConstraints,
    background_filter: BackgroundFilter<'_>,
    requested_palette_size: usize,
) -> Result<Vec<Candidate>> {
    let background_filter = background_filter.prepare()?;

    let channel_values = channel_values(grid_size.step()?);
    let mut candidates =
        Vec::with_capacity(channel_values.len() * channel_values.len() * channel_values.len());

    for &r in &channel_values {
        for &g in &channel_values {
            for &b in &channel_values {
                let rgb = Rgb8 { r, g, b };
                let candidate = Candidate::from_rgb(rgb);

                if constraints.allows(candidate) && background_filter.allows(candidate) {
                    candidates.push(candidate);
                }
            }
        }
    }

    if candidates.len() < requested_palette_size {
        return Err(GlasbeyError::InsufficientCandidates {
            available: candidates.len(),
            requested: requested_palette_size,
        });
    }

    Ok(candidates)
}

impl BackgroundFilter<'_> {
    fn prepare(self) -> Result<PreparedBackgroundFilter> {
        match self {
            Self::None => Ok(PreparedBackgroundFilter::None),
            Self::NormalOklabDistance {
                backgrounds,
                min_distance_squared,
                weights,
                colorblind_mode,
            } => {
                if !min_distance_squared.is_finite() || min_distance_squared < 0.0 {
                    return Err(GlasbeyError::InvalidConstraintRange {
                        constraint: "background",
                        message:
                            "contrast distance must be finite and greater than or equal to zero",
                    });
                }
                Ok(PreparedBackgroundFilter::NormalOklabDistance {
                    background_profiles: backgrounds
                        .iter()
                        .map(|&background| ColorProfile::from_rgb(background, colorblind_mode))
                        .collect(),
                    min_distance_squared,
                    weights,
                    colorblind_mode,
                })
            }
            Self::WcagNonTextContrast {
                backgrounds,
                min_ratio,
            } => {
                if !min_ratio.is_finite() || min_ratio <= 0.0 {
                    return Err(GlasbeyError::InvalidConstraintRange {
                        constraint: "background_contrast",
                        message: "WCAG contrast ratio must be finite and greater than zero",
                    });
                }
                Ok(PreparedBackgroundFilter::WcagNonTextContrast {
                    background_luminances: backgrounds
                        .iter()
                        .map(|&background| relative_luminance_srgb(background))
                        .collect(),
                    min_ratio,
                })
            }
        }
    }

    pub(crate) fn validate_user_colors(self, role: &'static str, colors: &[Rgb8]) -> Result<()> {
        let Self::WcagNonTextContrast {
            backgrounds,
            min_ratio,
        } = self
        else {
            return Ok(());
        };

        for &color in colors {
            for &background in backgrounds {
                let ratio = wcag_contrast_ratio(color, background);
                if ratio < min_ratio {
                    return Err(GlasbeyError::InsufficientBackgroundContrast {
                        role,
                        color: color.to_hex(),
                        background: background.to_hex(),
                        ratio,
                        required: min_ratio,
                    });
                }
            }
        }

        Ok(())
    }
}

impl PreparedBackgroundFilter {
    fn allows(&self, candidate: Candidate) -> bool {
        match self {
            Self::None => true,
            Self::NormalOklabDistance {
                background_profiles,
                min_distance_squared,
                weights,
                colorblind_mode,
            } => {
                let candidate_profile = ColorProfile::from_rgb_and_normal(
                    candidate.rgb,
                    candidate.lab,
                    *colorblind_mode,
                );
                background_profiles.iter().all(|&background_profile| {
                    weights.color_profile_distance_squared(candidate_profile, background_profile)
                        >= *min_distance_squared
                })
            }
            Self::WcagNonTextContrast {
                background_luminances,
                min_ratio,
            } => {
                let candidate_luminance = relative_luminance_srgb(candidate.rgb);
                background_luminances.iter().all(|&background_luminance| {
                    wcag_contrast_ratio_from_luminance(candidate_luminance, background_luminance)
                        >= *min_ratio
                })
            }
        }
    }
}

fn channel_values(step: u8) -> Vec<u8> {
    let step = u16::from(step);
    let mut values = Vec::new();
    let mut value = 0u16;

    while value < 255 {
        values.push(value as u8);
        value += step;
    }

    if values.last() != Some(&255) {
        values.push(255);
    }

    values
}

fn validate_finite_bounds(constraint: &'static str, minimum: f32, maximum: f32) -> Result<()> {
    if !minimum.is_finite() || !maximum.is_finite() {
        return Err(GlasbeyError::InvalidConstraintRange {
            constraint,
            message: "bounds must be finite",
        });
    }
    if minimum > maximum {
        return Err(GlasbeyError::InvalidConstraintRange {
            constraint,
            message: "minimum must be less than or equal to maximum",
        });
    }

    Ok(())
}

fn validate_non_negative_bound(
    constraint: &'static str,
    label: &'static str,
    value: Option<f32>,
) -> Result<()> {
    let Some(value) = value else {
        return Ok(());
    };

    if !value.is_finite() {
        return Err(GlasbeyError::InvalidConstraintRange {
            constraint,
            message: "bounds must be finite",
        });
    }

    if value < 0.0 {
        return Err(GlasbeyError::InvalidConstraintRange {
            constraint,
            message: if label == "minimum" {
                "minimum must be greater than or equal to zero"
            } else {
                "maximum must be greater than or equal to zero"
            },
        });
    }

    Ok(())
}

fn validate_hue_bound(value: f32) -> Result<()> {
    if !value.is_finite() {
        return Err(GlasbeyError::InvalidConstraintRange {
            constraint: "hue",
            message: "bounds must be finite",
        });
    }

    if !(0.0..=360.0).contains(&value) {
        return Err(GlasbeyError::InvalidConstraintRange {
            constraint: "hue",
            message: "bounds must be between 0 and 360 degrees",
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::rgb;

    fn rgb_values(candidates: &[Candidate]) -> Vec<Rgb8> {
        candidates.iter().map(|candidate| candidate.rgb).collect()
    }

    fn small_candidates(constraints: CandidateConstraints) -> Result<Vec<Candidate>> {
        generate_candidates(GridSize::Step(255), constraints, 0)
    }

    #[test]
    fn named_grid_counts_are_deterministic() {
        assert_eq!(
            generate_candidates(GridSize::Coarse, CandidateConstraints::default(), 0)
                .unwrap()
                .len(),
            17 * 17 * 17
        );
        assert_eq!(
            generate_candidates(GridSize::Medium, CandidateConstraints::default(), 0)
                .unwrap()
                .len(),
            33 * 33 * 33
        );
        assert_eq!(
            generate_candidates(GridSize::Fine, CandidateConstraints::default(), 0)
                .unwrap()
                .len(),
            65 * 65 * 65
        );
    }

    #[test]
    fn custom_grid_includes_zero_and_255() {
        let candidates =
            generate_candidates(GridSize::Step(250), CandidateConstraints::default(), 0).unwrap();

        assert_eq!(candidates.len(), 27);
        assert_eq!(candidates.first().unwrap().rgb, rgb(0, 0, 0));
        assert_eq!(candidates.last().unwrap().rgb, rgb(255, 255, 255));
    }

    #[test]
    fn rejects_zero_grid_step() {
        assert!(matches!(
            generate_candidates(GridSize::Step(0), CandidateConstraints::default(), 0),
            Err(GlasbeyError::InvalidGridStep)
        ));
    }

    #[test]
    fn candidate_order_is_stable() {
        let candidates =
            generate_candidates(GridSize::Step(255), CandidateConstraints::default(), 0).unwrap();

        assert_eq!(
            rgb_values(&candidates),
            vec![
                rgb(0, 0, 0),
                rgb(0, 0, 255),
                rgb(0, 255, 0),
                rgb(0, 255, 255),
                rgb(255, 0, 0),
                rgb(255, 0, 255),
                rgb(255, 255, 0),
                rgb(255, 255, 255),
            ]
        );
    }

    #[test]
    fn filters_by_lightness() {
        let dark = small_candidates(
            CandidateConstraints::new().with_lightness(LightnessRange::new(0.0, 0.1).unwrap()),
        )
        .unwrap();
        let light = small_candidates(
            CandidateConstraints::new().with_lightness(LightnessRange::new(0.99, 1.0).unwrap()),
        )
        .unwrap();

        assert_eq!(rgb_values(&dark), vec![rgb(0, 0, 0)]);
        assert_eq!(rgb_values(&light), vec![rgb(255, 255, 255)]);
    }

    #[test]
    fn filters_by_chroma() {
        let neutrals = small_candidates(
            CandidateConstraints::new().with_chroma(ChromaRange::new(None, Some(0.01)).unwrap()),
        )
        .unwrap();
        let saturated = small_candidates(
            CandidateConstraints::new().with_chroma(ChromaRange::new(Some(0.1), None).unwrap()),
        )
        .unwrap();

        assert_eq!(
            rgb_values(&neutrals),
            vec![rgb(0, 0, 0), rgb(255, 255, 255)]
        );
        assert!(!rgb_values(&saturated).contains(&rgb(0, 0, 0)));
        assert!(!rgb_values(&saturated).contains(&rgb(255, 255, 255)));
    }

    #[test]
    fn filters_by_non_wrapping_hue_range() {
        let candidates = small_candidates(
            CandidateConstraints::new().with_hue(HueRange::new(100.0, 180.0).unwrap()),
        )
        .unwrap();
        let rgbs = rgb_values(&candidates);

        assert!(rgbs.contains(&rgb(0, 255, 0)));
        assert!(!rgbs.contains(&rgb(255, 0, 0)));
    }

    #[test]
    fn filters_by_wrapping_hue_range() {
        let candidates = small_candidates(
            CandidateConstraints::new().with_hue(HueRange::new(330.0, 40.0).unwrap()),
        )
        .unwrap();
        let rgbs = rgb_values(&candidates);

        assert!(rgbs.contains(&rgb(255, 0, 0)));
        assert!(!rgbs.contains(&rgb(0, 255, 0)));
    }

    #[test]
    fn filters_by_background_contrast_distance() {
        let backgrounds = [rgb(255, 255, 255)];
        let candidates = generate_candidates_with_background_filter(
            GridSize::Step(255),
            CandidateConstraints::default(),
            BackgroundFilter::NormalOklabDistance {
                backgrounds: &backgrounds,
                min_distance_squared: NORMAL_BACKGROUND_DISTANCE_SQUARED,
                weights: DistanceWeights::default(),
                colorblind_mode: ColorblindMode::None,
            },
            0,
        )
        .unwrap();
        let rgbs = rgb_values(&candidates);

        assert!(!rgbs.contains(&rgb(255, 255, 255)));
        assert!(rgbs.contains(&rgb(255, 0, 0)));
    }

    #[test]
    fn background_filter_can_apply_multiple_backgrounds() {
        let backgrounds = [rgb(255, 255, 255), rgb(0, 0, 0)];
        let candidates = generate_candidates_with_background_filter(
            GridSize::Step(255),
            CandidateConstraints::default(),
            BackgroundFilter::NormalOklabDistance {
                backgrounds: &backgrounds,
                min_distance_squared: NORMAL_BACKGROUND_DISTANCE_SQUARED,
                weights: DistanceWeights::default(),
                colorblind_mode: ColorblindMode::None,
            },
            0,
        )
        .unwrap();
        let rgbs = rgb_values(&candidates);

        assert!(!rgbs.contains(&rgb(255, 255, 255)));
        assert!(!rgbs.contains(&rgb(0, 0, 0)));
    }

    #[test]
    fn filters_by_wcag_non_text_contrast() {
        let backgrounds = [rgb(255, 255, 255)];
        let candidates = generate_candidates_with_background_filter(
            GridSize::Step(255),
            CandidateConstraints::default(),
            BackgroundFilter::WcagNonTextContrast {
                backgrounds: &backgrounds,
                min_ratio: WCAG_NON_TEXT_CONTRAST_RATIO,
            },
            0,
        )
        .unwrap();
        let rgbs = rgb_values(&candidates);

        assert!(!rgbs.contains(&rgb(255, 255, 255)));
        assert!(!rgbs.contains(&rgb(255, 255, 0)));
        assert!(rgbs.contains(&rgb(0, 0, 255)));
    }

    #[test]
    fn rejects_invalid_constraint_ranges() {
        for result in [
            LightnessRange::new(0.8, 0.2),
            LightnessRange::new(-0.1, 0.2),
        ] {
            assert!(matches!(
                result,
                Err(GlasbeyError::InvalidConstraintRange { .. })
            ));
        }
        for result in [
            ChromaRange::new(Some(0.5), Some(0.1)),
            ChromaRange::new(Some(-0.1), None),
            ChromaRange::new(None, None),
        ] {
            assert!(matches!(
                result,
                Err(GlasbeyError::InvalidConstraintRange { .. })
            ));
        }
        for result in [HueRange::new(-1.0, 100.0), HueRange::new(0.0, 361.0)] {
            assert!(matches!(
                result,
                Err(GlasbeyError::InvalidConstraintRange { .. })
            ));
        }
    }

    #[test]
    fn errors_when_too_few_candidates_remain() {
        let error = generate_candidates(GridSize::Step(255), CandidateConstraints::default(), 9)
            .unwrap_err();

        assert!(matches!(
            error,
            GlasbeyError::InsufficientCandidates {
                available: 8,
                requested: 9
            }
        ));

        let message = error.to_string();
        assert!(message.contains("8 candidate colors"));
        assert!(message.contains("palette_size=9"));
        assert!(
            message.contains("relaxing lightness, chroma, hue, background_contrast, or grid_size")
        );
    }
}
