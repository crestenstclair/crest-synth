// path: src/shell/egui_renderer.rs

//! `EguiRenderer` is the concrete `egui`-backed implementation of the
//! `port.Shell.GuiRenderer` port. It lives entirely on the non-real-time
//! (UI/MIDI) side of the codebase: it only ever consumes a fully-formed,
//! owned `ViewState` snapshot that has already crossed the RT boundary via
//! the `ParameterBridge` / `EventRing` elsewhere in the application. It
//! never reaches back across that boundary, never blocks on I/O, and is
//! never invoked from the audio callback.
//!
//! Layout is intentionally simple: one small render function per
//! `SurfaceKind`, each of which paints its rows with the currently
//! focused row visually distinguished so every action already reachable
//! via gamepad d-pad/stick navigation (tracked upstream by `FocusIndex`)
//! is also visible on screen.

use crate::shell::gui_renderer::{
    ChannelStripRow, GuiRenderer, MixerViewState, ModMatrixCell, ModMatrixEditorState,
    PatchEditorState, PatchParamRow, PresetBrowserState, PresetEntry, SurfaceKind,
    SurfaceSwitcher, ViewState,
};

/// Renders a [`ViewState`] snapshot through an owned `egui::Context`.
///
/// The `egui::Context` is accepted via the constructor rather than created
/// internally (dependency injection): callers that already own a context
/// (e.g. one wired up by a windowing/backend crate such as `eframe`) pass
/// it in, and tests can pass a fresh, headless `egui::Context::default()`
/// without needing any real window or GPU surface.
pub struct EguiRenderer {
    ctx: egui::Context,
    surface_override: Option<SurfaceKind>,
}

impl EguiRenderer {
    /// Full constructor: accepts the `egui::Context` this renderer paints
    /// into.
    pub fn new(ctx: egui::Context) -> Self {
        Self {
            ctx,
            surface_override: None,
        }
    }

    /// Convenience constructor for callers (and tests) that don't already
    /// own an `egui::Context` and are happy with a fresh, default one.
    pub fn with_default_context() -> Self {
        Self::new(egui::Context::default())
    }
}

impl Default for EguiRenderer {
    fn default() -> Self {
        Self::with_default_context()
    }
}

impl GuiRenderer for EguiRenderer {
    fn render(&mut self, view: ViewState) {
        // `show_surface` (below) lets a caller pin the renderer to a fixed
        // surface regardless of what the latest snapshot says is active;
        // absent that, the snapshot's own `active_surface` wins.
        let surface = self.surface_override.unwrap_or(view.active_surface);

        // `Context::run` lays out one immediate-mode frame and returns; it
        // never blocks indefinitely, so a frame always completes and the
        // gamepad-driven UI stays responsive.
        let _ = self.ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| match surface {
                SurfaceKind::PatchEditor => render_patch_editor(ui, &view.patch_editor),
                SurfaceKind::MixerView => render_mixer_view(ui, &view.mixer_view),
                SurfaceKind::PresetBrowser => render_preset_browser(ui, &view.preset_browser),
                SurfaceKind::ModMatrixEditor => {
                    render_mod_matrix_editor(ui, &view.mod_matrix_editor)
                }
            });
        });
    }
}

/// Extension point kept separate from [`GuiRenderer`] per the port's ISP
/// intent: overrides which surface is painted on the *next* `render` call,
/// independent of whatever `active_surface` the caller's snapshot carries.
impl SurfaceSwitcher for EguiRenderer {
    fn show_surface(&mut self, surface: SurfaceKind) {
        self.surface_override = Some(surface);
    }
}

fn render_patch_editor(ui: &mut egui::Ui, state: &PatchEditorState) {
    ui.heading(&state.patch_name);
    for (index, row) in state.params.iter().enumerate() {
        render_focusable_row(ui, index == state.focus.value(), &row.label, &row.display_value);
    }
}

fn render_mixer_view(ui: &mut egui::Ui, state: &MixerViewState) {
    for (index, strip) in state.channel_strips.iter().enumerate() {
        render_focusable_row(
            ui,
            index == state.focus.value(),
            &strip.label,
            &channel_strip_summary(strip),
        );
    }

    for aux_label in &state.aux_bus_labels {
        ui.label(aux_label);
    }

    ui.label(format!("Master {:.1} dB", state.master_level_db));
}

