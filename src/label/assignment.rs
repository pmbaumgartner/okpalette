use std::cmp::Ordering;

use super::graph::{GraphEdge, LabelGraph};
use crate::algorithm::DistanceWeights;
use crate::candidates::Candidate;
use crate::color::{ColorProfile, ColorblindMode, Rgb8};

const SWAP_PASSES: usize = 2;
const SWAP_PAIR_BUDGET: usize = 50_000;

#[derive(Debug, Clone)]
struct Assignment {
    colors: Vec<Option<Rgb8>>,
    profiles: Vec<Option<ColorProfile>>,
    candidate_indices: Vec<Option<usize>>,
}

pub(super) fn assign_generated_palette(
    label_count: usize,
    fixed_colors: &[Option<Rgb8>],
    graph: &LabelGraph,
    palette_candidates: &[Candidate],
    weights: DistanceWeights,
    colorblind_mode: ColorblindMode,
) -> Vec<Rgb8> {
    let mut available = vec![true; palette_candidates.len()];
    let mut assignment = Assignment::new(label_count, fixed_colors, colorblind_mode);
    let order = label_processing_order(graph, fixed_colors);

    for label_id in order {
        if assignment.colors[label_id].is_some() {
            continue;
        }

        let candidate_index = select_assignment_candidate(
            label_id,
            palette_candidates,
            &available,
            &assignment,
            graph,
            weights,
            colorblind_mode,
        )
        .expect("available palette colors were checked before label assignment");
        let candidate = palette_candidates[candidate_index];
        assignment.assign_generated(label_id, candidate_index, candidate, colorblind_mode);
        available[candidate_index] = false;
    }

    improve_with_swaps(&mut assignment, graph, weights, colorblind_mode);

    assignment
        .colors
        .into_iter()
        .map(|color| color.expect("all labels were assigned colors"))
        .collect()
}

impl Assignment {
    fn new(
        label_count: usize,
        fixed_colors: &[Option<Rgb8>],
        colorblind_mode: ColorblindMode,
    ) -> Self {
        let mut colors = vec![None; label_count];
        let mut profiles = vec![None; label_count];

        for (label_id, &color) in fixed_colors.iter().enumerate() {
            if let Some(color) = color {
                colors[label_id] = Some(color);
                profiles[label_id] = Some(ColorProfile::from_rgb(color, colorblind_mode));
            }
        }

        Self {
            colors,
            profiles,
            candidate_indices: vec![None; label_count],
        }
    }

    fn assign_generated(
        &mut self,
        label_id: usize,
        candidate_index: usize,
        candidate: Candidate,
        colorblind_mode: ColorblindMode,
    ) {
        self.colors[label_id] = Some(candidate.rgb);
        self.profiles[label_id] = Some(ColorProfile::from_rgb_and_normal(
            candidate.rgb,
            candidate.lab,
            colorblind_mode,
        ));
        self.candidate_indices[label_id] = Some(candidate_index);
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
    candidates: &[Candidate],
    available: &[bool],
    assignment: &Assignment,
    graph: &LabelGraph,
    weights: DistanceWeights,
    colorblind_mode: ColorblindMode,
) -> Option<usize> {
    let mut best_index = None;
    let mut best_score = f32::NEG_INFINITY;

    for (candidate_index, candidate) in candidates.iter().enumerate() {
        if !available[candidate_index] {
            continue;
        }

        let score = assigned_neighbor_distance(
            label_id,
            ColorProfile::from_rgb_and_normal(candidate.rgb, candidate.lab, colorblind_mode),
            assignment,
            graph,
            weights,
            colorblind_mode,
        );
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
    colorblind_mode: ColorblindMode,
) -> f32 {
    let mut weighted_sum = 0.0;
    let mut total_weight = 0.0;

    for &(neighbor, edge_weight) in &graph.adjacency[label_id] {
        if let Some(neighbor_profile) = assignment.profiles[neighbor] {
            let assignment_weight = assignment_edge_weight(edge_weight);
            weighted_sum += assignment_weight
                * weights.color_profile_distance_squared(
                    candidate_profile,
                    neighbor_profile,
                    colorblind_mode,
                );
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
    graph: &LabelGraph,
    weights: DistanceWeights,
    colorblind_mode: ColorblindMode,
) {
    let non_fixed_labels: Vec<usize> = assignment
        .candidate_indices
        .iter()
        .enumerate()
        .filter_map(|(label_id, candidate_index)| candidate_index.is_some().then_some(label_id))
        .collect();

    if non_fixed_labels.len() < 2 || graph.edges.is_empty() {
        return;
    }

    for _ in 0..SWAP_PASSES {
        let mut best_swap = None;
        let mut best_delta = 0.0;
        let mut evaluations = 0usize;

        'outer: for (left_offset, &left) in non_fixed_labels.iter().enumerate() {
            for &right in &non_fixed_labels[left_offset + 1..] {
                evaluations += 1;
                let delta = swap_delta(
                    &graph.edges,
                    &assignment.profiles,
                    left,
                    right,
                    weights,
                    colorblind_mode,
                );
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
        assignment.candidate_indices.swap(left, right);
    }
}

fn swap_delta(
    edges: &[GraphEdge],
    profiles: &[Option<ColorProfile>],
    left_label: usize,
    right_label: usize,
    weights: DistanceWeights,
    colorblind_mode: ColorblindMode,
) -> f32 {
    let left_profile = profiles[left_label].expect("left label is assigned");
    let right_profile = profiles[right_label].expect("right label is assigned");
    let mut before = 0.0;
    let mut after = 0.0;

    for edge in edges {
        if edge.left != left_label
            && edge.right != left_label
            && edge.left != right_label
            && edge.right != right_label
        {
            continue;
        }

        let edge_left_profile = profiles[edge.left].expect("edge endpoint is assigned");
        let edge_right_profile = profiles[edge.right].expect("edge endpoint is assigned");
        let assignment_weight = assignment_edge_weight(edge.weight);
        before += assignment_weight
            * weights.color_profile_distance_squared(
                edge_left_profile,
                edge_right_profile,
                colorblind_mode,
            );

        let swapped_left_profile = if edge.left == left_label {
            right_profile
        } else if edge.left == right_label {
            left_profile
        } else {
            edge_left_profile
        };
        let swapped_right_profile = if edge.right == left_label {
            right_profile
        } else if edge.right == right_label {
            left_profile
        } else {
            edge_right_profile
        };
        after += assignment_weight
            * weights.color_profile_distance_squared(
                swapped_left_profile,
                swapped_right_profile,
                colorblind_mode,
            );
    }

    after - before
}

#[cfg(test)]
mod tests {
    use super::super::graph::{GraphEdge, LabelGraph};
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
            edges: Vec::new(),
        };
        let fixed_colors = vec![None; 7];

        let order = label_processing_order(&graph, &fixed_colors);

        assert_eq!(order[0], 1);
    }

    #[test]
    fn swap_delta_detects_improvement() {
        let graph = LabelGraph {
            adjacency: vec![vec![(1, 1.0)], vec![(0, 1.0), (2, 1.0)], vec![(1, 1.0)]],
            edges: vec![
                GraphEdge {
                    left: 0,
                    right: 1,
                    weight: 1.0,
                },
                GraphEdge {
                    left: 1,
                    right: 2,
                    weight: 1.0,
                },
            ],
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

        let delta = swap_delta(
            &graph.edges,
            &profiles,
            1,
            2,
            DistanceWeights::default(),
            ColorblindMode::None,
        );

        assert!(delta > 0.0);
    }
}
