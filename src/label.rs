use std::cmp::Ordering;
use std::collections::HashMap;

use kiddo::{KdTree, SquaredEuclidean};

use crate::algorithm::{
    select_palette, weighted_oklab_distance_squared, DistanceWeights, PaletteAnchors,
    PaletteOptions,
};
use crate::candidates::{
    generate_candidates_with_background_filter, BackgroundFilter, Candidate, CandidateConstraints,
    GridSize,
};
use crate::color::{Oklab, Rgb8};
use crate::error::{GlasbeyError, Result};

const SWAP_PASSES: usize = 2;
const SWAP_PAIR_BUDGET: usize = 50_000;

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

#[derive(Debug, Clone, Copy, PartialEq)]
struct GraphEdge {
    left: usize,
    right: usize,
    weight: f32,
}

#[derive(Debug, Clone, PartialEq)]
struct LabelGraph {
    adjacency: Vec<Vec<(usize, f32)>>,
    edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct SamplePoint {
    original_index: usize,
    label_id: usize,
}

#[derive(Debug, Clone)]
struct Assignment {
    colors: Vec<Option<Rgb8>>,
    labs: Vec<Option<Oklab>>,
    candidate_indices: Vec<Option<usize>>,
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
        .map(candidate_from_rgb)
        .collect();
    let mut available = vec![true; palette_candidates.len()];

    let mut assignment = Assignment::new(options.label_count, options.fixed_colors);
    let order = label_processing_order(&graph, options.fixed_colors);
    for label_id in order {
        if assignment.colors[label_id].is_some() {
            continue;
        }

        let candidate_index = select_assignment_candidate(
            label_id,
            &palette_candidates,
            &available,
            &assignment,
            &graph,
            options.weights,
        )
        .expect("available palette colors were checked before label assignment");
        let candidate = palette_candidates[candidate_index];
        assignment.assign_generated(label_id, candidate_index, candidate);
        available[candidate_index] = false;
    }

    improve_with_swaps(&mut assignment, &graph, options.weights);

    Ok(assignment
        .colors
        .into_iter()
        .map(|color| color.expect("all labels were assigned colors"))
        .collect())
}

fn candidate_from_rgb(rgb: Rgb8) -> Candidate {
    let lab = rgb.to_oklab();
    let oklch = lab.to_oklch();
    Candidate {
        rgb,
        lab,
        chroma: oklch.c,
        hue: oklch.h,
    }
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

fn build_label_graph(options: LabelPaletteOptions<'_>) -> Result<LabelGraph> {
    if options.label_count <= 1 || options.label_ids.is_empty() {
        return Ok(LabelGraph::empty(options.label_count));
    }

    let sample = deterministic_sample(
        options.label_ids,
        options.label_count,
        options.max_points.unwrap_or(options.label_ids.len()),
    );

    match options.dimension {
        1 => build_label_graph_for_dimension::<1>(
            options.coordinates,
            options.label_ids,
            &sample,
            options.label_count,
            options.neighbors,
        ),
        2 => build_label_graph_for_dimension::<2>(
            options.coordinates,
            options.label_ids,
            &sample,
            options.label_count,
            options.neighbors,
        ),
        3 => build_label_graph_for_dimension::<3>(
            options.coordinates,
            options.label_ids,
            &sample,
            options.label_count,
            options.neighbors,
        ),
        _ => unreachable!("dimension was validated"),
    }
}

fn deterministic_sample(
    label_ids: &[usize],
    label_count: usize,
    max_points: usize,
) -> Vec<SamplePoint> {
    if label_ids.len() <= max_points {
        return label_ids
            .iter()
            .copied()
            .enumerate()
            .map(|(original_index, label_id)| SamplePoint {
                original_index,
                label_id,
            })
            .collect();
    }

    let mut by_label = vec![Vec::new(); label_count];
    for (index, &label_id) in label_ids.iter().enumerate() {
        by_label[label_id].push(index);
    }

    let mut quotas = vec![0usize; label_count];
    let represented_labels: Vec<usize> = by_label
        .iter()
        .enumerate()
        .filter_map(|(label_id, indices)| (!indices.is_empty()).then_some(label_id))
        .collect();

    for &label_id in &represented_labels {
        quotas[label_id] = 1;
    }

    let remaining_budget = max_points - represented_labels.len();
    let remaining_points: usize = by_label
        .iter()
        .map(|indices| indices.len().saturating_sub(1))
        .sum();

    if remaining_budget > 0 && remaining_points > 0 {
        let mut fractions = Vec::new();
        let mut assigned = 0usize;

        for &label_id in &represented_labels {
            let extra_available = by_label[label_id].len() - 1;
            let raw = remaining_budget * extra_available;
            let whole = raw / remaining_points;
            let remainder = raw % remaining_points;
            let whole = whole.min(extra_available);
            quotas[label_id] += whole;
            assigned += whole;
            if quotas[label_id] < by_label[label_id].len() {
                fractions.push((label_id, remainder));
            }
        }

        fractions.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));

        let mut leftover = remaining_budget.saturating_sub(assigned);
        for (label_id, _) in fractions {
            if leftover == 0 {
                break;
            }
            if quotas[label_id] < by_label[label_id].len() {
                quotas[label_id] += 1;
                leftover -= 1;
            }
        }
    }

