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

impl Rgb8 {
    pub fn to_oklab(self) -> Oklab {
        let r = srgb_channel_to_linear(self.r);
        let g = srgb_channel_to_linear(self.g);
        let b = srgb_channel_to_linear(self.b);

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

    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
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

    fn assert_oklab_approx(actual: Oklab, expected: Oklab) {
        assert_approx(actual.l, expected.l);
        assert_approx(actual.a, expected.a);
        assert_approx(actual.b, expected.b);
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
}
