mod assignment;
mod graph;
mod sampling;

use crate::algorithm::{select_palette, DistanceWeights, PaletteAnchors, PaletteOptions};
use crate::candidates::{
    generate_candidates_with_background_filter, BackgroundFilter, Candidate, CandidateConstraints,
    GridSize,
};
use crate::color::Rgb8;
use crate::error::{GlasbeyError, Result};

use self::assignment::assign_generated_palette;
use self::graph::build_label_graph;

#[derive(Debug, Clone, Copy)]
pub struct LabelPaletteOptions<'a> {
    pub coordinates: &'a [f64],
    pub dimension: usize,
    pub label_ids: &'a [usize],
    pub label_count: usize,
    pub fixed_colors: &'a [Option<Rgb8>],
    pub constraints: CandidateConstraints,
    pub background_filter: BackgroundFilter<'a>,
    pub grid_size: GridSize,
    pub anchors: PaletteAnchors<'a>,
    pub weights: DistanceWeights,
    pub neighbors: usize,
    pub max_points: Option<usize>,
}

pub fn select_label_palette(options: LabelPaletteOptions<'_>) -> Result<Vec<Rgb8>> {
    validate_options(options)?;

    if options.label_count == 0 {
        return Ok(Vec::new());
    }

    let fixed_count = options
        .fixed_colors
        .iter()
        .filter(|color| color.is_some())
        .count();
    let generated_count = options.label_count - fixed_count;
    let graph = build_label_graph(options)?;

    if generated_count == 0 {
        return Ok(options
            .fixed_colors
            .iter()
            .map(|color| color.expect("all labels have fixed colors"))
            .collect());
    }

    options.weights.validate()?;

    let fixed_anchor_colors: Vec<Rgb8> = options.fixed_colors.iter().flatten().copied().collect();
    let seed_anchor_colors: Vec<Rgb8> = options
        .anchors
        .seed_colors
        .iter()
        .copied()
        .chain(fixed_anchor_colors.iter().copied())
        .collect();
    let candidates = generate_candidates_with_background_filter(
        options.grid_size,
        options.constraints,
        options.background_filter,
        generated_count,
    )?;
    let generated_palette = select_palette(
        &candidates,
        PaletteOptions {
            palette_size: generated_count,
            anchors: PaletteAnchors {
                seed_colors: &seed_anchor_colors,
                avoid_colors: options.anchors.avoid_colors,
                backgrounds: options.anchors.backgrounds,
            },
            weights: options.weights,
        },
    )?;
    let palette_candidates: Vec<Candidate> = generated_palette
        .into_iter()
        .map(Candidate::from_rgb)
        .collect();

    Ok(assign_generated_palette(
        options.label_count,
        options.fixed_colors,
        &graph,
        &palette_candidates,
        options.weights,
    ))
}

