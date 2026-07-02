// path: src/shell/gui_renderer.rs

//! `GuiRenderer` is the port through which the UI/MIDI-side application
//! core drives whatever concrete windowing/graphics adapter is plugged in
//! (egui, a Steam Deck touch UI, a headless test double, etc.).
//!
//! This module lives entirely on the non-real-time side of the codebase.
//! It never touches the audio thread: a `ViewState` is a plain, owned
//! snapshot built from data that already crossed the RT boundary via the
//! `ParameterBridge` / `EventRing`, so rendering it never allocates on,
//! locks against, or blocks the audio callback.
//!
//! Every view exposes an explicit `focus` cursor so every action reachable
//! by mouse/touch is also reachable by gamepad d-pad/stick navigation and
//! a single "activate" button.

/// A zero-based cursor position within a list of focusable UI elements.
///
/// Newtype so a raw `usize` index can never be silently mixed up with a
/// patch slot, preset slot, or matrix row/column index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct FocusIndex(usize);

impl FocusIndex {
    pub fn new(index: usize) -> Self {
        Self(index)
    }

    pub fn value(self) -> usize {
        self.0
    }

    /// Clamped move used by gamepad d-pad/stick navigation; never panics
    /// and never wraps past the bounds of `len`.
    pub fn moved_by(self, delta: isize, len: usize) -> Self {
        if len == 0 {
            return Self(0);
        }
        let max = len - 1;
        let current = self.0.min(max) as isize;
        let next = (current + delta).clamp(0, max as isize);
        Self(next as usize)
    }
}

/// Identifies which top-level surface of the shell is currently on screen.
///
/// Exactly one of these is active at a time; switching between them is a
/// gamepad-reachable action (e.g. a shoulder-button tab cycle) modeled by
/// [`GuiRenderer::show_surface`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceKind {
    PatchEditor,
    MixerView,
    PresetBrowser,
    ModMatrixEditor,
}

/// One editable parameter row in the patch editor, already resolved from
/// engine/patch state into display-ready values.
#[derive(Debug, Clone, PartialEq)]
pub struct PatchParamRow {
    pub label: String,
    pub display_value: String,
}

/// Snapshot of the patch editor surface.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PatchEditorState {
    pub patch_name: String,
    pub params: Vec<PatchParamRow>,
    pub focus: FocusIndex,
}

/// One channel strip's display-ready state within the mixer view.
#[derive(Debug, Clone, PartialEq)]
pub struct ChannelStripRow {
    pub label: String,
    pub level_db: f32,
    pub pan: f32,
    pub muted: bool,
    pub soloed: bool,
}

/// Snapshot of the mixer view surface: channel strips plus the aux/master
/// bus meters that terminate the canonical signal path.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MixerViewState {
    pub channel_strips: Vec<ChannelStripRow>,
    pub aux_bus_labels: Vec<String>,
    pub master_level_db: f32,
    pub focus: FocusIndex,
}

/// One entry in the preset browser list.
#[derive(Debug, Clone, PartialEq)]
pub struct PresetEntry {
    pub name: String,
    pub bank_label: String,
}

/// Snapshot of the preset browser surface.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PresetBrowserState {
    pub entries: Vec<PresetEntry>,
    pub focus: FocusIndex,
}

/// One routing cell in the modulation matrix: a source-to-destination
/// depth, already clamped/validated upstream by the modulation context.
#[derive(Debug, Clone, PartialEq)]
pub struct ModMatrixCell {
    pub source_label: String,
    pub destination_label: String,
    pub depth: f32,
}

/// Snapshot of the mod matrix editor surface.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ModMatrixEditorState {
    pub cells: Vec<ModMatrixCell>,
    pub row_focus: FocusIndex,
    pub column_focus: FocusIndex,
}

/// Everything the shell needs to paint a single frame.
///
/// `active_surface` names which of the four snapshots is on screen; the
/// renderer is free to keep the other snapshots around for cheap surface
/// switching, but only `active_surface` is guaranteed fresh.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewState {
    pub active_surface: SurfaceKind,
    pub patch_editor: PatchEditorState,
    pub mixer_view: MixerViewState,
    pub preset_browser: PresetBrowserState,
    pub mod_matrix_editor: ModMatrixEditorState,
}

impl ViewState {
    pub fn new(active_surface: SurfaceKind) -> Self {
        Self {
            active_surface,
            patch_editor: PatchEditorState::default(),
            mixer_view: MixerViewState::default(),
            preset_browser: PresetBrowserState::default(),
            mod_matrix_editor: ModMatrixEditorState::default(),
        }
    }
}

/// The `Shell.GuiRenderer` port: `render(view) -> ()`.
///
/// Concrete adapters (egui, a headless test double, a future native Steam
/// Deck UI) implement this trait. The application core depends only on
/// this narrow interface (ISP) — it never names a concrete windowing
/// toolkit, so core logic stays testable without one (DIP).
pub trait GuiRenderer {
    /// Paint the given snapshot. Implementations must not block on I/O
    /// indefinitely (a frame must complete so the UI stays gamepad-
    /// responsive) and must treat `view` as a fully-formed, read-only
    /// snapshot — no reaching back across the RT boundary from here.
    fn render(&mut self, view: ViewState);
}

