use rayon::prelude::*;

use crate::candidates::Candidate;
use crate::color::{Oklab, Rgb8};
use crate::error::{GlasbeyError, Result};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DistanceWeights {
    pub lightness: f32,
    pub chroma: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PaletteAnchors<'a> {
    pub seed_colors: &'a [Rgb8],
    pub avoid_colors: &'a [Rgb8],
    pub backgrounds: &'a [Rgb8],
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PaletteOptions<'a> {
    pub palette_size: usize,
    pub anchors: PaletteAnchors<'a>,
    pub weights: DistanceWeights,
}

impl Default for DistanceWeights {
    fn default() -> Self {
        Self {
            lightness: 1.0,
            chroma: 1.0,
        }
    }
}

impl DistanceWeights {
    pub(crate) fn validate(self) -> Result<()> {
        if !self.lightness.is_finite() || !self.chroma.is_finite() {
            return Err(GlasbeyError::InvalidDistanceWeights {
                message: "weights must be finite",
            });
        }

        if self.lightness < 0.0 || self.chroma < 0.0 {
            return Err(GlasbeyError::InvalidDistanceWeights {
                message: "weights must be greater than or equal to zero",
            });
        }

        if self.lightness == 0.0 && self.chroma == 0.0 {
            return Err(GlasbeyError::InvalidDistanceWeights {
                message: "at least one weight must be positive",
            });
        }

        Ok(())
    }

    pub(crate) fn oklab_distance_squared(self, left: Oklab, right: Oklab) -> f32 {
        let dl = left.l - right.l;
        let da = left.a - right.a;
        let db = left.b - right.b;

        self.lightness * dl * dl + self.chroma * (da * da + db * db)
    }
}

pub fn select_palette(candidates: &[Candidate], options: PaletteOptions<'_>) -> Result<Vec<Rgb8>> {
    options.weights.validate()?;

    if options.palette_size == 0 {
        return Ok(Vec::new());
    }

    let excluded = exact_anchor_exclusions(candidates, options.anchors);
    let available = excluded.iter().filter(|&&is_excluded| !is_excluded).count();
    if available < options.palette_size {
        return Err(GlasbeyError::InsufficientCandidates {
            available,
            requested: options.palette_size,
        });
    }

    let mut nearest_distances = vec![f32::INFINITY; candidates.len()];
    update_from_anchors(
        candidates,
        &mut nearest_distances,
        options.anchors,
        options.weights,
    );
    exclude_candidates(&mut nearest_distances, &excluded);

    let mut palette = Vec::with_capacity(options.palette_size);
    for _ in 0..options.palette_size {
        let selected_index = select_farthest_candidate(&nearest_distances)
            .expect("available candidates were checked before selection");
        let selected = candidates[selected_index];

        palette.push(selected.rgb);
        update_nearest_distances(
            candidates,
            &mut nearest_distances,
            selected.lab,
            options.weights,
        );
        nearest_distances[selected_index] = f32::NEG_INFINITY;
    }

    Ok(palette)
}

fn exact_anchor_exclusions(candidates: &[Candidate], anchors: PaletteAnchors<'_>) -> Vec<bool> {
    candidates
        .iter()
        .map(|candidate| is_exact_anchor_match(candidate.rgb, anchors))
        .collect()
}

fn is_exact_anchor_match(rgb: Rgb8, anchors: PaletteAnchors<'_>) -> bool {
    anchors.seed_colors.contains(&rgb)
        || anchors.avoid_colors.contains(&rgb)
        || anchors.backgrounds.contains(&rgb)
}

fn update_from_anchors(
    candidates: &[Candidate],
    nearest_distances: &mut [f32],
    anchors: PaletteAnchors<'_>,
    weights: DistanceWeights,
) {
    for &seed_color in anchors.seed_colors {
        update_nearest_distances(
            candidates,
            nearest_distances,
            seed_color.to_oklab(),
            weights,
        );
    }

    for &avoid_color in anchors.avoid_colors {
        update_nearest_distances(
            candidates,
            nearest_distances,
            avoid_color.to_oklab(),
            weights,
        );
    }

    for &background in anchors.backgrounds {
        update_nearest_distances(
            candidates,
            nearest_distances,
            background.to_oklab(),
            weights,
        );
    }
}

fn exclude_candidates(nearest_distances: &mut [f32], excluded: &[bool]) {
    for (nearest_distance, &is_excluded) in nearest_distances.iter_mut().zip(excluded) {
        if is_excluded {
            *nearest_distance = f32::NEG_INFINITY;
        }
    }
}

fn update_nearest_distances(
    candidates: &[Candidate],
    nearest_distances: &mut [f32],
    anchor: Oklab,
    weights: DistanceWeights,
) {
    candidates
        .par_iter()
        .zip(nearest_distances.par_iter_mut())
        .for_each(|(candidate, nearest_distance)| {
            let distance = weights.oklab_distance_squared(candidate.lab, anchor);
            if distance < *nearest_distance {
                *nearest_distance = distance;
            }
        });
}

