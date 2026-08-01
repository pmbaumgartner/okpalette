use std::cmp::Ordering;

use super::graph::LabelGraph;
use crate::candidates::Candidate;
use crate::color::{ColorProfile, ColorblindMode, Rgb8};
use crate::distance::DistanceWeights;

const SWAP_PASSES: usize = 2;
const SWAP_PAIR_BUDGET: usize = 50_000;

#[derive(Debug, Clone)]
struct Assignment {
    colors: Vec<Option<Rgb8>>,
    profiles: Vec<Option<ColorProfile>>,
}

pub(super) fn assign_generated_palette(
    fixed_colors: &[Option<Rgb8>],
    graph: &LabelGraph,
    palette_candidates: &[Candidate],
    weights: DistanceWeights,
    colorblind_mode: ColorblindMode,
) -> Vec<Rgb8> {
    let mut available = vec![true; palette_candidates.len()];
    let candidate_profiles: Vec<ColorProfile> = palette_candidates
        .iter()
        .map(|candidate| {
            ColorProfile::from_rgb_and_normal(candidate.rgb, candidate.lab, colorblind_mode)
        })
        .collect();
    let mut assignment = Assignment::new(fixed_colors, colorblind_mode);
    let generated_labels: Vec<usize> = label_processing_order(graph, fixed_colors)
        .into_iter()
        .filter(|&label_id| fixed_colors[label_id].is_none())
        .collect();

    for &label_id in &generated_labels {
        let candidate_index = select_assignment_candidate(
            label_id,
            &candidate_profiles,
            &available,
            &assignment,
            graph,
            weights,
        )
        .expect("available palette colors were checked before label assignment");
        assignment.assign_generated(
            label_id,
            palette_candidates[candidate_index].rgb,
            candidate_profiles[candidate_index],
        );
        available[candidate_index] = false;
    }

    improve_with_swaps(&mut assignment, &generated_labels, graph, weights);

    assignment
        .colors
        .into_iter()
        .map(|color| color.expect("all labels were assigned colors"))
        .collect()
}

impl Assignment {
    fn new(fixed_colors: &[Option<Rgb8>], colorblind_mode: ColorblindMode) -> Self {
        let mut colors = vec![None; fixed_colors.len()];
        let mut profiles = vec![None; fixed_colors.len()];

        for (label_id, &color) in fixed_colors.iter().enumerate() {
            if let Some(color) = color {
                colors[label_id] = Some(color);
                profiles[label_id] = Some(ColorProfile::from_rgb(color, colorblind_mode));
            }
        }

        Self { colors, profiles }
    }

    fn assign_generated(&mut self, label_id: usize, color: Rgb8, profile: ColorProfile) {
        self.colors[label_id] = Some(color);
        self.profiles[label_id] = Some(profile);
    }
}

fn label_processing_order(graph: &LabelGraph, fixed_colors: &[Option<Rgb8>]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..fixed_colors.len()).collect();
    order.sort_by(|&left, &right| {
        compare_descending_f32(
            assignment_degree(graph, left),
            assignment_degree(graph, right),
        )
        .then_with(|| {
            compare_descending_f32(
                fixed_assignment_neighbor_weight(graph, left, fixed_colors),
                fixed_assignment_neighbor_weight(graph, right, fixed_colors),
            )
        })
        .then_with(|| left.cmp(&right))
    });
    order
}

fn assignment_degree(graph: &LabelGraph, label_id: usize) -> f32 {
    graph.adjacency[label_id]
        .iter()
        .map(|(_, weight)| assignment_edge_weight(*weight))
        .sum()
}

fn fixed_assignment_neighbor_weight<T>(
    graph: &LabelGraph,
    label_id: usize,
    fixed_colors: &[Option<T>],
) -> f32 {
    graph.adjacency[label_id]
        .iter()
        .filter_map(|(neighbor, weight)| fixed_colors[*neighbor].is_some().then_some(*weight))
        .map(assignment_edge_weight)
        .sum()
}

fn assignment_edge_weight(edge_weight: f32) -> f32 {
    edge_weight.sqrt()
}

fn compare_descending_f32(left: f32, right: f32) -> Ordering {
    right.total_cmp(&left)
}

fn select_assignment_candidate(
    label_id: usize,
    candidate_profiles: &[ColorProfile],
    available: &[bool],
    assignment: &Assignment,
    graph: &LabelGraph,
    weights: DistanceWeights,
) -> Option<usize> {
    let mut best_index = None;
    let mut best_score = f32::NEG_INFINITY;

    for (candidate_index, &candidate_profile) in candidate_profiles.iter().enumerate() {
        if !available[candidate_index] {
            continue;
        }

        let score =
            assigned_neighbor_distance(label_id, candidate_profile, assignment, graph, weights);
        if score > best_score {
            best_score = score;
            best_index = Some(candidate_index);
        }
    }

    best_index
}

