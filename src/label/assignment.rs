use std::cmp::Ordering;

use super::graph::{GraphEdge, LabelGraph};
use crate::algorithm::DistanceWeights;
use crate::candidates::Candidate;
use crate::color::{Oklab, Rgb8};

const SWAP_PASSES: usize = 2;
const SWAP_PAIR_BUDGET: usize = 50_000;

#[derive(Debug, Clone)]
struct Assignment {
    colors: Vec<Option<Rgb8>>,
    labs: Vec<Option<Oklab>>,
    candidate_indices: Vec<Option<usize>>,
}

pub(super) fn assign_generated_palette(
    label_count: usize,
    fixed_colors: &[Option<Rgb8>],
    graph: &LabelGraph,
    palette_candidates: &[Candidate],
    weights: DistanceWeights,
) -> Vec<Rgb8> {
    let mut available = vec![true; palette_candidates.len()];
    let mut assignment = Assignment::new(label_count, fixed_colors);
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
        )
        .expect("available palette colors were checked before label assignment");
        let candidate = palette_candidates[candidate_index];
        assignment.assign_generated(label_id, candidate_index, candidate);
        available[candidate_index] = false;
    }

    improve_with_swaps(&mut assignment, graph, weights);

    assignment
        .colors
        .into_iter()
        .map(|color| color.expect("all labels were assigned colors"))
        .collect()
}

impl Assignment {
    fn new(label_count: usize, fixed_colors: &[Option<Rgb8>]) -> Self {
        let mut colors = vec![None; label_count];
        let mut labs = vec![None; label_count];

        for (label_id, &color) in fixed_colors.iter().enumerate() {
            if let Some(color) = color {
                colors[label_id] = Some(color);
                labs[label_id] = Some(color.to_oklab());
            }
        }

        Self {
            colors,
            labs,
            candidate_indices: vec![None; label_count],
        }
    }

    fn assign_generated(&mut self, label_id: usize, candidate_index: usize, candidate: Candidate) {
        self.colors[label_id] = Some(candidate.rgb);
        self.labs[label_id] = Some(candidate.lab);
        self.candidate_indices[label_id] = Some(candidate_index);
    }
}

fn label_processing_order(graph: &LabelGraph, fixed_colors: &[Option<Rgb8>]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..fixed_colors.len()).collect();
    order.sort_by(|&left, &right| {
        compare_descending_f32(graph.degree(left), graph.degree(right))
            .then_with(|| {
                compare_descending_f32(
                    graph.fixed_neighbor_weight(left, fixed_colors),
                    graph.fixed_neighbor_weight(right, fixed_colors),
                )
            })
            .then_with(|| left.cmp(&right))
    });
    order
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
) -> Option<usize> {
    let mut best_index = None;
    let mut best_score = f32::NEG_INFINITY;

    for (candidate_index, candidate) in candidates.iter().enumerate() {
        if !available[candidate_index] {
            continue;
        }

        let score = assigned_neighbor_distance(label_id, candidate.lab, assignment, graph, weights);
        if score > best_score {
            best_score = score;
            best_index = Some(candidate_index);
        }
    }

    best_index
}

fn assigned_neighbor_distance(
    label_id: usize,
    candidate_lab: Oklab,
    assignment: &Assignment,
    graph: &LabelGraph,
    weights: DistanceWeights,
) -> f32 {
    let mut weighted_sum = 0.0;
    let mut total_weight = 0.0;

    for &(neighbor, edge_weight) in &graph.adjacency[label_id] {
        if let Some(neighbor_lab) = assignment.labs[neighbor] {
            weighted_sum +=
                edge_weight * weights.oklab_distance_squared(candidate_lab, neighbor_lab);
            total_weight += edge_weight;
        }
    }

    if total_weight == 0.0 {
        0.0
    } else {
        weighted_sum / total_weight
    }
}

fn improve_with_swaps(assignment: &mut Assignment, graph: &LabelGraph, weights: DistanceWeights) {
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
                let delta = swap_delta(&graph.edges, &assignment.labs, left, right, weights);
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
        assignment.labs.swap(left, right);
        assignment.candidate_indices.swap(left, right);
    }
}

fn swap_delta(
    edges: &[GraphEdge],
    labs: &[Option<Oklab>],
    left_label: usize,
    right_label: usize,
    weights: DistanceWeights,
) -> f32 {
    let left_lab = labs[left_label].expect("left label is assigned");
    let right_lab = labs[right_label].expect("right label is assigned");
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

        let edge_left_lab = labs[edge.left].expect("edge endpoint is assigned");
        let edge_right_lab = labs[edge.right].expect("edge endpoint is assigned");
        before += edge.weight * weights.oklab_distance_squared(edge_left_lab, edge_right_lab);

        let swapped_left_lab = if edge.left == left_label {
            right_lab
        } else if edge.left == right_label {
            left_lab
        } else {
            edge_left_lab
        };
        let swapped_right_lab = if edge.right == left_label {
            right_lab
        } else if edge.right == right_label {
            left_lab
        } else {
            edge_right_lab
        };
        after += edge.weight * weights.oklab_distance_squared(swapped_left_lab, swapped_right_lab);
    }

    after - before
}

#[cfg(test)]
mod tests {
    use super::super::graph::{GraphEdge, LabelGraph};
    use super::*;
    use crate::test_support::rgb;

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
        let labs = vec![
            Some(rgb(0, 0, 0).to_oklab()),
            Some(rgb(20, 20, 20).to_oklab()),
            Some(rgb(255, 255, 255).to_oklab()),
        ];

        let delta = swap_delta(&graph.edges, &labs, 1, 2, DistanceWeights::default());

        assert!(delta > 0.0);
    }
}
