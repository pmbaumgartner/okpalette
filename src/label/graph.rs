use std::collections::HashMap;

use kiddo::{KdTree, SquaredEuclidean};

use super::sampling::{deterministic_sample, SamplePoint};
use super::ValidatedLabelGeometry;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct LabelGraph {
    pub(super) adjacency: Vec<Vec<(usize, f32)>>,
}

pub(super) fn build_label_graph(geometry: ValidatedLabelGeometry<'_>) -> LabelGraph {
    if geometry.label_count <= 1 || geometry.label_ids.is_empty() {
        return LabelGraph::empty(geometry.label_count);
    }

    let sample = deterministic_sample(geometry);

    match geometry.dimension {
        1 => build_label_graph_for_dimension::<1>(
            geometry.coordinates,
            geometry.label_ids,
            &sample,
            geometry.label_count,
            geometry.neighbors,
        ),
        2 => build_label_graph_for_dimension::<2>(
            geometry.coordinates,
            geometry.label_ids,
            &sample,
            geometry.label_count,
            geometry.neighbors,
        ),
        3 => build_label_graph_for_dimension::<3>(
            geometry.coordinates,
            geometry.label_ids,
            &sample,
            geometry.label_count,
            geometry.neighbors,
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
) -> LabelGraph {
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

    LabelGraph::from_weights(label_count, weights)
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
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.adjacency.iter().all(Vec::is_empty)
    }

    fn from_weights(label_count: usize, weights: HashMap<(usize, usize), f64>) -> Self {
        if weights.is_empty() {
            return Self::empty(label_count);
        }

        let max_weight = weights.values().copied().fold(0.0, f64::max);
        let mut adjacency = vec![Vec::new(); label_count];
        let mut edges: Vec<(usize, usize, f32)> = weights
            .into_iter()
            .map(|((left, right), weight)| (left, right, (weight / max_weight) as f32))
            .collect();
        edges.sort_by_key(|&(left, right, _)| (left, right));

        for (left, right, weight) in edges {
            adjacency[left].push((right, weight));
            adjacency[right].push((left, weight));
        }

        Self { adjacency }
    }
}

#[cfg(test)]
mod tests {
    use super::super::LabelPaletteOptions;
    use super::*;
    use crate::algorithm::PaletteAnchors;
    use crate::candidates::{BackgroundFilter, CandidateConstraints, GridSize};
    use crate::color::{ColorblindMode, Rgb8};
    use crate::distance::DistanceWeights;

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

        let graph = build_label_graph(super::super::validate_options(options).unwrap());

        assert!(!graph.is_empty());
        assert!(graph
            .adjacency
            .iter()
            .flatten()
            .all(|&(_, weight)| weight > 0.0 && weight <= 1.0));
    }
}
