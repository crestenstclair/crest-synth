// path: src/design_system/rgba.rs

/// An 8-bit straight-alpha RGBA color.
///
/// This is the raw value a `SemanticToken` resolves to, and the only place a
/// literal color lives. Skin code converts this into the renderer's native
/// color type (e.g. `egui::Color32`) through the `Into` impl — never by
/// constructing a color literal in draw code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    /// Construct from individual 8-bit channels.
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Fully opaque black.
    pub const BLACK: Rgba = Rgba::new(0, 0, 0, 255);

    /// Fully opaque white.
    pub const WHITE: Rgba = Rgba::new(255, 255, 255, 255);

    /// Fully transparent black.
    pub const TRANSPARENT: Rgba = Rgba::new(0, 0, 0, 0);
}

/// Convert to egui's `Color32` so skin drawing code can pass `Rgba` values
/// directly to egui without ever writing a literal `Color32` in draw code.
impl From<Rgba> for egui::Color32 {
    fn from(c: Rgba) -> Self {
        egui::Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_channels() {
        let color = Rgba::new(10, 20, 30, 40);
        assert_eq!(color.r, 10);
        assert_eq!(color.g, 20);
        assert_eq!(color.b, 30);
        assert_eq!(color.a, 40);
    }

    #[test]
    fn constants_have_correct_values() {
        assert_eq!(Rgba::BLACK, Rgba::new(0, 0, 0, 255));
        assert_eq!(Rgba::WHITE, Rgba::new(255, 255, 255, 255));
        assert_eq!(Rgba::TRANSPARENT, Rgba::new(0, 0, 0, 0));
    }

    #[test]
    fn equality_is_channel_wise() {
        let a = Rgba::new(1, 2, 3, 4);
        let b = Rgba::new(1, 2, 3, 4);
        let c = Rgba::new(1, 2, 3, 5);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn into_egui_color32_preserves_channels() {
        let rgba = Rgba::new(100, 150, 200, 255);
        let c32: egui::Color32 = rgba.into();
        // egui::Color32::from_rgba_unmultiplied stores channels as-is
        assert_eq!(c32.r(), 100);
        assert_eq!(c32.g(), 150);
        assert_eq!(c32.b(), 200);
        assert_eq!(c32.a(), 255);
    }

    #[test]
    fn transparent_has_zero_alpha() {
        assert_eq!(Rgba::TRANSPARENT.a, 0);
    }
}
