// path: src/bin/gamepad_demo.rs
//! gamepad_demo — headless prover for GamepadNavigator + GlyphResolver.
//!
//! No device is opened and no window is created.  A scripted, deterministic
//! sequence of raw `GamepadEvent`s is fed through `GamepadNavigator`, which
//! translates them into `GamepadAction` values and drives the cursor/edit
//! model.  In-code assertions verify expected actions and the final cursor
//! position.  `GlyphResolver` is exercised for two different `ControllerKind`
//! values, asserting that the same logical button resolves to a *different*
//! glyph on each controller family.
//!
//! Exit 0 on success; panics with a clear message on any mismatch.

use crest_synth::shell::gamepad_action::GamepadAction;
use crest_synth::shell::gamepad_input::{
    ButtonId, ControllerId, ControllerType, GamepadEvent, StubGamepadInput,
};
use crest_synth::shell::gamepad_navigator::{CursorModel, GamepadNavigator};
use crest_synth::shell::glyph_resolver::{ControllerKind, GamepadButton, GlyphResolver};

fn main() {
    run_nav_demo();
    run_glyph_demo();
    println!("gamepad_demo: all assertions passed");
}

// ── Navigation demo ───────────────────────────────────────────────────────────────────────

fn run_nav_demo() {
    // Script: DPadRight → Navigate (col +1)
    //         DPadDown  → Navigate (row +1)
    //         South     → Select   (editing = true)
    //         East      → Back     (editing = false)
    //         RightBumper → NextPage (page → 1)
    //         LeftBumper  → PreviousPage (page → 0)
    //         RightTrigger → TweakUp
    //         LeftTrigger  → TweakDown
    //         North        → AssignMod
    //         Start        → QuickSave
    let scripted_events: Vec<GamepadEvent> = vec![
        GamepadEvent::ButtonPressed {
            controller: ControllerId(0),
            button: ButtonId::DPadRight,
        },
        GamepadEvent::ButtonPressed {
            controller: ControllerId(0),
            button: ButtonId::DPadDown,
        },
        GamepadEvent::ButtonPressed {
            controller: ControllerId(0),
            button: ButtonId::South,
        },
        GamepadEvent::ButtonPressed {
            controller: ControllerId(0),
            button: ButtonId::East,
        },
        GamepadEvent::ButtonPressed {
            controller: ControllerId(0),
            button: ButtonId::RightBumper,
        },
        GamepadEvent::ButtonPressed {
            controller: ControllerId(0),
            button: ButtonId::LeftBumper,
        },
        GamepadEvent::ButtonPressed {
            controller: ControllerId(0),
            button: ButtonId::RightTrigger,
        },
        GamepadEvent::ButtonPressed {
            controller: ControllerId(0),
            button: ButtonId::LeftTrigger,
        },
        GamepadEvent::ButtonPressed {
            controller: ControllerId(0),
            button: ButtonId::North,
        },
        GamepadEvent::ButtonPressed {
            controller: ControllerId(0),
            button: ButtonId::Start,
        },
    ];

    let expected_actions: Vec<GamepadAction> = vec![
        GamepadAction::Navigate,
        GamepadAction::Navigate,
        GamepadAction::Select,
        GamepadAction::Back,
        GamepadAction::NextPage,
        GamepadAction::PreviousPage,
        GamepadAction::TweakUp,
        GamepadAction::TweakDown,
        GamepadAction::AssignMod,
        GamepadAction::QuickSave,
    ];

    let mut stub = StubGamepadInput::new();
    stub.add_controller(ControllerId(0), ControllerType::Gamepad);
    for event in scripted_events {
        stub.inject(event);
    }

    let mut navigator = GamepadNavigator::new(stub);
    // Grid: 4 rows × 8 cols, 3 pages — plenty of room for the test gestures.
    let mut cursor = CursorModel::new(4, 8, 3);

    let actions = navigator.poll(&mut cursor);

    assert_eq!(
        actions.len(),
        expected_actions.len(),
        "action count mismatch: got {} expected {}",
        actions.len(),
        expected_actions.len()
    );

    for (i, (got, want)) in actions.iter().zip(expected_actions.iter()).enumerate() {
        assert_eq!(
            got, want,
            "action[{i}] mismatch: got {got:?}, expected {want:?}"
        );
    }

    // After the script: DPadRight moved col to 1, DPadDown moved row to 1.
    // Select→editing, East→not editing; page returned to 0 after next+prev.
    assert_eq!(
        cursor.col, 1,
        "expected col=1 after DPadRight, got {}",
        cursor.col
    );
    assert_eq!(
        cursor.row, 1,
        "expected row=1 after DPadDown, got {}",
        cursor.row
    );
    assert!(
        !cursor.editing,
        "expected editing=false after Back, got {}",
        cursor.editing
    );
    assert_eq!(
        cursor.page, 0,
        "expected page=0 after NextPage+PreviousPage, got {}",
        cursor.page
    );

    let n = actions.len();
    println!("nav actions ok: {n}");
}

// ── Glyph demo ──────────────────────────────────────────────────────────────────────────

fn run_glyph_demo() {
    // Use two clearly different controller families: Xbox (A/B/X/Y) vs
    // PlayStation (✕/○/□/△).  The South button is the clearest divergence.
    let xbox_resolver = GlyphResolver::new(ControllerKind::Xbox);
    let ps_resolver = GlyphResolver::new(ControllerKind::PlayStation);

    let button = GamepadButton::South;

    let xbox_glyph = xbox_resolver.resolve(button);
    let ps_glyph = ps_resolver.resolve(button);

    assert_ne!(
        xbox_glyph, ps_glyph,
        "expected Xbox and PlayStation to resolve {button:?} to different glyphs, \
         both returned {:?}",
        xbox_glyph
    );

    // Also verify the North button differs (Y vs △).
    let north = GamepadButton::North;
    let xbox_north = xbox_resolver.resolve(north);
    let ps_north = ps_resolver.resolve(north);
    assert_ne!(
        xbox_north, ps_north,
        "expected Xbox and PlayStation to resolve {north:?} to different glyphs, \
         both returned {:?}",
        xbox_north
    );

    println!("glyphs resolved: per-controller");
    println!(
        "  Xbox South={:?}  PlayStation South={:?}",
        xbox_glyph.0, ps_glyph.0
    );
    println!(
        "  Xbox North={:?}  PlayStation North={:?}",
        xbox_north.0, ps_north.0
    );
}