fn channel_strip_summary(strip: &ChannelStripRow) -> String {
    format!(
        "{:.1} dB  pan {:.2}{}{}",
        strip.level_db,
        strip.pan,
        if strip.muted { "  MUTE" } else { "" },
        if strip.soloed { "  SOLO" } else { "" },
    )
}

fn render_preset_browser(ui: &mut egui::Ui, state: &PresetBrowserState) {
    for (index, entry) in state.entries.iter().enumerate() {
        render_focusable_row(ui, index == state.focus.value(), &entry.name, &entry.bank_label);
    }
}

fn render_mod_matrix_editor(ui: &mut egui::Ui, state: &ModMatrixEditorState) {
    for (index, cell) in state.cells.iter().enumerate() {
        let focused = index == state.row_focus.value();
        render_focusable_row(ui, focused, &mod_matrix_label(cell), &format!("{:.2}", cell.depth));
    }
}

fn mod_matrix_label(cell: &ModMatrixCell) -> String {
    format!("{} -> {}", cell.source_label, cell.destination_label)
}

/// Paints one label/value row, visually distinguishing the row currently
/// under gamepad focus so keyboard/gamepad and mouse/touch users see the
/// same cursor.
fn render_focusable_row(ui: &mut egui::Ui, focused: bool, label: &str, value: &str) {
    let text = format!("{}{}: {}", if focused { "> " } else { "  " }, label, value);
    if focused {
        ui.colored_label(egui::Color32::YELLOW, text);
    } else {
        ui.label(text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::gui_renderer::{FocusIndex, PatchParamRow as ParamRow};

    fn sample_view() -> ViewState {
        let mut view = ViewState::new(SurfaceKind::PatchEditor);
        view.patch_editor.patch_name = "Init Patch".to_string();
        view.patch_editor.params.push(ParamRow {
            label: "Cutoff".to_string(),
            display_value: "1.2 kHz".to_string(),
        });
        view.patch_editor.focus = FocusIndex::new(0);

        view.mixer_view.channel_strips.push(ChannelStripRow {
            label: "Ch 1".to_string(),
            level_db: -3.0,
            pan: 0.0,
            muted: false,
            soloed: false,
        });
        view.mixer_view.aux_bus_labels.push("Reverb Bus".to_string());
        view.mixer_view.master_level_db = -1.5;

        view.preset_browser.entries.push(PresetEntry {
            name: "Warm Pad".to_string(),
            bank_label: "Factory A".to_string(),
        });

        view.mod_matrix_editor.cells.push(ModMatrixCell {
            source_label: "LFO 1".to_string(),
            destination_label: "Cutoff".to_string(),
            depth: 0.5,
        });

        view
    }

    #[test]
    fn render_does_not_panic_for_every_surface() {
        let mut renderer = EguiRenderer::default();

        for surface in [
            SurfaceKind::PatchEditor,
            SurfaceKind::MixerView,
            SurfaceKind::PresetBrowser,
            SurfaceKind::ModMatrixEditor,
        ] {
            let mut view = sample_view();
            view.active_surface = surface;
            renderer.render(view);
        }
    }

    #[test]
    fn render_can_be_called_repeatedly_across_frames() {
        let mut renderer = EguiRenderer::default();

        renderer.render(sample_view());
        renderer.render(sample_view());
        renderer.render(sample_view());
    }

    #[test]
    fn show_surface_overrides_the_snapshots_active_surface() {
        let mut renderer = EguiRenderer::default();
        renderer.show_surface(SurfaceKind::MixerView);

        assert_eq!(renderer.surface_override, Some(SurfaceKind::MixerView));

        let mut view = sample_view();
        view.active_surface = SurfaceKind::PatchEditor;

        // Should render the overridden mixer surface without panicking,
        // even though the snapshot itself targets the patch editor.
        renderer.render(view);
    }

    #[test]
    fn new_accepts_an_injected_context_instead_of_creating_its_own() {
        let ctx = egui::Context::default();
        let mut renderer = EguiRenderer::new(ctx);

        renderer.render(sample_view());
    }
}
