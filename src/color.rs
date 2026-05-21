use crate::error::{GlasbeyError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Oklab {
    pub l: f32,
    pub a: f32,
    pub b: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Oklch {
    pub l: f32,
    pub c: f32,
    pub h: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ColorblindMode {
    #[default]
    None,
    Protan,
    Deutan,
    Tritan,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ColorProfile {
    pub normal: Oklab,
    pub protan: Option<Oklab>,
    pub deutan: Option<Oklab>,
    pub tritan: Option<Oklab>,
}

impl Rgb8 {
    pub fn to_oklab(self) -> Oklab {
        let r = srgb_channel_to_linear(self.r);
        let g = srgb_channel_to_linear(self.g);
        let b = srgb_channel_to_linear(self.b);

        linear_rgb_to_oklab(r, g, b)
    }

    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

impl ColorblindMode {
    pub fn parse(value: Option<&str>) -> Result<Self> {
        match value {
            None => Ok(Self::None),
            Some("protan") => Ok(Self::Protan),
            Some("deutan") => Ok(Self::Deutan),
            Some("tritan") => Ok(Self::Tritan),
            Some("all") => Ok(Self::All),
            Some(_) => Err(GlasbeyError::InvalidConstraintRange {
                constraint: "colorblind_mode",
                message: "must be None, 'protan', 'deutan', 'tritan', or 'all'",
            }),
        }
    }

    pub(crate) fn includes_protan(self) -> bool {
        matches!(self, Self::Protan | Self::All)
    }

    pub(crate) fn includes_deutan(self) -> bool {
        matches!(self, Self::Deutan | Self::All)
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
        Self {
            normal,
            protan: colorblind_mode
                .includes_protan()
                .then(|| simulate_machado_oklab(rgb, PROTAN_MATRIX)),
            deutan: colorblind_mode
                .includes_deutan()
                .then(|| simulate_machado_oklab(rgb, DEUTAN_MATRIX)),
            tritan: colorblind_mode
                .includes_tritan()
                .then(|| simulate_machado_oklab(rgb, TRITAN_MATRIX)),
        }
    }
}

pub fn relative_luminance_srgb(rgb: Rgb8) -> f64 {
    0.2126 * srgb_channel_to_linear_f64(rgb.r)
        + 0.7152 * srgb_channel_to_linear_f64(rgb.g)
        + 0.0722 * srgb_channel_to_linear_f64(rgb.b)
}

pub fn wcag_contrast_ratio(left: Rgb8, right: Rgb8) -> f64 {
    let left_luminance = relative_luminance_srgb(left);
    let right_luminance = relative_luminance_srgb(right);
    let light = left_luminance.max(right_luminance);
    let dark = left_luminance.min(right_luminance);

    (light + 0.05) / (dark + 0.05)
}

impl Oklab {
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
        assert_eq!(ColorblindMode::parse(None), Ok(ColorblindMode::None));
        assert_eq!(
            ColorblindMode::parse(Some("protan")),
            Ok(ColorblindMode::Protan)
        );
        assert_eq!(
            ColorblindMode::parse(Some("deutan")),
            Ok(ColorblindMode::Deutan)
        );
        assert_eq!(
            ColorblindMode::parse(Some("tritan")),
            Ok(ColorblindMode::Tritan)
        );
        assert_eq!(ColorblindMode::parse(Some("all")), Ok(ColorblindMode::All));
        assert!(matches!(
            ColorblindMode::parse(Some("protanopia")),
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
        assert_oklab_approx(normal.normal, rgb(255, 0, 0).to_oklab());
        assert_eq!(normal.protan, None);
        assert_eq!(normal.deutan, None);
        assert_eq!(normal.tritan, None);

        let protan = ColorProfile::from_rgb(rgb(255, 0, 0), ColorblindMode::Protan);
        assert!(protan.protan.is_some());
        assert_eq!(protan.deutan, None);
        assert_eq!(protan.tritan, None);

        let all = ColorProfile::from_rgb(rgb(255, 0, 0), ColorblindMode::All);
        assert!(all.protan.is_some());
        assert!(all.deutan.is_some());
        assert!(all.tritan.is_some());
    }
}
