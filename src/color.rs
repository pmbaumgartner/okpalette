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
            (
                Rgb8 { r: 0, g: 0, b: 0 },
                Oklab {
                    l: 0.0,
                    a: 0.0,
                    b: 0.0,
                },
            ),
            (
                Rgb8 {
                    r: 255,
                    g: 255,
                    b: 255,
                },
                Oklab {
                    l: 1.0,
                    a: 0.0,
                    b: 0.0,
                },
            ),
            (
                Rgb8 { r: 255, g: 0, b: 0 },
                Oklab {
                    l: 0.627_955,
                    a: 0.224_863,
                    b: 0.125_846,
                },
            ),
            (
                Rgb8 { r: 0, g: 255, b: 0 },
                Oklab {
                    l: 0.866_440,
                    a: -0.233_888,
                    b: 0.179_498,
                },
            ),
            (
                Rgb8 { r: 0, g: 0, b: 255 },
                Oklab {
                    l: 0.452_014,
                    a: -0.032_457,
                    b: -0.311_528,
                },
            ),
        ];

        for (rgb, expected) in cases {
            assert_oklab_approx(rgb.to_oklab(), expected);
        }
    }

    #[test]
    fn converts_oklab_to_oklch() {
        let oklch = Oklab {
            l: 0.25,
            a: 3.0,
            b: 4.0,
        }
        .to_oklch();

        assert_approx(oklch.l, 0.25);
        assert_approx(oklch.c, 5.0);
        assert_approx(oklch.h, 53.130_104);
    }

    #[test]
    fn normalizes_oklch_hue() {
        let oklch = Oklab {
            l: 0.25,
            a: 0.0,
            b: -1.0,
        }
        .to_oklch();

        assert_approx(oklch.h, 270.0);
    }

    #[test]
    fn formats_lowercase_hex() {
        assert_eq!(
            Rgb8 {
                r: 0,
                g: 15,
                b: 170
            }
            .to_hex(),
            "#000faa"
        );
        assert_eq!(
            Rgb8 {
                r: 255,
                g: 128,
                b: 1
            }
            .to_hex(),
            "#ff8001"
        );
    }
}
