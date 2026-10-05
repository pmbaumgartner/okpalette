use rayon::prelude::*;

use crate::candidates::{
    generate_candidates_with_background_filter, BackgroundFilter, Candidate, CandidateConstraints,
    GridSize,
};
use crate::color::{ColorProfile, ColorblindMode, Rgb8};
use crate::distance::DistanceWeights;
use crate::error::{GlasbeyError, Result};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct PaletteAnchors<'a> {
    pub(crate) seed_colors: &'a [Rgb8],
    pub(crate) avoid_colors: &'a [Rgb8],
    pub(crate) backgrounds: &'a [Rgb8],
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct PaletteOptions<'a> {
    pub(crate) palette_size: usize,
    pub(crate) anchors: PaletteAnchors<'a>,
    pub(crate) weights: DistanceWeights,
    pub(crate) colorblind_mode: ColorblindMode,
}

/// Generate and select colors using already assembled, role-validated anchors.
pub(crate) fn generate_constrained_palette(
    grid_size: GridSize,
    constraints: CandidateConstraints,
    background_filter: BackgroundFilter<'_>,
    options: PaletteOptions<'_>,
) -> Result<Vec<Rgb8>> {
    let candidates = generate_candidates_with_background_filter(
        grid_size,
        constraints,
        background_filter,
        options.palette_size,
    )?;
    select_palette(&candidates, options)
}

pub(crate) fn select_palette(
    candidates: &[Candidate],
    options: PaletteOptions<'_>,
) -> Result<Vec<Rgb8>> {
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

    let candidate_profiles = candidate_profiles(candidates, options.colorblind_mode);
    let mut nearest_distances = vec![f32::INFINITY; candidates.len()];
    update_from_anchors(
        &candidate_profiles,
        &mut nearest_distances,
        options.anchors,
        options.weights,
        options.colorblind_mode,
    );
    exclude_candidates(&mut nearest_distances, &excluded);

    let mut palette = Vec::with_capacity(options.palette_size);
    for _ in 0..options.palette_size {
        let selected_index = select_farthest_candidate(&nearest_distances)
            .expect("available candidates were checked before selection");
        let selected = candidates[selected_index];
        let selected_profile = candidate_profiles[selected_index];

        palette.push(selected.rgb);
        update_nearest_distances(
            &candidate_profiles,
            &mut nearest_distances,
            selected_profile,
            options.weights,
        );
        nearest_distances[selected_index] = f32::NEG_INFINITY;
    }

    Ok(palette)
}

