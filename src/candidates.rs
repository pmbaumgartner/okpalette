use crate::algorithm::DistanceWeights;
use crate::color::{Oklab, Rgb8};
use crate::error::{GlasbeyError, Result};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Candidate {
    pub rgb: Rgb8,
    pub lab: Oklab,
    pub chroma: f32,
    pub hue: f32,
}

impl Candidate {
    pub fn from_rgb(rgb: Rgb8) -> Self {
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
pub enum GridSize {
    Coarse,
    Medium,
    Fine,
    Step(u8),
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CandidateConstraints {
    pub lightness: Option<(f32, f32)>,
    pub chroma: Option<(Option<f32>, Option<f32>)>,
    pub hue: Option<(f32, f32)>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BackgroundFilter<'a> {
    pub backgrounds: &'a [Oklab],
    pub min_distance_squared: Option<f32>,
    pub weights: DistanceWeights,
}

impl GridSize {
    pub fn step(self) -> Result<u8> {
        match self {
            Self::Coarse => Ok(16),
            Self::Medium => Ok(8),
            Self::Fine => Ok(4),
            Self::Step(0) => Err(GlasbeyError::InvalidGridStep),
            Self::Step(step) => Ok(step),
        }
    }
}

pub fn generate_candidates(
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

pub fn generate_candidates_with_background_filter(
    grid_size: GridSize,
    constraints: CandidateConstraints,
    background_filter: BackgroundFilter<'_>,
    requested_palette_size: usize,
) -> Result<Vec<Candidate>> {
    constraints.validate()?;
    background_filter.validate()?;

    let channel_values = channel_values(grid_size.step()?);
    let mut candidates =
        Vec::with_capacity(channel_values.len() * channel_values.len() * channel_values.len());

    for &r in &channel_values {
        for &g in &channel_values {
            for &b in &channel_values {
                let rgb = Rgb8 { r, g, b };
                let candidate = Candidate::from_rgb(rgb);

                if constraints.allows(candidate) && background_filter.allows(candidate.lab) {
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
    fn validate(self) -> Result<()> {
        if let Some(min_distance_squared) = self.min_distance_squared {
            if !min_distance_squared.is_finite() || min_distance_squared < 0.0 {
                return Err(GlasbeyError::InvalidConstraintRange {
                    constraint: "background",
                    message: "contrast distance must be finite and greater than or equal to zero",
                });
            }
        }

        self.weights.validate()
    }

    fn allows(self, lab: Oklab) -> bool {
        let Some(min_distance_squared) = self.min_distance_squared else {
            return true;
        };

        self.backgrounds.iter().all(|&background| {
            self.weights.oklab_distance_squared(lab, background) >= min_distance_squared
        })
    }
}

impl CandidateConstraints {
    fn validate(self) -> Result<()> {
        if let Some((min, max)) = self.lightness {
            validate_required_range("lightness", min, max, 0.0, 1.0)?;
        }

        if let Some((min, max)) = self.chroma {
            validate_optional_bound("chroma", "minimum", min)?;
            validate_optional_bound("chroma", "maximum", max)?;
            if let (Some(min), Some(max)) = (min, max) {
                if min > max {
                    return Err(GlasbeyError::InvalidConstraintRange {
                        constraint: "chroma",
                        message: "minimum must be less than or equal to maximum",
                    });
                }
            }
        }

        if let Some((start, end)) = self.hue {
            validate_hue_bound(start)?;
            validate_hue_bound(end)?;
        }

        Ok(())
    }

    fn allows(self, candidate: Candidate) -> bool {
        if let Some((min, max)) = self.lightness {
            if candidate.lab.l < min || candidate.lab.l > max {
                return false;
            }
        }

        if let Some((min, max)) = self.chroma {
            if let Some(min) = min {
                if candidate.chroma < min {
                    return false;
                }
            }

            if let Some(max) = max {
                if candidate.chroma > max {
                    return false;
                }
            }
        }

        if let Some((start, end)) = self.hue {
            if !hue_in_range(candidate.hue, start, end) {
                return false;
            }
        }

        true
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

fn validate_required_range(
    constraint: &'static str,
    min: f32,
    max: f32,
    allowed_min: f32,
    allowed_max: f32,
) -> Result<()> {
    if !min.is_finite() || !max.is_finite() {
        return Err(GlasbeyError::InvalidConstraintRange {
            constraint,
            message: "bounds must be finite",
        });
    }

    if min < allowed_min || max > allowed_max {
        return Err(GlasbeyError::InvalidConstraintRange {
            constraint,
            message: "bounds are outside the allowed range",
        });
    }

    if min > max {
        return Err(GlasbeyError::InvalidConstraintRange {
            constraint,
            message: "minimum must be less than or equal to maximum",
        });
    }

    Ok(())
}

fn validate_optional_bound(
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

fn hue_in_range(hue: f32, start: f32, end: f32) -> bool {
    if start <= end {
        hue >= start && hue <= end
    } else {
        hue >= start || hue <= end
    }
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
        let dark = small_candidates(CandidateConstraints {
            lightness: Some((0.0, 0.1)),
            ..CandidateConstraints::default()
        })
        .unwrap();
        let light = small_candidates(CandidateConstraints {
            lightness: Some((0.99, 1.0)),
            ..CandidateConstraints::default()
        })
        .unwrap();

        assert_eq!(rgb_values(&dark), vec![rgb(0, 0, 0)]);
        assert_eq!(rgb_values(&light), vec![rgb(255, 255, 255)]);
    }

    #[test]
    fn filters_by_chroma() {
        let neutrals = small_candidates(CandidateConstraints {
            chroma: Some((None, Some(0.01))),
            ..CandidateConstraints::default()
        })
        .unwrap();
        let saturated = small_candidates(CandidateConstraints {
            chroma: Some((Some(0.1), None)),
            ..CandidateConstraints::default()
        })
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
        let candidates = small_candidates(CandidateConstraints {
            hue: Some((100.0, 180.0)),
            ..CandidateConstraints::default()
        })
        .unwrap();
        let rgbs = rgb_values(&candidates);

        assert!(rgbs.contains(&rgb(0, 255, 0)));
        assert!(!rgbs.contains(&rgb(255, 0, 0)));
    }

    #[test]
    fn filters_by_wrapping_hue_range() {
        let candidates = small_candidates(CandidateConstraints {
            hue: Some((330.0, 40.0)),
            ..CandidateConstraints::default()
        })
        .unwrap();
        let rgbs = rgb_values(&candidates);

        assert!(rgbs.contains(&rgb(255, 0, 0)));
        assert!(!rgbs.contains(&rgb(0, 255, 0)));
    }

    #[test]
    fn filters_by_background_contrast_distance() {
        let background = rgb(255, 255, 255).to_oklab();
        let candidates = generate_candidates_with_background_filter(
            GridSize::Step(255),
            CandidateConstraints::default(),
            BackgroundFilter {
                backgrounds: &[background],
                min_distance_squared: Some(0.006),
                ..BackgroundFilter::default()
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
        let backgrounds = [rgb(255, 255, 255).to_oklab(), rgb(0, 0, 0).to_oklab()];
        let candidates = generate_candidates_with_background_filter(
            GridSize::Step(255),
            CandidateConstraints::default(),
            BackgroundFilter {
                backgrounds: &backgrounds,
                min_distance_squared: Some(0.006),
                ..BackgroundFilter::default()
            },
            0,
        )
        .unwrap();
        let rgbs = rgb_values(&candidates);

        assert!(!rgbs.contains(&rgb(255, 255, 255)));
        assert!(!rgbs.contains(&rgb(0, 0, 0)));
    }

    #[test]
    fn rejects_invalid_constraint_ranges() {
        for constraints in [
            CandidateConstraints {
                lightness: Some((0.8, 0.2)),
                ..CandidateConstraints::default()
            },
            CandidateConstraints {
                lightness: Some((-0.1, 0.2)),
                ..CandidateConstraints::default()
            },
            CandidateConstraints {
                chroma: Some((Some(0.5), Some(0.1))),
                ..CandidateConstraints::default()
            },
            CandidateConstraints {
                chroma: Some((Some(-0.1), None)),
                ..CandidateConstraints::default()
            },
            CandidateConstraints {
                hue: Some((-1.0, 100.0)),
                ..CandidateConstraints::default()
            },
            CandidateConstraints {
                hue: Some((0.0, 361.0)),
                ..CandidateConstraints::default()
            },
        ] {
            assert!(
                matches!(
                    generate_candidates(GridSize::Step(255), constraints, 0),
                    Err(GlasbeyError::InvalidConstraintRange { .. })
                ),
                "{constraints:?} should fail with invalid constraint range"
            );
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