fn select_farthest_candidate(nearest_distances: &[f32]) -> Option<usize> {
    let mut best_index = None;
    let mut best_distance = f32::NEG_INFINITY;

    for (index, &distance) in nearest_distances.iter().enumerate() {
        if distance > best_distance {
            best_index = Some(index);
            best_distance = distance;
        }
    }

    best_index
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgb(r: u8, g: u8, b: u8) -> Rgb8 {
        Rgb8 { r, g, b }
    }

    fn candidate(rgb: Rgb8) -> Candidate {
        Candidate::from_rgb(rgb)
    }

    fn candidate_with_lab(rgb: Rgb8, lab: Oklab) -> Candidate {
        let oklch = lab.to_oklch();
        Candidate {
            rgb,
            lab,
            chroma: oklch.c,
            hue: oklch.h,
        }
    }

    fn candidates(colors: &[Rgb8]) -> Vec<Candidate> {
        colors.iter().copied().map(candidate).collect()
    }

    fn options(palette_size: usize) -> PaletteOptions<'static> {
        PaletteOptions {
            palette_size,
            anchors: PaletteAnchors::default(),
            weights: DistanceWeights::default(),
        }
    }

    fn assert_no_duplicates(colors: &[Rgb8]) {
        for (index, color) in colors.iter().enumerate() {
            assert!(
                !colors[index + 1..].contains(color),
                "duplicate color in palette: {color:?}"
            );
        }
    }

    #[test]
    fn selects_requested_number_without_duplicates() {
        let candidates = candidates(&[
            rgb(0, 0, 0),
            rgb(255, 255, 255),
            rgb(255, 0, 0),
            rgb(0, 255, 0),
            rgb(0, 0, 255),
        ]);

        let palette = select_palette(&candidates, options(3)).unwrap();

        assert_eq!(palette.len(), 3);
        assert_no_duplicates(&palette);
    }

    #[test]
    fn repeated_runs_return_identical_palettes() {
        let candidates = candidates(&[
            rgb(0, 0, 0),
            rgb(255, 255, 255),
            rgb(255, 0, 0),
            rgb(0, 255, 0),
            rgb(0, 0, 255),
            rgb(255, 255, 0),
            rgb(255, 0, 255),
            rgb(0, 255, 255),
        ]);
        let expected = select_palette(&candidates, options(5)).unwrap();

        for _ in 0..25 {
            assert_eq!(
                select_palette(&candidates, options(5)),
                Ok(expected.clone())
            );
        }
    }

    #[test]
    fn selecting_all_available_candidates_succeeds() {
        let candidates = candidates(&[
            rgb(0, 0, 0),
            rgb(255, 255, 255),
            rgb(255, 0, 0),
            rgb(0, 255, 0),
        ]);

        let palette = select_palette(&candidates, options(candidates.len())).unwrap();

        assert_eq!(palette.len(), candidates.len());
        assert_no_duplicates(&palette);
        for candidate in candidates {
            assert!(palette.contains(&candidate.rgb));
        }
    }

    #[test]
    fn zero_palette_size_returns_empty_palette() {
        let candidates = candidates(&[rgb(0, 0, 0)]);

        assert_eq!(select_palette(&candidates, options(0)), Ok(Vec::new()));
    }

    #[test]
    fn seed_colors_influence_first_generated_color() {
        let candidates = vec![
            candidate_with_lab(
                rgb(10, 0, 0),
                Oklab {
                    l: 0.1,
                    a: 0.0,
                    b: 0.0,
                },
            ),
            candidate_with_lab(
                rgb(20, 0, 0),
                Oklab {
                    l: 0.9,
                    a: 0.0,
                    b: 0.0,
                },
            ),
        ];
        let seed_colors = [rgb(0, 0, 0)];
        let seeded_options = PaletteOptions {
            palette_size: 1,
            anchors: PaletteAnchors {
                seed_colors: &seed_colors,
                ..PaletteAnchors::default()
            },
            weights: DistanceWeights::default(),
        };

        assert_eq!(
            select_palette(&candidates, options(1)),
            Ok(vec![candidates[0].rgb])
        );
        assert_eq!(
            select_palette(&candidates, seeded_options),
            Ok(vec![candidates[1].rgb])
        );
    }

    #[test]
    fn avoid_colors_are_not_returned_when_they_match_candidates() {
        let avoided = rgb(255, 0, 0);
        let candidates = candidates(&[avoided, rgb(0, 255, 0), rgb(0, 0, 255)]);
        let avoid_colors = [avoided];
        let options = PaletteOptions {
            palette_size: 2,
            anchors: PaletteAnchors {
                avoid_colors: &avoid_colors,
                ..PaletteAnchors::default()
            },
            weights: DistanceWeights::default(),
        };

        let palette = select_palette(&candidates, options).unwrap();

        assert_eq!(palette.len(), 2);
        assert!(!palette.contains(&avoided));
    }

    #[test]
    fn background_is_treated_as_an_avoid_anchor() {
        let background = rgb(255, 255, 255);
        let candidates = candidates(&[background, rgb(255, 0, 0), rgb(0, 0, 255)]);
        let options = PaletteOptions {
            palette_size: 2,
            anchors: PaletteAnchors {
                backgrounds: &[background],
                ..PaletteAnchors::default()
            },
            weights: DistanceWeights::default(),
        };

        let palette = select_palette(&candidates, options).unwrap();

        assert_eq!(palette.len(), 2);
        assert!(!palette.contains(&background));
    }

    #[test]
    fn non_grid_anchors_influence_distance_without_reducing_selectable_count() {
        let candidates = vec![
            candidate_with_lab(
                rgb(10, 0, 0),
                Oklab {
                    l: 0.1,
                    a: 0.0,
                    b: 0.0,
                },
            ),
            candidate_with_lab(
                rgb(20, 0, 0),
                Oklab {
                    l: 0.9,
                    a: 0.0,
                    b: 0.0,
                },
            ),
        ];
        let avoid_colors = [rgb(0, 0, 0)];
        let options = PaletteOptions {
            palette_size: 2,
            anchors: PaletteAnchors {
                avoid_colors: &avoid_colors,
                ..PaletteAnchors::default()
            },
            weights: DistanceWeights::default(),
        };

        let palette = select_palette(&candidates, options).unwrap();

        assert_eq!(palette, vec![candidates[1].rgb, candidates[0].rgb]);
    }

    #[test]
    fn too_large_requests_after_anchor_exclusions_return_insufficient_candidates() {
        let excluded_seed = rgb(0, 0, 0);
        let excluded_avoid = rgb(255, 255, 255);
        let candidates = candidates(&[excluded_seed, excluded_avoid, rgb(255, 0, 0)]);
        let seed_colors = [excluded_seed];
        let avoid_colors = [excluded_avoid];
        let options = PaletteOptions {
            palette_size: 2,
            anchors: PaletteAnchors {
                seed_colors: &seed_colors,
                avoid_colors: &avoid_colors,
                ..PaletteAnchors::default()
            },
            weights: DistanceWeights::default(),
        };

        assert_eq!(
            select_palette(&candidates, options),
            Err(GlasbeyError::InsufficientCandidates {
                available: 1,
                requested: 2,
            })
        );
    }

    #[test]
    fn equal_score_ties_choose_lower_candidate_index() {
        let candidates = vec![
            candidate_with_lab(
                rgb(10, 0, 0),
                Oklab {
                    l: 0.5,
                    a: 0.0,
                    b: 0.0,
                },
            ),
            candidate_with_lab(
                rgb(20, 0, 0),
                Oklab {
                    l: 0.5,
                    a: 0.0,
                    b: 0.0,
                },
            ),
        ];
        let seed_colors = [rgb(0, 0, 0)];
        let options = PaletteOptions {
            palette_size: 1,
            anchors: PaletteAnchors {
                seed_colors: &seed_colors,
                ..PaletteAnchors::default()
            },
            weights: DistanceWeights::default(),
        };

        assert_eq!(
            select_palette(&candidates, options),
            Ok(vec![candidates[0].rgb])
        );
    }

    #[test]
    fn invalid_weights_return_weight_error() {
        let candidates = candidates(&[rgb(0, 0, 0)]);
        let invalid_weights = [
            DistanceWeights {
                lightness: f32::NAN,
                chroma: 1.0,
            },
            DistanceWeights {
                lightness: f32::INFINITY,
                chroma: 1.0,
            },
            DistanceWeights {
                lightness: -1.0,
                chroma: 1.0,
            },
            DistanceWeights {
                lightness: 0.0,
                chroma: 0.0,
            },
        ];

        for weights in invalid_weights {
            let options = PaletteOptions {
                weights,
                ..options(1)
            };

            assert!(matches!(
                select_palette(&candidates, options),
                Err(GlasbeyError::InvalidDistanceWeights { .. })
            ));
        }
    }

    #[test]
    fn rayon_update_path_is_stable_across_repeated_calls() {
        let candidates: Vec<Candidate> = (0..64)
            .map(|index| {
                candidate(rgb(
                    (index * 3) as u8,
                    (255 - index * 2) as u8,
                    (index * 5) as u8,
                ))
            })
            .collect();
        let seed_colors = [rgb(4, 8, 12)];
        let avoid_colors = [rgb(16, 32, 64)];
        let options = PaletteOptions {
            palette_size: 12,
            anchors: PaletteAnchors {
                seed_colors: &seed_colors,
                avoid_colors: &avoid_colors,
                backgrounds: &[rgb(240, 240, 240)],
            },
            weights: DistanceWeights {
                lightness: 0.7,
                chroma: 1.3,
            },
        };
        let expected = select_palette(&candidates, options).unwrap();

        for _ in 0..50 {
            assert_eq!(select_palette(&candidates, options), Ok(expected.clone()));
        }
    }
}
