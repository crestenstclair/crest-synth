// path: src/bin/gamepad_demo.rs

//! `gamepad_demo` — headless prover for `GamepadNavigator` + `GlyphResolver`.
//!
//! Takes no arguments and opens NO device and NO window. It is a headless
//! harness over the host-agnostic Shell domain services: a SCRIPTED,
//! deterministic sequence of raw gamepad events is fed through the real
//! `crate::shell::gamepad_navigator::GamepadNavigator`, which translates
//! them into the mapped `crate::shell::gamepad_action::GamepadAction`
//! vocabulary and drives this demo's OWN cursor/edit model (never egui's
//! built-in focus). Separately, the real
//! `crate::shell::glyph_resolver::GlyphResolver` is driven for two
//! different `ControllerType`s and its output is asserted to differ per
//! controller for the same logical button.
//!
//! No `gilrs`, `egui`, or `eframe` import appears anywhere in this file —
//! the `GamepadInput` port is satisfied here by a local scripted stub, not
//! a real hardware adapter.

use crest_synth::shell::controller_glyph::{ControllerType, GamepadButton as GlyphButton};
use crest_synth::shell::gamepad_action::{
    Direction, GamepadAction as MappedAction, ShoulderSide, StickAxis,
};
use crest_synth::shell::gamepad_input::{
    AxisValue, GamepadAction as RawAction, GamepadAxis, GamepadButton, GamepadId, GamepadInput,
};
use crest_synth::shell::gamepad_navigator::GamepadNavigator;
use crest_synth::shell::glyph_resolver::GlyphResolver;

/// This demo's own app cursor/edit model over a small grid of focusable
/// cells — exactly the kind of controller-first navigation state the
/// mixer and editor views maintain themselves rather than delegating to a
/// GUI framework's focus system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CursorModel {
    row: usize,
    col: usize,
    rows: usize,
    cols: usize,
    editing: bool,
    view_index: i32,
    patch_index: i32,
    session_saved: bool,
    browser_open: bool,
    last_selected: Option<(usize, usize)>,
}

impl CursorModel {
    fn new(rows: usize, cols: usize) -> Self {
        Self {
            row: 0,
            col: 0,
            rows,
            cols,
            editing: false,
            view_index: 0,
            patch_index: 0,
            session_saved: false,
            browser_open: false,
            last_selected: None,
        }
    }

    /// Applies a single mapped `GamepadAction`, mutating the cursor/edit
    /// model. This is the demo's own navigation reducer — it never
    /// touches any GUI framework's focus state.
    fn apply(&mut self, action: &MappedAction) {
        match action {
            MappedAction::Navigate(direction) => self.navigate(*direction),
            MappedAction::FineAdjust(_) => {
                // Fine-adjust affects the focused parameter's value, not
                // cursor position; nothing to do for this cursor model.
            }
            MappedAction::Scroll(_) => {
                // Scroll affects viewport offset, not the logical cursor
                // cell under test here.
            }
            MappedAction::Select => {
                self.editing = true;
                self.last_selected = Some((self.row, self.col));
            }
            MappedAction::Back => {
                self.editing = false;
            }
            MappedAction::SwitchView(side) => {
                self.view_index += match side {
                    ShoulderSide::Left => -1,
                    ShoulderSide::Right => 1,
                };
            }
            MappedAction::SwitchPatch(side) => {
                self.patch_index += match side {
                    ShoulderSide::Left => -1,
                    ShoulderSide::Right => 1,
                };
            }
            MappedAction::SaveSession => {
                self.session_saved = true;
            }
            MappedAction::OpenBrowser => {
                self.browser_open = true;
            }
        }
    }

    fn navigate(&mut self, direction: Direction) {
        match direction {
            Direction::Up => {
                self.row = self.row.saturating_sub(1);
            }
            Direction::Down => {
                if self.row + 1 < self.rows {
                    self.row += 1;
                }
            }
            Direction::Left => {
                self.col = self.col.saturating_sub(1);
            }
            Direction::Right => {
                if self.col + 1 < self.cols {
                    self.col += 1;
                }
            }
        }
    }
}

/// A deterministic `GamepadInput` stub: hands back one pre-scripted batch
/// of raw events the first time it is polled, then reports no further
/// input. This is the seam that lets `GamepadNavigator` — which is
/// generic over `GamepadInput` per Dependency Inversion / Liskov — be
/// driven headlessly with no real hardware.
struct ScriptedGamepadInput {
    batch: Option<Vec<RawAction>>,
}

