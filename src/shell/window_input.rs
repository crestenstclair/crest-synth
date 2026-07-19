/// A platform-independent key understood at the application window boundary.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum WindowKey {
    W,
    S,
    A,
    D,
    K,
    Other,
}

/// The normalized kind of a window-boundary input.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum WindowInputKind {
    KeyDown,
    KeyUp,
    FocusLost,
}

/// A normalized window-boundary input shared by native and deterministic adapters.
///
/// Platform key codes are converted to `WindowKey` values before this type is
/// constructed. The value contains no application, projection, parameter, or
/// audio state and must be translated into an application event before it can
/// reach the control layer.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct WindowInput {
    key: WindowKey,
    kind: WindowInputKind,
}

/// The complete normalized vocabulary accepted at the window boundary.
///
/// Entries are concrete production values so deterministic adapters can feed
/// them directly through the same translator as the native window.
const WINDOW_INPUT_SURFACE_DESCRIPTOR: [WindowInput; 13] = [
    WindowInput::key_down(WindowKey::W),
    WindowInput::key_down(WindowKey::S),
    WindowInput::key_down(WindowKey::A),
    WindowInput::key_down(WindowKey::D),
    WindowInput::key_down(WindowKey::K),
    WindowInput::key_down(WindowKey::Other),
    WindowInput::key_up(WindowKey::W),
    WindowInput::key_up(WindowKey::S),
    WindowInput::key_up(WindowKey::A),
    WindowInput::key_up(WindowKey::D),
    WindowInput::key_up(WindowKey::K),
    WindowInput::key_up(WindowKey::Other),
    WindowInput::focus_lost(),
];

impl WindowInput {
    /// Returns all 13 unique valid normalized input values.
    ///
    /// This production-owned descriptor is the only exhaustive GUI-input
    /// vocabulary deterministic scenes and acceptance tests need to consume.
    pub const fn surface_descriptor() -> &'static [Self] {
        &WINDOW_INPUT_SURFACE_DESCRIPTOR
    }

    /// Creates normalized input data.
    ///
    /// Focus loss is not associated with a physical key, so its key is always
    /// canonicalized to `WindowKey::Other`.
    pub const fn new(key: WindowKey, kind: WindowInputKind) -> Self {
        let key = match kind {
            WindowInputKind::KeyDown | WindowInputKind::KeyUp => key,
            WindowInputKind::FocusLost => WindowKey::Other,
        };

        Self { key, kind }
    }

    /// Creates a normalized key-down input.
    pub const fn key_down(key: WindowKey) -> Self {
        Self::new(key, WindowInputKind::KeyDown)
    }

    /// Creates a normalized key-up input.
    pub const fn key_up(key: WindowKey) -> Self {
        Self::new(key, WindowInputKind::KeyUp)
    }

    /// Creates a normalized focus-loss input.
    pub const fn focus_lost() -> Self {
        Self::new(WindowKey::Other, WindowInputKind::FocusLost)
    }

    /// Returns the normalized key.
    pub const fn key(&self) -> WindowKey {
        self.key
    }

    /// Returns the normalized input kind.
    pub const fn kind(&self) -> WindowInputKind {
        self.kind
    }
}

#[cfg(test)]
mod tests {
    use super::{WindowInput, WindowInputKind, WindowKey};

    #[test]
    fn surface_descriptor_contains_exactly_the_normalized_vocabulary() {
        let descriptor = WindowInput::surface_descriptor();

        assert_eq!(descriptor.len(), 13);
        for (index, input) in descriptor.iter().enumerate() {
            assert!(
                !descriptor[..index].contains(input),
                "duplicate descriptor entry: {input:?}"
            );
        }

        for key in [
            WindowKey::W,
            WindowKey::S,
            WindowKey::A,
            WindowKey::D,
            WindowKey::K,
            WindowKey::Other,
        ] {
            assert!(descriptor.contains(&WindowInput::key_down(key)));
            assert!(descriptor.contains(&WindowInput::key_up(key)));
        }

        let focus_lost = descriptor
            .iter()
            .filter(|input| input.kind() == WindowInputKind::FocusLost)
            .copied()
            .collect::<Vec<_>>();
        assert_eq!(focus_lost, vec![WindowInput::focus_lost()]);
        assert_eq!(focus_lost[0].key(), WindowKey::Other);
    }

    #[test]
    fn key_down_preserves_every_normalized_key() {
        let keys = [
            WindowKey::W,
            WindowKey::S,
            WindowKey::A,
            WindowKey::D,
            WindowKey::K,
            WindowKey::Other,
        ];

        for key in keys {
            let input = WindowInput::key_down(key);
            assert_eq!(input.key(), key);
            assert_eq!(input.kind(), WindowInputKind::KeyDown);
        }
    }

    #[test]
    fn key_up_preserves_every_normalized_key() {
        let keys = [
            WindowKey::W,
            WindowKey::S,
            WindowKey::A,
            WindowKey::D,
            WindowKey::K,
            WindowKey::Other,
        ];

        for key in keys {
            let input = WindowInput::key_up(key);
            assert_eq!(input.key(), key);
            assert_eq!(input.kind(), WindowInputKind::KeyUp);
        }
    }

    #[test]
    fn focus_loss_has_one_canonical_representation() {
        let canonical = WindowInput::focus_lost();
        let normalized = WindowInput::new(WindowKey::W, WindowInputKind::FocusLost);

        assert_eq!(canonical, normalized);
        assert_eq!(canonical.key(), WindowKey::Other);
        assert_eq!(canonical.kind(), WindowInputKind::FocusLost);
    }

    #[test]
    fn normalized_input_is_copyable_shell_data() {
        let original = WindowInput::key_down(WindowKey::K);
        let copied = original;

        assert_eq!(copied, original);
    }
}