    let mut sample = Vec::with_capacity(max_points);
    for (label_id, indices) in by_label.iter().enumerate() {
        for original_index in evenly_spaced_indices(indices, quotas[label_id]) {
            sample.push(SamplePoint {
                original_index,
                label_id,
            });
        }
    }
    sample.sort_by_key(|point| point.original_index);
    sample
}

fn evenly_spaced_indices(indices: &[usize], quota: usize) -> Vec<usize> {
    if quota == 0 || indices.is_empty() {
        return Vec::new();
    }

    if quota >= indices.len() {
        return indices.to_vec();
    }

    if quota == 1 {
        return vec![indices[0]];
    }

    let last = indices.len() - 1;
    (0..quota)
        .map(|rank| {
            let index = (rank * last + (quota - 1) / 2) / (quota - 1);
            indices[index]
        })
        .collect()
}

fn build_label_graph_for_dimension<const D: usize>(
    coordinates: &[f64],
    label_ids: &[usize],
    sample: &[SamplePoint],
    label_count: usize,
    neighbors: usize,
) -> Result<LabelGraph> {
    let mut tree: KdTree<f64, D> = KdTree::new();
    for (sample_index, point) in sample.iter().enumerate() {
        tree.add(
            &point_for_dimension::<D>(coordinates, point.original_index),
            sample_index as u64,
        );
    }

    let search_count = sample.len().min(
        (neighbors.saturating_mul(8) + 1)
            .max(label_count.saturating_mul(2))
            .max(32),
    );
    let mut weights: HashMap<(usize, usize), f64> = HashMap::new();

    for point in sample {
        let query_label = point.label_id;
        let query = point_for_dimension::<D>(coordinates, point.original_index);
        let nearest = tree.nearest_n::<SquaredEuclidean>(&query, search_count);
        let mut contacts = Vec::new();

        for neighbor in nearest {
            let neighbor_point = sample[neighbor.item as usize];
            let neighbor_label = label_ids[neighbor_point.original_index];
            if neighbor_point.original_index == point.original_index
                || neighbor_label == query_label
            {
                continue;
            }

            contacts.push((neighbor_label, neighbor.distance));
            if contacts.len() == neighbors {
                break;
            }
        }

        if contacts.is_empty() {
            continue;
        }

        let base_distance = contacts[0].1.max(f64::EPSILON);
        for (rank, (neighbor_label, distance)) in contacts.into_iter().enumerate() {
            let rank_decay = 1.0 / (rank as f64 + 1.0);
            let distance_decay = base_distance.sqrt() / distance.max(base_distance).sqrt();
            let weight = rank_decay * distance_decay;
            let edge = ordered_pair(query_label, neighbor_label);
            *weights.entry(edge).or_insert(0.0) += weight;
        }
    }

    Ok(LabelGraph::from_weights(label_count, weights))
}

