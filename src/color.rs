use std::{fmt, str::FromStr};

use crate::error::{GlasbeyError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// An 8-bit sRGB color.
pub struct Rgb8 {
    /// Red channel in `0..=255`.
    pub r: u8,
    /// Green channel in `0..=255`.
    pub g: u8,
    /// Blue channel in `0..=255`.
    pub b: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// A color in OKLab space.
pub struct Oklab {
    /// Perceptual lightness.
    pub l: f32,
    /// Green-red opponent axis.
    pub a: f32,
    /// Blue-yellow opponent axis.
    pub b: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// A color in cylindrical OKLCH space.
pub struct Oklch {
    /// Perceptual lightness.
    pub l: f32,
    /// Chroma.
    pub c: f32,
    /// Hue angle in degrees in `0.0..360.0`.
    pub h: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
/// Color-vision-deficiency simulations included when comparing colors.
///
/// String parsing accepts `"protan"`, `"deutan"`, `"tritan"`,
/// `"red-green"` (or `"daltonism"`), and `"all"`.
pub enum ColorblindMode {
    /// Compare ordinary sRGB colors only.
    #[default]
    None,
    /// Include a protanopia simulation.
    Protan,
    /// Include a deuteranopia simulation.
    Deutan,
    /// Include a tritanopia simulation.
    Tritan,
    /// Include both protanopia and deuteranopia simulations.
    RedGreen,
    /// Include protanopia, deuteranopia, and tritanopia simulations.
    All,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ColorProfile {
    components: [Oklab; 4],
    len: u8,
}

impl Rgb8 {
    /// Construct a color from red, green, and blue byte values.
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Convert this sRGB color to OKLab.
    pub fn to_oklab(self) -> Oklab {
        let r = srgb_channel_to_linear(self.r);
        let g = srgb_channel_to_linear(self.g);
        let b = srgb_channel_to_linear(self.b);

        linear_rgb_to_oklab(r, g, b)
    }

    /// Format this color as a lowercase `#rrggbb` string.
    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

impl From<[u8; 3]> for Rgb8 {
    fn from([r, g, b]: [u8; 3]) -> Self {
        Self::new(r, g, b)
    }
}

impl From<(u8, u8, u8)> for Rgb8 {
    fn from((r, g, b): (u8, u8, u8)) -> Self {
        Self::new(r, g, b)
    }
}

impl FromStr for Rgb8 {
    type Err = GlasbeyError;

    fn from_str(value: &str) -> Result<Self> {
        crate::parse::parse_hex_color(value)
    }
}

impl fmt::Display for Rgb8 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

impl FromStr for ColorblindMode {
    type Err = GlasbeyError;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "protan" => Ok(Self::Protan),
            "deutan" => Ok(Self::Deutan),
            "tritan" => Ok(Self::Tritan),
            "red-green" | "daltonism" => Ok(Self::RedGreen),
            "all" => Ok(Self::All),
            _ => Err(GlasbeyError::InvalidConstraintRange {
                constraint: "colorblind_mode",
                message: "must be 'protan', 'deutan', 'tritan', 'red-green', 'daltonism', or 'all'",
            }),
        }
    }
}

impl ColorblindMode {
    pub(crate) fn includes_protan(self) -> bool {
        matches!(self, Self::Protan | Self::RedGreen | Self::All)
    }

    pub(crate) fn includes_deutan(self) -> bool {
        matches!(self, Self::Deutan | Self::RedGreen | Self::All)
    }

    pub(crate) fn includes_tritan(self) -> bool {
        matches!(self, Self::Tritan | Self::All)
    }
}

impl ColorProfile {
    pub(crate) fn from_rgb(rgb: Rgb8, colorblind_mode: ColorblindMode) -> Self {
        Self::from_rgb_and_normal(rgb, rgb.to_oklab(), colorblind_mode)
    }

    pub(crate) fn from_rgb_and_normal(
        rgb: Rgb8,
        normal: Oklab,
        colorblind_mode: ColorblindMode,
    ) -> Self {
        let mut components = [normal; 4];
        let mut len = 1;

        for matrix in [
            colorblind_mode.includes_protan().then_some(PROTAN_MATRIX),
            colorblind_mode.includes_deutan().then_some(DEUTAN_MATRIX),
            colorblind_mode.includes_tritan().then_some(TRITAN_MATRIX),
        ]
        .into_iter()
        .flatten()
        {
            components[len] = simulate_machado_oklab(rgb, matrix);
            len += 1;
        }

        Self {
            components,
            len: len as u8,
        }
    }

    pub(crate) fn components(&self) -> &[Oklab] {
        &self.components[..usize::from(self.len)]
    }
}

pub(crate) fn relative_luminance_srgb(rgb: Rgb8) -> f64 {
    0.2126 * srgb_channel_to_linear_f64(rgb.r)
        + 0.7152 * srgb_channel_to_linear_f64(rgb.g)
        + 0.0722 * srgb_channel_to_linear_f64(rgb.b)
}

pub(crate) fn wcag_contrast_ratio(left: Rgb8, right: Rgb8) -> f64 {
    wcag_contrast_ratio_from_luminance(
        relative_luminance_srgb(left),
        relative_luminance_srgb(right),
    )
}

pub(crate) fn wcag_contrast_ratio_from_luminance(left_luminance: f64, right_luminance: f64) -> f64 {
    let light = left_luminance.max(right_luminance);
    let dark = left_luminance.min(right_luminance);

    (light + 0.05) / (dark + 0.05)
}

impl Oklab {
    /// Convert this color to cylindrical OKLCH.
    pub fn to_oklch(self) -> Oklch {
        Oklch {
            l: self.l,
            c: self.a.hypot(self.b),
            h: self.b.atan2(self.a).to_degrees().rem_euclid(360.0),
        }
    }
}

fn srgb_channel_to_linear(channel: u8) -> f32 {
    let value = f32::from(channel) / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_rgb_to_oklab(r: f32, g: f32, b: f32) -> Oklab {
    let linear_l = 0.412_221_46 * r + 0.536_332_55 * g + 0.051_445_995 * b;
    let linear_m = 0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b;
    let linear_s = 0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b;

    let l_ = linear_l.cbrt();
    let m_ = linear_m.cbrt();
    let s_ = linear_s.cbrt();

    Oklab {
        l: 0.210_454_26 * l_ + 0.793_617_8 * m_ - 0.004_072_047 * s_,
        a: 1.977_998_5 * l_ - 2.428_592_2 * m_ + 0.450_593_7 * s_,
        b: 0.025_904_037 * l_ + 0.782_771_77 * m_ - 0.808_675_77 * s_,
    }
}

const PROTAN_MATRIX: [[f32; 3]; 3] = [
    [0.152_286, 1.052_583, -0.204_868],
    [0.114_503, 0.786_281, 0.099_216],
    [-0.003_882, -0.048_116, 1.051_998],
];

const DEUTAN_MATRIX: [[f32; 3]; 3] = [
    [0.367_322, 0.860_646, -0.227_968],
    [0.280_085, 0.672_501, 0.047_413],
    [-0.011_820, 0.042_940, 0.968_881],
];

const TRITAN_MATRIX: [[f32; 3]; 3] = [
    [1.255_528, -0.076_749, -0.178_779],
    [-0.078_411, 0.930_809, 0.147_602],
    [0.004_733, 0.691_367, 0.303_900],
];

fn simulate_machado_oklab(rgb: Rgb8, matrix: [[f32; 3]; 3]) -> Oklab {
    let (r, g, b) = simulate_machado_linear_rgb(rgb, matrix);
    linear_rgb_to_oklab(r, g, b)
}

fn simulate_machado_linear_rgb(rgb: Rgb8, matrix: [[f32; 3]; 3]) -> (f32, f32, f32) {
    let r = srgb_channel_to_linear(rgb.r);
    let g = srgb_channel_to_linear(rgb.g);
    let b = srgb_channel_to_linear(rgb.b);

    (
        transformed_channel(matrix[0], r, g, b),
        transformed_channel(matrix[1], r, g, b),
        transformed_channel(matrix[2], r, g, b),
    )
}

fn transformed_channel(row: [f32; 3], r: f32, g: f32, b: f32) -> f32 {
    (row[0] * r + row[1] * g + row[2] * b).clamp(0.0, 1.0)
}

fn srgb_channel_to_linear_f64(channel: u8) -> f64 {
    let value = f64::from(channel) / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{lab, rgb};

    fn assert_approx(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= 0.000_01,
            "expected {actual} to be within tolerance of {expected}"
        );
    }

    fn assert_approx_f64(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= 0.000_01,
            "expected {actual} to be within tolerance of {expected}"
        );
    }

    fn assert_oklab_approx(actual: Oklab, expected: Oklab) {
        assert_approx(actual.l, expected.l);
        assert_approx(actual.a, expected.a);
        assert_approx(actual.b, expected.b);
    }

    fn assert_linear_rgb_approx(actual: (f32, f32, f32), expected: (f32, f32, f32)) {
        assert_approx(actual.0, expected.0);
        assert_approx(actual.1, expected.1);
        assert_approx(actual.2, expected.2);
    }

    #[test]
    fn converts_rgb_to_oklab_snapshots() {
        let cases = [
            (rgb(0, 0, 0), lab(0.0, 0.0, 0.0)),
            (rgb(255, 255, 255), lab(1.0, 0.0, 0.0)),
            (rgb(255, 0, 0), lab(0.627_955, 0.224_863, 0.125_846)),
            (rgb(0, 255, 0), lab(0.866_440, -0.233_888, 0.179_498)),
            (rgb(0, 0, 255), lab(0.452_014, -0.032_457, -0.311_528)),
        ];

        for (rgb, expected) in cases {
            assert_oklab_approx(rgb.to_oklab(), expected);
        }
    }

    #[test]
    fn converts_oklab_to_oklch() {
        let oklch = lab(0.25, 3.0, 4.0).to_oklch();

        assert_approx(oklch.l, 0.25);
        assert_approx(oklch.c, 5.0);
        assert_approx(oklch.h, 53.130_104);
    }

    #[test]
    fn normalizes_oklch_hue() {
        let oklch = lab(0.25, 0.0, -1.0).to_oklch();

        assert_approx(oklch.h, 270.0);
    }

    #[test]
    fn formats_lowercase_hex() {
        assert_eq!(rgb(0, 15, 170).to_hex(), "#000faa");
        assert_eq!(rgb(255, 128, 1).to_hex(), "#ff8001");
    }

    #[test]
    fn computes_relative_luminance_snapshots() {
        assert_approx_f64(relative_luminance_srgb(rgb(0, 0, 0)), 0.0);
        assert_approx_f64(relative_luminance_srgb(rgb(255, 255, 255)), 1.0);
        assert_approx_f64(relative_luminance_srgb(rgb(127, 127, 127)), 0.212_231);
    }

    #[test]
    fn computes_wcag_contrast_ratio_snapshots() {
        assert_approx_f64(wcag_contrast_ratio(rgb(0, 0, 0), rgb(255, 255, 255)), 21.0);
        assert_approx_f64(wcag_contrast_ratio(rgb(255, 0, 0), rgb(255, 0, 0)), 1.0);
        assert_approx_f64(
            wcag_contrast_ratio(rgb(255, 255, 255), rgb(0, 0, 0)),
            wcag_contrast_ratio(rgb(0, 0, 0), rgb(255, 255, 255)),
        );
    }

    #[test]
    fn parses_colorblind_modes() {
        assert_eq!("protan".parse(), Ok(ColorblindMode::Protan));
        assert_eq!("deutan".parse(), Ok(ColorblindMode::Deutan));
        assert_eq!("tritan".parse(), Ok(ColorblindMode::Tritan));
        assert_eq!("red-green".parse(), Ok(ColorblindMode::RedGreen));
        assert_eq!("daltonism".parse(), Ok(ColorblindMode::RedGreen));
        assert_eq!("all".parse(), Ok(ColorblindMode::All));
        assert!(matches!(
            "protanopia".parse::<ColorblindMode>(),
            Err(GlasbeyError::InvalidConstraintRange {
                constraint: "colorblind_mode",
                ..
            })
        ));
    }

    #[test]
    fn applies_machado_severity_one_linear_rgb_snapshots() {
        assert_linear_rgb_approx(
            simulate_machado_linear_rgb(rgb(255, 0, 0), PROTAN_MATRIX),
            (0.152_286, 0.114_503, 0.0),
        );
        assert_linear_rgb_approx(
            simulate_machado_linear_rgb(rgb(0, 255, 0), DEUTAN_MATRIX),
            (0.860_646, 0.672_501, 0.042_940),
        );
        assert_linear_rgb_approx(
            simulate_machado_linear_rgb(rgb(0, 0, 255), TRITAN_MATRIX),
            (0.0, 0.147_602, 0.303_900),
        );
    }

    #[test]
    fn color_profile_precomputes_only_selected_simulations() {
        let normal = ColorProfile::from_rgb(rgb(255, 0, 0), ColorblindMode::None);
        assert_oklab_approx(normal.components()[0], rgb(255, 0, 0).to_oklab());
        assert_eq!(normal.components().len(), 1);

        let protan = ColorProfile::from_rgb(rgb(255, 0, 0), ColorblindMode::Protan);
        assert_eq!(protan.components().len(), 2);

        let red_green = ColorProfile::from_rgb(rgb(255, 0, 0), ColorblindMode::RedGreen);
        assert_eq!(red_green.components().len(), 3);

        let all = ColorProfile::from_rgb(rgb(255, 0, 0), ColorblindMode::All);
        assert_eq!(all.components().len(), 4);
    }
}
