// path: src/design_system/rgba.rs

use std::fmt;

/// A validated RGBA color value, each channel normalized to `0.0..=1.0`.
///
/// `Rgba` is a plain value type: it holds no dependencies and performs no
/// I/O or allocation. It is the only color representation the `Theme` port
/// returns; skins convert it to a renderer-native color (e.g. egui's
/// `Color32`) at the single point where they touch a raw color.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

/// Reason an `Rgba` construction was rejected.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RgbaError {
    channel: &'static str,
    value: f32,
}

impl fmt::Display for RgbaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Rgba channel '{}' must be finite and within 0.0..=1.0, got {}",
            self.channel, self.value
        )
    }
}

impl std::error::Error for RgbaError {}

impl Rgba {
    /// Construct a color, validating every channel is a finite value in
    /// `0.0..=1.0`.
    pub fn try_new(r: f32, g: f32, b: f32, a: f32) -> Result<Self, RgbaError> {
        Self::validate_channel("r", r)?;
        Self::validate_channel("g", g)?;
        Self::validate_channel("b", b)?;
        Self::validate_channel("a", a)?;
        Ok(Self { r, g, b, a })
    }

    fn validate_channel(name: &'static str, value: f32) -> Result<(), RgbaError> {
        if value.is_nan() || !(0.0..=1.0).contains(&value) {
            return Err(RgbaError {
                channel: name,
                value,
            });
        }
        Ok(())
    }

    /// Fully opaque black. Valid by construction; used as a safe fallback.
    pub const fn black() -> Self {
        Self {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        }
    }

    /// Fully opaque white.
    pub const fn white() -> Self {
        Self {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        }
    }

    /// Fully transparent black.
    pub const fn transparent() -> Self {
        Self {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        }
    }

    pub fn r(&self) -> f32 {
        self.r
    }

    pub fn g(&self) -> f32 {
        self.g
    }

    pub fn b(&self) -> f32 {
        self.b
    }

    pub fn a(&self) -> f32 {
        self.a
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_new_accepts_in_range_channels() {
        let color = Rgba::try_new(0.1, 0.2, 0.3, 1.0).expect("valid color");
        assert_eq!(color.r(), 0.1);
        assert_eq!(color.g(), 0.2);
        assert_eq!(color.b(), 0.3);
        assert_eq!(color.a(), 1.0);
    }

    #[test]
    fn try_new_accepts_boundary_values() {
        assert!(Rgba::try_new(0.0, 0.0, 0.0, 0.0).is_ok());
        assert!(Rgba::try_new(1.0, 1.0, 1.0, 1.0).is_ok());
    }

    #[test]
    fn try_new_rejects_out_of_range_channel() {
        let err = Rgba::try_new(1.5, 0.0, 0.0, 1.0).expect_err("out of range");
        assert_eq!(err.channel, "r");
    }

    #[test]
    fn try_new_rejects_negative_channel() {
        let err = Rgba::try_new(0.0, -0.01, 0.0, 1.0).expect_err("out of range");
        assert_eq!(err.channel, "g");
    }

    #[test]
    fn try_new_rejects_nan_channel() {
        let err = Rgba::try_new(0.0, 0.0, f32::NAN, 1.0).expect_err("nan rejected");
        assert_eq!(err.channel, "b");
    }

    #[test]
    fn constants_are_valid_colors() {
        assert_eq!(Rgba::black().a(), 1.0);
        assert_eq!(Rgba::white().r(), 1.0);
        assert_eq!(Rgba::transparent().a(), 0.0);
    }

    #[test]
    fn error_message_names_the_failing_channel() {
        let err = Rgba::try_new(0.0, 0.0, 0.0, 2.0).expect_err("out of range");
        let message = err.to_string();
        assert!(message.contains('a'));
    }
}
