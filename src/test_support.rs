use crate::color::{Oklab, Rgb8};

pub(crate) fn rgb(r: u8, g: u8, b: u8) -> Rgb8 {
    Rgb8 { r, g, b }
}

pub(crate) fn lab(l: f32, a: f32, b: f32) -> Oklab {
    Oklab { l, a, b }
}

pub(crate) fn assert_unique_rgb(colors: &[Rgb8]) {
    for (index, color) in colors.iter().enumerate() {
        assert!(
            !colors[index + 1..].contains(color),
            "duplicate color in palette: {color:?}"
        );
    }
}

pub(crate) fn assert_png_dimensions(bytes: &[u8], width: u32, height: u32) {
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    assert_eq!(u32::from_be_bytes(bytes[16..20].try_into().unwrap()), width);
    assert_eq!(
        u32::from_be_bytes(bytes[20..24].try_into().unwrap()),
        height
    );
}

pub(crate) fn assert_canonical_hex_palette(colors: &[String], size: usize) {
    assert_eq!(colors.len(), size);
    assert!(colors.iter().all(|color| {
        color.len() == 7
            && color.starts_with('#')
            && color.chars().skip(1).all(|ch| ch.is_ascii_hexdigit())
            && color == &color.to_lowercase()
    }));
}

pub(crate) fn separated_label_fixture() -> ([f64; 8], [usize; 4], [Option<Rgb8>; 4]) {
    (
        [0.0, 0.0, 10.0, 0.0, 0.1, 0.0, 10.1, 0.0],
        [0, 1, 2, 3],
        [None, None, None, None],
    )
}
