use okpalette::{
    extend_palette, generate_palette, BackgroundContrast, CandidateConstraints, ChromaRange,
    DistanceWeights, GridSize, LabelPaletteRequest, LightnessRange, OkPaletteError,
    PaletteGenerator, Rgb8,
};

fn fast_generator() -> PaletteGenerator {
    PaletteGenerator::new().grid_size(GridSize::try_step(64).unwrap())
}

#[test]
fn one_shot_api_generates_and_extends_palettes() {
    let palette = generate_palette(3).unwrap();
    assert_eq!(
        palette.iter().map(ToString::to_string).collect::<Vec<_>>(),
        ["#000058", "#90ff00", "#ff38ff"]
    );

    let extended = extend_palette(&palette[..2], 4).unwrap();
    assert_eq!(&extended[..2], &palette[..2]);
    assert_eq!(extended.len(), 4);
}

#[test]
fn generator_supports_typed_common_options_and_extensions() {
    let red = Rgb8::new(255, 0, 0);
    let white = Rgb8::new(255, 255, 255);
    let generator = fast_generator()
        .seed_colors([red])
        .backgrounds([white], BackgroundContrast::Normal)
        .unwrap()
        .constraints(
            CandidateConstraints::new()
                .with_lightness(LightnessRange::new(0.2, 0.9).unwrap())
                .with_chroma(ChromaRange::new(Some(0.04), None).unwrap()),
        )
        .distance_weights(DistanceWeights::new(0.8, 1.2).unwrap());

    let palette = generator.generate(3).unwrap();
    assert_eq!(palette.len(), 3);
    assert!(!palette.contains(&red));
    assert!(!palette.contains(&white));

    let extended = generator.extend(&[red], 3).unwrap();
    assert_eq!(extended[0], red);
    assert_eq!(extended.len(), 3);

    let generated_only = generator.generate_extension(&[red], 2).unwrap();
    assert_eq!(generated_only.len(), 2);
    assert!(!generated_only.contains(&red));
}

#[test]
fn generator_rejects_an_extension_target_smaller_than_the_palette() {
    let error = fast_generator()
        .extend(&[Rgb8::new(255, 0, 0), Rgb8::new(0, 0, 255)], 1)
        .unwrap_err();

    assert!(error.to_string().contains("target_size"));
}

#[test]
fn invalid_generator_configuration_is_rejected_at_construction() {
    assert!(matches!(
        GridSize::try_step(0),
        Err(OkPaletteError::InvalidGridStep)
    ));
    assert!(matches!(
        PaletteGenerator::new().backgrounds([], BackgroundContrast::Normal),
        Err(OkPaletteError::InvalidConstraintRange {
            constraint: "background",
            ..
        })
    ));
}

#[test]
fn generator_supports_position_aware_label_palettes() {
    let coordinates = [0.0, 0.0, 0.1, 0.0, 4.0, 4.0];
    let label_ids = [0, 1, 2];
    let fixed = [None, Some(Rgb8::new(255, 0, 0)), None];
    let request = LabelPaletteRequest::new(&coordinates, 2, &label_ids, 3)
        .fixed_colors(&fixed)
        .neighbors(2);

    let palette = fast_generator().generate_for_labels(request).unwrap();
    assert_eq!(palette.len(), 3);
    assert_eq!(palette[1], Rgb8::new(255, 0, 0));
}

#[test]
fn rgb_colors_parse_and_format_as_hex() {
    let color = "#0fA".parse::<Rgb8>().unwrap();

    assert_eq!(color, Rgb8::new(0, 255, 170));
    assert_eq!(color.to_string(), "#00ffaa");
}
