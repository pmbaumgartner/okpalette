use crate::color::{ColorProfile, Oklab};
use crate::error::{GlasbeyError, Result};

/// Relative weights for squared distance in OKLab space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DistanceWeights {
    lightness: f32,
    chroma: f32,
}

impl DistanceWeights {
    /// Create validated lightness and chroma distance weights.
    pub fn new(lightness: f32, chroma: f32) -> Result<Self> {
        if !lightness.is_finite() || !chroma.is_finite() {
            return Err(GlasbeyError::InvalidDistanceWeights {
                message: "weights must be finite",
            });
        }

        if lightness < 0.0 || chroma < 0.0 {
            return Err(GlasbeyError::InvalidDistanceWeights {
                message: "weights must be greater than or equal to zero",
            });
        }

        if lightness == 0.0 && chroma == 0.0 {
            return Err(GlasbeyError::InvalidDistanceWeights {
                message: "at least one weight must be positive",
            });
        }

        Ok(Self { lightness, chroma })
    }

    /// Return the lightness-axis weight.
    pub const fn lightness(self) -> f32 {
        self.lightness
    }

    /// Return the chroma-plane weight.
    pub const fn chroma(self) -> f32 {
        self.chroma
    }

    pub(crate) fn oklab_distance_squared(self, left: Oklab, right: Oklab) -> f32 {
        let dl = left.l - right.l;
        let da = left.a - right.a;
        let db = left.b - right.b;

        self.lightness * dl * dl + self.chroma * (da * da + db * db)
    }

    pub(crate) fn color_profile_distance_squared(
        self,
        left: ColorProfile,
        right: ColorProfile,
    ) -> f32 {
        assert_eq!(
            left.components().len(),
            right.components().len(),
            "color profiles must use the same colorblind mode"
        );
        left.components()
            .iter()
            .zip(right.components())
            .map(|(&left, &right)| self.oklab_distance_squared(left, right))
            .reduce(f32::min)
            .expect("color profiles always contain the normal color")
    }
}

impl Default for DistanceWeights {
    fn default() -> Self {
        Self {
            lightness: 1.0,
            chroma: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_weights_at_construction() {
        for (lightness, chroma) in [
            (f32::NAN, 1.0),
            (f32::INFINITY, 1.0),
            (-1.0, 1.0),
            (0.0, 0.0),
        ] {
            assert!(matches!(
                DistanceWeights::new(lightness, chroma),
                Err(GlasbeyError::InvalidDistanceWeights { .. })
            ));
        }
    }
}