/// Extension point for surface switching. Kept as a separate, narrow
/// trait (ISP) rather than folded into [`GuiRenderer`]: adapters that only
/// need to paint a fixed surface (e.g. a kiosk-mode renderer) can depend
/// on `GuiRenderer` alone and ignore this one.
pub trait SurfaceSwitcher {
    /// Move to the next surface in a fixed gamepad-reachable cycle:
    /// PatchEditor -> MixerView -> PresetBrowser -> ModMatrixEditor -> ...
    fn show_surface(&mut self, surface: SurfaceKind);
}

/// Cycles a [`SurfaceKind`] to the next surface in the fixed tab order.
/// Free function (not a method with hidden state) so any adapter can
/// reuse the same cycle order without duplicating it.
pub fn next_surface(current: SurfaceKind) -> SurfaceKind {
    match current {
        SurfaceKind::PatchEditor => SurfaceKind::MixerView,
        SurfaceKind::MixerView => SurfaceKind::PresetBrowser,
        SurfaceKind::PresetBrowser => SurfaceKind::ModMatrixEditor,
        SurfaceKind::ModMatrixEditor => SurfaceKind::PatchEditor,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal in-memory `GuiRenderer` test double: records the last
    /// view it was asked to paint so tests can assert on dispatch without
    /// standing up any real graphics backend.
    struct RecordingRenderer {
        last_rendered: Option<ViewState>,
        render_count: usize,
    }

    impl RecordingRenderer {
        fn new() -> Self {
            Self {
                last_rendered: None,
                render_count: 0,
            }
        }
    }

    impl GuiRenderer for RecordingRenderer {
        fn render(&mut self, view: ViewState) {
            self.last_rendered = Some(view);
            self.render_count += 1;
        }
    }

    impl SurfaceSwitcher for RecordingRenderer {
        fn show_surface(&mut self, surface: SurfaceKind) {
            if let Some(view) = self.last_rendered.as_mut() {
                view.active_surface = surface;
            }
        }
    }

    fn sample_view() -> ViewState {
        let mut view = ViewState::new(SurfaceKind::PatchEditor);
        view.patch_editor.patch_name = "Init Patch".to_string();
        view.patch_editor.params.push(PatchParamRow {
            label: "Cutoff".to_string(),
            display_value: "1.2 kHz".to_string(),
        });
        view
    }

    #[test]
    fn render_records_the_exact_view_it_was_given() {
        let mut renderer = RecordingRenderer::new();
        let view = sample_view();

        renderer.render(view.clone());

        assert_eq!(renderer.last_rendered, Some(view));
        assert_eq!(renderer.render_count, 1);
    }

    #[test]
    fn render_can_be_called_multiple_times_and_counts_each_frame() {
        let mut renderer = RecordingRenderer::new();

        renderer.render(sample_view());
        renderer.render(sample_view());
        renderer.render(sample_view());

        assert_eq!(renderer.render_count, 3);
    }

    #[test]
    fn focus_index_moves_within_bounds_and_never_wraps() {
        let focus = FocusIndex::new(0);

        let moved_backward_from_zero = focus.moved_by(-1, 5);
        assert_eq!(moved_backward_from_zero.value(), 0);

        let moved_forward = focus.moved_by(2, 5);
        assert_eq!(moved_forward.value(), 2);

        let clamped_at_end = FocusIndex::new(4).moved_by(10, 5);
        assert_eq!(clamped_at_end.value(), 4);
    }

    #[test]
    fn focus_index_on_empty_list_stays_at_zero() {
        let focus = FocusIndex::new(0);

        let moved = focus.moved_by(3, 0);

        assert_eq!(moved.value(), 0);
    }

    #[test]
    fn next_surface_cycles_through_all_four_surfaces_and_back() {
        let start = SurfaceKind::PatchEditor;

        let after_one = next_surface(start);
        let after_two = next_surface(after_one);
        let after_three = next_surface(after_two);
        let after_four = next_surface(after_three);

        assert_eq!(after_one, SurfaceKind::MixerView);
        assert_eq!(after_two, SurfaceKind::PresetBrowser);
        assert_eq!(after_three, SurfaceKind::ModMatrixEditor);
        assert_eq!(after_four, SurfaceKind::PatchEditor);
    }

    #[test]
    fn surface_switcher_updates_the_active_surface_of_the_last_rendered_view() {
        let mut renderer = RecordingRenderer::new();
        renderer.render(sample_view());

        renderer.show_surface(SurfaceKind::MixerView);

        assert_eq!(
            renderer.last_rendered.as_ref().map(|v| v.active_surface),
            Some(SurfaceKind::MixerView)
        );
    }
}
