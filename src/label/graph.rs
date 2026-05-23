use std::collections::HashMap;

use kiddo::{KdTree, SquaredEuclidean};

use super::sampling::{deterministic_sample, SamplePoint};
use super::LabelPaletteOptions;
use crate::error::Result;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct GraphEdge {
    pub(super) left: usize,
    pub(super) right: usize,
    pub(super) weight: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct LabelGraph {
    pub(super) adjacency: Vec<Vec<(usize, f32)>>,
    pub(super) edges: Vec<GraphEdge>,
}

pub(super) fn build_label_graph(options: LabelPaletteOptions<'_>) -> Result<LabelGraph> {
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
    pub(super) fn empty(label_count: usize) -> Self {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algorithm::{DistanceWeights, PaletteAnchors};
    use crate::candidates::{BackgroundFilter, CandidateConstraints, GridSize};
    use crate::color::{ColorblindMode, Rgb8};

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
            colorblind_mode: ColorblindMode::None,
            neighbors: 2,
            max_points: None,
        }
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
}
