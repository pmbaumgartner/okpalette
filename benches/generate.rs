use std::{hint::black_box, time::Duration};

use _core::algorithm::{select_palette, DistanceWeights, PaletteAnchors, PaletteOptions};
use _core::candidates::{generate_candidates, CandidateConstraints, GridSize};
use _core::color::{ColorblindMode, Rgb8};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode};

const BENCH_CASES: &[(usize, &str, GridSize)] = &[
    (32, "medium", GridSize::Medium),
    (32, "fine", GridSize::Fine),
    (256, "medium", GridSize::Medium),
    (256, "fine", GridSize::Fine),
];

fn default_constraints() -> CandidateConstraints {
    CandidateConstraints {
        lightness: Some((0.20, 0.90)),
        chroma: Some((Some(0.04), None)),
        hue: None,
    }
}

fn default_options(palette_size: usize) -> PaletteOptions<'static> {
    PaletteOptions {
        palette_size,
        anchors: PaletteAnchors::default(),
        weights: DistanceWeights::default(),
        colorblind_mode: ColorblindMode::None,
    }
}

fn generate_palette_hex(palette_size: usize, grid_size: GridSize) -> Vec<String> {
    let candidates = generate_candidates(grid_size, default_constraints(), palette_size)
        .expect("benchmark constraints should leave enough candidates");
    let palette = select_palette(&candidates, default_options(palette_size))
        .expect("benchmark palette generation should succeed");

    palette.into_iter().map(Rgb8::to_hex).collect()
}

fn bench_generate_palette(c: &mut Criterion) {
    let mut group = c.benchmark_group("generate_palette");
    group.sample_size(10);
    group.sampling_mode(SamplingMode::Flat);
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(5));

    for &(palette_size, grid_name, grid_size) in BENCH_CASES {
        group.bench_with_input(
            BenchmarkId::new(grid_name, palette_size),
            &(palette_size, grid_size),
            |b, &(palette_size, grid_size)| {
                b.iter(|| generate_palette_hex(black_box(palette_size), black_box(grid_size)));
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_generate_palette);
criterion_main!(benches);