fn validate_options(options: LabelPaletteOptions<'_>) -> Result<()> {
    if !(1..=3).contains(&options.dimension) {
        return Err(GlasbeyError::InvalidLabelPaletteInput {
            message: "dimension must be 1, 2, or 3",
        });
    }

    if options.coordinates.len() != options.label_ids.len() * options.dimension {
        return Err(GlasbeyError::InvalidLabelPaletteInput {
            message: "coordinates length must equal label_ids length times dimension",
        });
    }

    if options.fixed_colors.len() != options.label_count {
        return Err(GlasbeyError::InvalidLabelPaletteInput {
            message: "fixed_colors length must equal label_count",
        });
    }

    if options.neighbors == 0 {
        return Err(GlasbeyError::InvalidLabelPaletteInput {
            message: "neighbors must be positive",
        });
    }

    if let Some(max_points) = options.max_points {
        if max_points == 0 {
            return Err(GlasbeyError::InvalidLabelPaletteInput {
                message: "max_points must be positive or None",
            });
        }

        if options.label_ids.len() > max_points && max_points < options.label_count {
            return Err(GlasbeyError::InvalidLabelPaletteInput {
                message: "max_points must be at least the number of labels",
            });
        }
    }

    for &coordinate in options.coordinates {
        if !coordinate.is_finite() {
            return Err(GlasbeyError::InvalidLabelPaletteInput {
                message: "coordinates must be finite",
            });
        }
    }

    for &label_id in options.label_ids {
        if label_id >= options.label_count {
            return Err(GlasbeyError::InvalidLabelPaletteInput {
                message: "label IDs must be less than label_count",
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::graph::LabelGraph;
    use super::*;
    use crate::candidates::generate_candidates;

    fn rgb(r: u8, g: u8, b: u8) -> Rgb8 {
        Rgb8 { r, g, b }
    }

    fn base_options<'a>(
        coordinates: &'a [f64],
        label_ids: &'a [usize],
        label_count: usize,
        fixed_colors: &'a [Option<Rgb8>],
    ) -> LabelPaletteOptions<'a> {
        LabelPaletteOptions {
            coordinates,
            dimension: 2,
            label_ids,
            label_count,
            fixed_colors,
            constraints: CandidateConstraints::default(),
            background_filter: BackgroundFilter::default(),
            grid_size: GridSize::Step(255),
            anchors: PaletteAnchors::default(),
            weights: DistanceWeights::default(),
            neighbors: 2,
            max_points: None,
        }
    }

    #[test]
    fn fixed_colors_are_returned_unchanged() {
        let coordinates = [0.0, 0.0, 1.0, 0.0, 0.0, 1.0];
        let labels = [0, 1, 2];
        let fixed = [None, Some(rgb(255, 0, 0)), None];
        let options = base_options(&coordinates, &labels, 3, &fixed);

        let palette = select_label_palette(options).unwrap();

        assert_eq!(palette.len(), 3);
        assert_eq!(palette[1], rgb(255, 0, 0));
    }

    #[test]
    fn repeated_generation_is_deterministic() {
        let coordinates = [0.0, 0.0, 0.1, 0.0, 1.0, 0.0, 1.1, 0.0, 2.0, 0.0];
        let labels = [0, 1, 2, 0, 1];
        let fixed = [None, None, None];
        let options = base_options(&coordinates, &labels, 3, &fixed);
        let expected = select_label_palette(options).unwrap();

        for _ in 0..25 {
            assert_eq!(select_label_palette(options), Ok(expected.clone()));
        }
    }

    #[test]
    fn generated_color_set_matches_regular_palette_before_assignment() {
        let coordinates = [0.0, 0.0, 10.0, 0.0, 0.1, 0.0, 10.1, 0.0];
        let labels = [0, 1, 2, 3];
        let fixed = [None, None, None, None];
        let options = base_options(&coordinates, &labels, 4, &fixed);
        let label_palette = select_label_palette(options).unwrap();
        let candidates =
            generate_candidates(GridSize::Step(255), CandidateConstraints::default(), 4).unwrap();
        let regular_palette = select_palette(
            &candidates,
            PaletteOptions {
                palette_size: 4,
                anchors: PaletteAnchors::default(),
                weights: DistanceWeights::default(),
            },
        )
        .unwrap();

        assert_eq!(sorted_rgb(label_palette), sorted_rgb(regular_palette));
    }

    #[test]
    fn position_aware_assignment_beats_first_seen_palette_assignment() {
        let coordinates = [0.0, 0.0, 10.0, 0.0, 0.1, 0.0, 10.1, 0.0];
        let labels = [0, 1, 2, 3];
        let fixed = [None, None, None, None];
        let options = base_options(&coordinates, &labels, 4, &fixed);
        let graph = build_label_graph(options).unwrap();
        let position_aware = select_label_palette(options).unwrap();
        let candidates =
            generate_candidates(GridSize::Step(255), CandidateConstraints::default(), 4).unwrap();
        let first_seen = select_palette(
            &candidates,
            PaletteOptions {
                palette_size: 4,
                anchors: PaletteAnchors::default(),
                weights: DistanceWeights::default(),
            },
        )
        .unwrap();

        assert!(
            graph_quality(&graph, &position_aware, DistanceWeights::default())
                > graph_quality(&graph, &first_seen, DistanceWeights::default())
        );
    }

    fn graph_quality(graph: &LabelGraph, palette: &[Rgb8], weights: DistanceWeights) -> f32 {
        graph
            .edges
            .iter()
            .map(|edge| {
                edge.weight
                    * weights.oklab_distance_squared(
                        palette[edge.left].to_oklab(),
                        palette[edge.right].to_oklab(),
                    )
            })
            .sum()
    }

    fn sorted_rgb(mut palette: Vec<Rgb8>) -> Vec<Rgb8> {
        palette.sort_by_key(|rgb| (rgb.r, rgb.g, rgb.b));
        palette
    }
}