impl ScriptedGamepadInput {
    fn new(events: Vec<RawAction>) -> Self {
        Self {
            batch: Some(events),
        }
    }
}

impl GamepadInput for ScriptedGamepadInput {
    fn poll(&mut self) -> Vec<RawAction> {
        self.batch.take().unwrap_or_default()
    }
}

/// Builds the deterministic, scripted sequence of raw gamepad events this
/// demo drives through the `GamepadNavigator`. Order matters: it is
/// chosen so the expected mapped actions and the expected final cursor
/// position can be hand-computed and asserted below.
fn scripted_events() -> Vec<RawAction> {
    let gamepad = GamepadId(0);
    vec![
        // Move down twice, right twice: (0,0) -> (2,2).
        RawAction::ButtonPressed {
            gamepad,
            button: GamepadButton::DPadDown,
        },
        RawAction::ButtonReleased {
            gamepad,
            button: GamepadButton::DPadDown,
        },
        RawAction::ButtonPressed {
            gamepad,
            button: GamepadButton::DPadDown,
        },
        RawAction::ButtonReleased {
            gamepad,
            button: GamepadButton::DPadDown,
        },
        RawAction::ButtonPressed {
            gamepad,
            button: GamepadButton::DPadRight,
        },
        RawAction::ButtonReleased {
            gamepad,
            button: GamepadButton::DPadRight,
        },
        RawAction::ButtonPressed {
            gamepad,
            button: GamepadButton::DPadRight,
        },
        RawAction::ButtonReleased {
            gamepad,
            button: GamepadButton::DPadRight,
        },
        // Fine-adjust via left stick (no cursor movement, but must map).
        RawAction::AxisMoved {
            gamepad,
            axis: GamepadAxis::LeftStickX,
            value: AxisValue::new(0.5),
        },
        // Select the focused cell.
        RawAction::ButtonPressed {
            gamepad,
            button: GamepadButton::South,
        },
        // Back out of edit mode.
        RawAction::ButtonPressed {
            gamepad,
            button: GamepadButton::East,
        },
        // Switch view right, switch patch left.
        RawAction::ButtonPressed {
            gamepad,
            button: GamepadButton::RightTrigger,
        },
        RawAction::ButtonPressed {
            gamepad,
            button: GamepadButton::LeftBumper,
        },
        // Save session and open the preset browser.
        RawAction::ButtonPressed {
            gamepad,
            button: GamepadButton::Start,
        },
        RawAction::ButtonPressed {
            gamepad,
            button: GamepadButton::Select,
        },
        // Three more down moves; the grid has only 3 rows so the cursor
        // model must clamp at row 2 while the navigator still dispatches
        // a Navigate action for every one of them.
        RawAction::ButtonPressed {
            gamepad,
            button: GamepadButton::DPadDown,
        },
        RawAction::ButtonPressed {
            gamepad,
            button: GamepadButton::DPadDown,
        },
        RawAction::ButtonPressed {
            gamepad,
            button: GamepadButton::DPadDown,
        },
        // Connect/disconnect noise that must translate to nothing.
        RawAction::Connected { gamepad },
        RawAction::Disconnected { gamepad },
    ]
}

/// Drives the scripted events through the real `GamepadNavigator`, asserts
/// the exact mapped-action sequence and the exact resulting cursor/edit
/// state, and returns the number of actions dispatched.
fn run_navigation_proof() -> usize {
    let input = ScriptedGamepadInput::new(scripted_events());
    let mut navigator = GamepadNavigator::new(input);
    let dispatched = navigator.poll_actions();

    let expected_actions = vec![
        MappedAction::Navigate(Direction::Down),
        MappedAction::Navigate(Direction::Down),
        MappedAction::Navigate(Direction::Right),
        MappedAction::Navigate(Direction::Right),
        MappedAction::FineAdjust(StickAxis::new(0.5, 0.0)),
        MappedAction::Select,
        MappedAction::Back,
        MappedAction::SwitchView(ShoulderSide::Right),
        MappedAction::SwitchPatch(ShoulderSide::Left),
        MappedAction::SaveSession,
        MappedAction::OpenBrowser,
        MappedAction::Navigate(Direction::Down),
        MappedAction::Navigate(Direction::Down),
        MappedAction::Navigate(Direction::Down),
    ];

    assert_eq!(
        dispatched, expected_actions,
        "scripted events produced unexpected GamepadActions: got {dispatched:?}, expected {expected_actions:?}"
    );

    let mut cursor = CursorModel::new(3, 3);
    for action in &dispatched {
        cursor.apply(action);
    }

    let expected_cursor = CursorModel {
        row: 2,
        col: 2,
        rows: 3,
        cols: 3,
        editing: false,
        view_index: 1,
        patch_index: -1,
        session_saved: true,
        browser_open: true,
        last_selected: Some((2, 2)),
    };

    assert_eq!(
        cursor, expected_cursor,
        "final cursor/edit model did not match expected state: got {cursor:?}, expected {expected_cursor:?}"
    );

    dispatched.len()
}

