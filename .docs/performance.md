---
kata: a3vs
created: 2026-05-22
---

# Performance

This project tracks palette-generation performance with non-gating Criterion
benchmarks. The benchmark target exercises the Rust core directly, so it does
not require Python packaging, maturin, or an installed `okpalette` wheel.

Run the benchmark with:

```bash
cargo bench --bench generate
```

To compare two local revisions with Criterion baselines:

```bash
cargo bench --bench generate -- --save-baseline before
# make the change
cargo bench --bench generate -- --baseline before
```

## Benchmarked Cases

The `generate_palette` benchmark covers the representative cases from the
roadmap:

| Case | Grid step | Goal |
| --- | ---: | ---: |
| 32 colors, medium grid | 8 | < 50 ms |
| 32 colors, fine grid | 4 | Track only |
| 256 colors, medium grid | 8 | < 500 ms |
| 256 colors, fine grid | 4 | < 3 s |

These goals are development targets, not CI gates. CI should compile the
benchmark through `cargo clippy --all-targets --all-features -- -D warnings`,
but it should not fail on wall-clock benchmark results because hosted runners
vary too much.

## Current Baseline

No previous benchmark harness existed before `a3vs`, so this issue establishes
the first comparable baseline.

Local baseline captured on 2026-05-22:

```text
command: cargo bench --bench generate
platform: Darwin arm64
rustc: rustc 1.93.1 (01f6ddf75 2026-02-11)
criterion: sample_size=10, flat sampling, 500 ms warmup, 5 s measurement target
```

| Case | Observed interval | Goal status |
| --- | ---: | --- |
| 32 colors, medium grid | 9.8618-10.828 ms | Under 50 ms target |
| 32 colors, fine grid | 54.872-68.988 ms | Tracked only |
| 256 colors, medium grid | 81.150-103.34 ms | Under 500 ms target |
| 256 colors, fine grid | 382.17-569.00 ms | Under 3 s target |

The short local run is useful for regression tracking, but the confidence
intervals are wide enough that release decisions should compare same-machine
Criterion baselines instead of relying on one absolute run.

## Interpreting Results

The benchmark includes candidate grid generation, OKLab conversion,
farthest-point palette selection, and conversion of selected colors to hex
strings. It intentionally skips the Python wrapper so the result can isolate
the Rust generation path.

If a case misses its target, the likely first bottleneck is repeated full-grid
distance updates during farthest-point selection. The next optimization path is
to profile candidate generation versus selection, then consider candidate-pool
caching, parallel tuning, or nearest-neighbor indexing only when the same
benchmark shows a stable regression or shortfall.