fn candidate_profiles(
    candidates: &[Candidate],
    colorblind_mode: ColorblindMode,
) -> Vec<ColorProfile> {
    candidates
        .par_iter()
        .map(|candidate| {
            ColorProfile::from_rgb_and_normal(candidate.rgb, candidate.lab, colorblind_mode)
        })
        .collect()
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
    candidate_profiles: &[ColorProfile],
    nearest_distances: &mut [f32],
    anchors: PaletteAnchors<'_>,
    weights: DistanceWeights,
    colorblind_mode: ColorblindMode,
) {
    for &anchor in anchors
        .seed_colors
        .iter()
        .chain(anchors.avoid_colors)
        .chain(anchors.backgrounds)
    {
        update_nearest_distances(
            candidate_profiles,
            nearest_distances,
            ColorProfile::from_rgb(anchor, colorblind_mode),
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
    candidate_profiles: &[ColorProfile],
    nearest_distances: &mut [f32],
    anchor: ColorProfile,
    weights: DistanceWeights,
) {
    candidate_profiles
        .par_iter()
        .zip(nearest_distances.par_iter_mut())
        .for_each(|(candidate_profile, nearest_distance)| {
            let distance = weights.color_profile_distance_squared(*candidate_profile, anchor);
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
    use crate::color::Oklab;
    use crate::test_support::{assert_unique_rgb, lab, rgb};

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
            colorblind_mode: ColorblindMode::None,
        }
    }

    fn colorblind_options(
        palette_size: usize,
        colorblind_mode: ColorblindMode,
    ) -> PaletteOptions<'static> {
        PaletteOptions {
            colorblind_mode,
            ..options(palette_size)
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
        assert_unique_rgb(&palette);
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
        assert_unique_rgb(&palette);
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
            candidate_with_lab(rgb(10, 0, 0), lab(0.1, 0.0, 0.0)),
            candidate_with_lab(rgb(20, 0, 0), lab(0.9, 0.0, 0.0)),
        ];
        let seed_colors = [rgb(0, 0, 0)];
        let seeded_options = PaletteOptions {
            palette_size: 1,
            anchors: PaletteAnchors {
                seed_colors: &seed_colors,
                ..PaletteAnchors::default()
            },
            weights: DistanceWeights::default(),
            colorblind_mode: ColorblindMode::None,
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
            colorblind_mode: ColorblindMode::None,
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
            colorblind_mode: ColorblindMode::None,
        };

        let palette = select_palette(&candidates, options).unwrap();

        assert_eq!(palette.len(), 2);
        assert!(!palette.contains(&background));
    }

    #[test]
    fn non_grid_anchors_influence_distance_without_reducing_selectable_count() {
        let candidates = vec![
            candidate_with_lab(rgb(10, 0, 0), lab(0.1, 0.0, 0.0)),
            candidate_with_lab(rgb(20, 0, 0), lab(0.9, 0.0, 0.0)),
        ];
        let avoid_colors = [rgb(0, 0, 0)];
        let options = PaletteOptions {
            palette_size: 2,
            anchors: PaletteAnchors {
                avoid_colors: &avoid_colors,
                ..PaletteAnchors::default()
            },
            weights: DistanceWeights::default(),
            colorblind_mode: ColorblindMode::None,
        };

        let palette = select_palette(&candidates, options).unwrap();

        assert_eq!(palette, vec![candidates[1].rgb, candidates[0].rgb]);
    }

    #[test]
    fn colorblind_mode_uses_worst_case_simulated_distance() {
        let candidates = candidates(&[rgb(255, 0, 0), rgb(0, 0, 255), rgb(0, 255, 255)]);

        assert_eq!(
            select_palette(&candidates, options(2)),
            Ok(vec![rgb(255, 0, 0), rgb(0, 0, 255)])
        );
        assert_eq!(
            select_palette(&candidates, colorblind_options(2, ColorblindMode::Protan)),
            Ok(vec![rgb(255, 0, 0), rgb(0, 255, 255)])
        );
    }

    #[test]
    fn all_colorblind_mode_scores_against_every_simulation() {
        let weights = DistanceWeights::default();
        let left = ColorProfile::from_rgb(rgb(255, 0, 0), ColorblindMode::All);
        let right = ColorProfile::from_rgb(rgb(0, 0, 255), ColorblindMode::All);

        let expected = [0, 1, 2, 3]
            .map(|index| {
                weights.oklab_distance_squared(left.components()[index], right.components()[index])
            })
            .into_iter()
            .reduce(f32::min)
            .unwrap();

        assert_eq!(left.components().len(), 4);
        assert_eq!(
            weights.color_profile_distance_squared(left, right),
            expected
        );
    }

    #[test]
    fn red_green_colorblind_mode_scores_protan_and_deutan_only() {
        let weights = DistanceWeights::default();
        let left = ColorProfile::from_rgb(rgb(255, 0, 0), ColorblindMode::RedGreen);
        let right = ColorProfile::from_rgb(rgb(0, 0, 255), ColorblindMode::RedGreen);

        let expected = [0, 1, 2]
            .map(|index| {
                weights.oklab_distance_squared(left.components()[index], right.components()[index])
            })
            .into_iter()
            .reduce(f32::min)
            .unwrap();

        assert_eq!(left.components().len(), 3);
        assert_eq!(right.components().len(), 3);
        assert_eq!(
            weights.color_profile_distance_squared(left, right),
            expected
        );
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
            colorblind_mode: ColorblindMode::None,
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
            candidate_with_lab(rgb(10, 0, 0), lab(0.5, 0.0, 0.0)),
            candidate_with_lab(rgb(20, 0, 0), lab(0.5, 0.0, 0.0)),
        ];
        let seed_colors = [rgb(0, 0, 0)];
        let options = PaletteOptions {
            palette_size: 1,
            anchors: PaletteAnchors {
                seed_colors: &seed_colors,
                ..PaletteAnchors::default()
            },
            weights: DistanceWeights::default(),
            colorblind_mode: ColorblindMode::None,
        };

        assert_eq!(
            select_palette(&candidates, options),
            Ok(vec![candidates[0].rgb])
        );
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
            weights: DistanceWeights::new(0.7, 1.3).unwrap(),
            colorblind_mode: ColorblindMode::None,
        };
        let expected = select_palette(&candidates, options).unwrap();

        for _ in 0..50 {
            assert_eq!(select_palette(&candidates, options), Ok(expected.clone()));
        }
    }
}
