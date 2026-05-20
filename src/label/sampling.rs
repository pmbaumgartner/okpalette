#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct SamplePoint {
    pub(super) original_index: usize,
    pub(super) label_id: usize,
}

pub(super) fn deterministic_sample(
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