fn assigned_neighbor_distance(
    label_id: usize,
    candidate_profile: ColorProfile,
    assignment: &Assignment,
    graph: &LabelGraph,
    weights: DistanceWeights,
) -> f32 {
    let mut weighted_sum = 0.0;
    let mut total_weight = 0.0;

    for &(neighbor, edge_weight) in &graph.adjacency[label_id] {
        if let Some(neighbor_profile) = assignment.profiles[neighbor] {
            let assignment_weight = assignment_edge_weight(edge_weight);
            weighted_sum += assignment_weight
                * weights.color_profile_distance_squared(candidate_profile, neighbor_profile);
            total_weight += assignment_weight;
        }
    }

    if total_weight == 0.0 {
        0.0
    } else {
        weighted_sum / total_weight
    }
}

fn improve_with_swaps(
    assignment: &mut Assignment,
    generated_labels: &[usize],
    graph: &LabelGraph,
    weights: DistanceWeights,
) {
    if generated_labels.len() < 2 || graph.is_empty() {
        return;
    }

    for _ in 0..SWAP_PASSES {
        let mut best_swap = None;
        let mut best_delta = 0.0;
        let mut evaluations = 0usize;

        'outer: for (left_offset, &left) in generated_labels.iter().enumerate() {
            for &right in &generated_labels[left_offset + 1..] {
                evaluations += 1;
                let delta = swap_delta(graph, &assignment.profiles, left, right, weights);
                if delta > best_delta {
                    best_delta = delta;
                    best_swap = Some((left, right));
                }

                if evaluations >= SWAP_PAIR_BUDGET {
                    break 'outer;
                }
            }
        }

        let Some((left, right)) = best_swap else {
            return;
        };

        if best_delta <= f32::EPSILON {
            return;
        }

        assignment.colors.swap(left, right);
        assignment.profiles.swap(left, right);
    }
}

fn swap_delta(
    graph: &LabelGraph,
    profiles: &[Option<ColorProfile>],
    left_label: usize,
    right_label: usize,
    weights: DistanceWeights,
) -> f32 {
    let left_profile = profiles[left_label].expect("left label is assigned");
    let right_profile = profiles[right_label].expect("right label is assigned");

    incident_swap_delta(
        &graph.adjacency[left_label],
        profiles,
        right_label,
        left_profile,
        right_profile,
        weights,
    ) + incident_swap_delta(
        &graph.adjacency[right_label],
        profiles,
        left_label,
        right_profile,
        left_profile,
        weights,
    )
}

fn incident_swap_delta(
    neighbors: &[(usize, f32)],
    profiles: &[Option<ColorProfile>],
    swapped_neighbor: usize,
    before_profile: ColorProfile,
    after_profile: ColorProfile,
    weights: DistanceWeights,
) -> f32 {
    neighbors
        .iter()
        .filter(|&&(neighbor, _)| neighbor != swapped_neighbor)
        .map(|&(neighbor, edge_weight)| {
            let neighbor_profile = profiles[neighbor].expect("edge endpoint is assigned");
            assignment_edge_weight(edge_weight)
                * (weights.color_profile_distance_squared(after_profile, neighbor_profile)
                    - weights.color_profile_distance_squared(before_profile, neighbor_profile))
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::super::graph::LabelGraph;
    use super::*;
    use crate::test_support::rgb;

    #[test]
    fn assignment_edge_weight_flattens_normalized_graph_pressure() {
        assert_eq!(assignment_edge_weight(1.0), 1.0);
        assert!((assignment_edge_weight(0.25) - 0.5).abs() <= f32::EPSILON);
    }

    #[test]
    fn label_processing_order_uses_flattened_assignment_weights() {
        let graph = LabelGraph {
            adjacency: vec![
                vec![(2, 1.0)],
                vec![(3, 0.09), (4, 0.09), (5, 0.09), (6, 0.09)],
                vec![(0, 1.0)],
                vec![(1, 0.09)],
                vec![(1, 0.09)],
                vec![(1, 0.09)],
                vec![(1, 0.09)],
            ],
        };
        let fixed_colors = vec![None; 7];

        let order = label_processing_order(&graph, &fixed_colors);

        assert_eq!(order[0], 1);
    }

    #[test]
    fn swap_delta_detects_improvement() {
        let graph = LabelGraph {
            adjacency: vec![vec![(1, 1.0)], vec![(0, 1.0), (2, 1.0)], vec![(1, 1.0)]],
        };
        let profiles = vec![
            Some(ColorProfile::from_rgb(rgb(0, 0, 0), ColorblindMode::None)),
            Some(ColorProfile::from_rgb(
                rgb(20, 20, 20),
                ColorblindMode::None,
            )),
            Some(ColorProfile::from_rgb(
                rgb(255, 255, 255),
                ColorblindMode::None,
            )),
        ];

        let delta = swap_delta(&graph, &profiles, 1, 2, DistanceWeights::default());

        assert!(delta > 0.0);
    }
}