/// Drives the real `GlyphResolver` for two different `ControllerType`s and
/// asserts each resolves a DIFFERENT glyph for the same logical button.
fn run_glyph_proof() {
    let xbox_resolver = GlyphResolver::with_controller_type(ControllerType::Xbox);
    let playstation_resolver = GlyphResolver::with_controller_type(ControllerType::PlayStation);

    let xbox_glyph = xbox_resolver.resolve(GlyphButton::South);
    let playstation_glyph = playstation_resolver.resolve(GlyphButton::South);
    assert_ne!(
        xbox_glyph.glyph_path(),
        playstation_glyph.glyph_path(),
        "GlyphResolver must resolve different glyphs per ControllerType for the same logical button, got {:?} for both",
        xbox_glyph.glyph_path()
    );

    // Cross-check a second logical button so the resolver is proven to be
    // controller-aware in general, not just for one lucky lookup.
    let xbox_glyph_2 = xbox_resolver.resolve(GlyphButton::LeftShoulder);
    let playstation_glyph_2 = playstation_resolver.resolve(GlyphButton::LeftShoulder);
    assert_ne!(
        xbox_glyph_2.glyph_path(),
        playstation_glyph_2.glyph_path(),
        "GlyphResolver must resolve different glyphs per ControllerType for the same logical button, got {:?} for both",
        xbox_glyph_2.glyph_path()
    );
}

fn main() {
    let actions_dispatched = run_navigation_proof();
    println!("nav actions ok: {actions_dispatched}");

    run_glyph_proof();
    println!("glyphs resolved: per-controller");

    println!("gamepad_demo: navigation and glyph resolution proven with no device and no window");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_model_clamps_at_grid_edges() {
        let mut cursor = CursorModel::new(2, 2);
        cursor.apply(&MappedAction::Navigate(Direction::Up));
        cursor.apply(&MappedAction::Navigate(Direction::Left));
        assert_eq!((cursor.row, cursor.col), (0, 0));

        cursor.apply(&MappedAction::Navigate(Direction::Down));
        cursor.apply(&MappedAction::Navigate(Direction::Down));
        cursor.apply(&MappedAction::Navigate(Direction::Right));
        cursor.apply(&MappedAction::Navigate(Direction::Right));
        assert_eq!((cursor.row, cursor.col), (1, 1));
    }

    #[test]
    fn cursor_model_tracks_select_save_and_browser() {
        let mut cursor = CursorModel::new(2, 2);
        cursor.apply(&MappedAction::Select);
        cursor.apply(&MappedAction::SaveSession);
        cursor.apply(&MappedAction::OpenBrowser);
        assert_eq!(cursor.last_selected, Some((0, 0)));
        assert!(cursor.session_saved);
        assert!(cursor.browser_open);
    }

    #[test]
    fn scripted_gamepad_input_yields_batch_once_then_empty() {
        let mut input = ScriptedGamepadInput::new(vec![RawAction::Connected {
            gamepad: GamepadId(0),
        }]);
        assert_eq!(input.poll().len(), 1);
        assert!(input.poll().is_empty());
    }

    #[test]
    fn glyph_resolver_differs_by_controller_type() {
        let xbox = GlyphResolver::with_controller_type(ControllerType::Xbox);
        let playstation = GlyphResolver::with_controller_type(ControllerType::PlayStation);
        assert_ne!(
            xbox.resolve(GlyphButton::South).glyph_path(),
            playstation.resolve(GlyphButton::South).glyph_path()
        );
    }

    #[test]
    fn full_scripted_run_reports_expected_action_count() {
        let dispatched = run_navigation_proof();
        assert_eq!(dispatched, 14);
    }
}