fn point_for_dimension<const D: usize>(coordinates: &[f64], point_index: usize) -> [f64; D] {
    let start = point_index * D;
    std::array::from_fn(|offset| coordinates[start + offset])
}

fn ordered_pair(left: usize, right: usize) -> (usize, usize) {
    if left < right {
        (left, right)
    } else {
        (right, left)
    }
}

impl LabelGraph {
    fn empty(label_count: usize) -> Self {
        Self {
            adjacency: vec![Vec::new(); label_count],
            edges: Vec::new(),
        }
    }

    fn from_weights(label_count: usize, weights: HashMap<(usize, usize), f64>) -> Self {
        if weights.is_empty() {
            return Self::empty(label_count);
        }

        let max_weight = weights.values().copied().fold(0.0, f64::max);
        let mut adjacency = vec![Vec::new(); label_count];
        let mut edges: Vec<GraphEdge> = weights
            .into_iter()
            .map(|((left, right), weight)| GraphEdge {
                left,
                right,
                weight: (weight / max_weight) as f32,
            })
            .collect();
        edges.sort_by_key(|edge| (edge.left, edge.right));

        for edge in &edges {
            adjacency[edge.left].push((edge.right, edge.weight));
            adjacency[edge.right].push((edge.left, edge.weight));
        }

        Self { adjacency, edges }
    }

    fn degree(&self, label_id: usize) -> f32 {
        self.adjacency[label_id]
            .iter()
            .map(|(_, weight)| *weight)
            .sum()
    }

    fn fixed_neighbor_weight(&self, label_id: usize, fixed_colors: &[Option<Rgb8>]) -> f32 {
        self.adjacency[label_id]
            .iter()
            .filter_map(|(neighbor, weight)| fixed_colors[*neighbor].is_some().then_some(*weight))
            .sum()
    }
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
                edge_weight * weighted_oklab_distance_squared(candidate_lab, neighbor_lab, weights);
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
        before +=
            edge.weight * weighted_oklab_distance_squared(edge_left_lab, edge_right_lab, weights);

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
        after += edge.weight
            * weighted_oklab_distance_squared(swapped_left_lab, swapped_right_lab, weights);
    }

    after - before
}

#[cfg(test)]
mod tests {
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
    fn deterministic_sample_keeps_rare_labels_represented() {
        let labels = [0, 0, 0, 0, 1, 2, 2, 2];
        let sample = deterministic_sample(&labels, 3, 4);
        let sampled_labels: Vec<usize> = sample.iter().map(|point| point.label_id).collect();

        assert_eq!(sample.len(), 4);
        assert!(sampled_labels.contains(&0));
        assert!(sampled_labels.contains(&1));
        assert!(sampled_labels.contains(&2));
    }

    #[test]
    fn graph_edges_are_normalized() {
        let coordinates = [0.0, 0.0, 1.0, 0.0, 8.0, 0.0];
        let labels = [0, 1, 2];
        let fixed = [None, None, None];
        let options = base_options(&coordinates, &labels, 3, &fixed);

        let graph = build_label_graph(options).unwrap();

        assert!(!graph.edges.is_empty());
        assert!(graph
            .edges
            .iter()
            .all(|edge| edge.weight > 0.0 && edge.weight <= 1.0));
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
                    * weighted_oklab_distance_squared(
                        palette[edge.left].to_oklab(),
                        palette[edge.right].to_oklab(),
                        weights,
                    )
            })
            .sum()
    }

    fn sorted_rgb(mut palette: Vec<Rgb8>) -> Vec<Rgb8> {
        palette.sort_by_key(|rgb| (rgb.r, rgb.g, rgb.b));
        palette
    }
}
