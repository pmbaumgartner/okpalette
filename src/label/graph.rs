use std::collections::HashMap;

use rstar::{primitives::GeomWithData, RTree};

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
        1 => build_label_graph_for_dimension::<1, 2>(
            geometry.coordinates,
            &sample,
            geometry.label_count,
            geometry.neighbors,
        ),
        2 => build_label_graph_for_dimension::<2, 2>(
            geometry.coordinates,
            &sample,
            geometry.label_count,
            geometry.neighbors,
        ),
        3 => build_label_graph_for_dimension::<3, 3>(
            geometry.coordinates,
            &sample,
            geometry.label_count,
            geometry.neighbors,
        ),
        _ => unreachable!("dimension was validated"),
    }
}

fn build_label_graph_for_dimension<const SOURCE_D: usize, const INDEX_D: usize>(
    coordinates: &[f64],
    sample: &[SamplePoint],
    label_count: usize,
    neighbors: usize,
) -> LabelGraph {
    let trees = build_label_trees::<SOURCE_D, INDEX_D>(coordinates, sample, label_count);
    let mut weights: HashMap<(usize, usize), f64> = HashMap::new();

    for point in sample {
        let query_label = point.label_id;
        let query = point_for_index::<SOURCE_D, INDEX_D>(coordinates, point.original_index);
        let contacts = nearest_different_label_contacts(&trees, query, query_label, neighbors);

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

type IndexedPoint<const D: usize> = GeomWithData<[f64; D], usize>;

fn build_label_trees<const SOURCE_D: usize, const INDEX_D: usize>(
    coordinates: &[f64],
    sample: &[SamplePoint],
    label_count: usize,
) -> Vec<RTree<IndexedPoint<INDEX_D>>> {
    let mut points_by_label = vec![Vec::new(); label_count];
    for point in sample {
        points_by_label[point.label_id].push(IndexedPoint::new(
            point_for_index::<SOURCE_D, INDEX_D>(coordinates, point.original_index),
            point.original_index,
        ));
    }

    points_by_label.into_iter().map(RTree::bulk_load).collect()
}

fn nearest_different_label_contacts<const D: usize>(
    trees: &[RTree<IndexedPoint<D>>],
    query: [f64; D],
    query_label: usize,
    neighbors: usize,
) -> Vec<(usize, f64)> {
    let mut contacts: Vec<(usize, f64, usize)> = trees
        .iter()
        .enumerate()
        .filter(|(label_id, _)| *label_id != query_label)
        .flat_map(|(label_id, tree)| {
            tree.nearest_neighbor_iter_with_distance_2(query)
                .take(neighbors)
                .map(move |(point, distance)| (label_id, distance, point.data))
        })
        .collect();
    contacts.sort_by(|left, right| {
        left.1
            .total_cmp(&right.1)
            .then_with(|| left.0.cmp(&right.0))
            .then_with(|| left.2.cmp(&right.2))
    });
    contacts.truncate(neighbors);
    contacts
        .into_iter()
        .map(|(label_id, distance, _)| (label_id, distance))
        .collect()
}

fn point_for_index<const SOURCE_D: usize, const INDEX_D: usize>(
    coordinates: &[f64],
    point_index: usize,
) -> [f64; INDEX_D] {
    let start = point_index * SOURCE_D;
    std::array::from_fn(|offset| {
        if offset < SOURCE_D {
            coordinates[start + offset]
        } else {
            0.0
        }
    })
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
            grid_size: GridSize::try_step(255).unwrap(),
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

    #[test]
    fn graph_accepts_many_points_with_the_same_axis_coordinate() {
        let mut coordinates = Vec::new();
        let mut labels = Vec::new();
        for index in 0..33 {
            coordinates.extend([0.0, f64::from(index)]);
            labels.push(index as usize % 3);
        }

        let graph = build_label_graph(super::super::ValidatedLabelGeometry {
            coordinates: &coordinates,
            dimension: 2,
            label_ids: &labels,
            label_count: 3,
            neighbors: 2,
            max_points: labels.len(),
        });

        assert!(!graph.is_empty());
    }

    #[test]
    fn dense_labels_still_find_the_nearest_different_label() {
        let mut coordinates = Vec::new();
        let mut labels = Vec::new();
        for (label_id, center) in [0.0, 0.1, 10.0].into_iter().enumerate() {
            for offset in 0..33 {
                coordinates.push(center + f64::from(offset) * 0.000_01);
                labels.push(label_id);
            }
        }

        let graph = build_label_graph(super::super::ValidatedLabelGeometry {
            coordinates: &coordinates,
            dimension: 1,
            label_ids: &labels,
            label_count: 3,
            neighbors: 1,
            max_points: labels.len(),
        });

        assert!(graph.adjacency[0]
            .iter()
            .any(|&(label_id, _)| label_id == 1));
        assert!(!graph.adjacency[0]
            .iter()
            .any(|&(label_id, _)| label_id == 2));
    }
}
