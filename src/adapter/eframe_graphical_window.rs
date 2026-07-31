use crate::control::{GraphicalShellProjection, SemanticAction};
use crate::real_time::AudioObservationSnapshot;
use crate::shell::app_window::{
    AppInputCallback, AppWindow, AudioObservationCallback, FrameObservationCallback,
    ProjectionCallback, TickCallback, WindowError,
};
use crate::shell::keyboard_input_translator::KeyboardInputTranslator;
use crate::shell::window_input::{WindowInput, WindowKey};
use crate::shell::{
    ShellFrameObservation, ShellFrameObservationError, ShellRegionId, ShellRegionObservation,
    ShellRegionRect,
};
use eframe::egui;
use egui_extras::{Size, StripBuilder};
use std::time::{Duration, Instant};

const AUTHORED_WIDTH: f32 = 1_920.0;
const AUTHORED_HEIGHT: f32 = 1_080.0;
const MINIMUM_WIDTH: f32 = 1_280.0;
const MINIMUM_HEIGHT: f32 = 800.0;
const CONTEXT_LINE_HEIGHT: f32 = 48.0;
const IDENTITY_HEADER_HEIGHT: f32 = 72.0;
const FOOTER_HEIGHT: f32 = 64.0;
const AUTHORED_SIDE_WIDTH: f32 = 420.0;
const MINIMUM_SIDE_WIDTH: f32 = 320.0;
const FRAME_INTERVAL: Duration = Duration::from_millis(16);

const BACKGROUND: egui::Color32 = egui::Color32::from_rgb(16, 18, 22);
const ELEVATED: egui::Color32 = egui::Color32::from_rgb(24, 27, 32);
const PANEL: egui::Color32 = egui::Color32::from_rgb(29, 33, 39);
const TEXT: egui::Color32 = egui::Color32::from_rgb(230, 234, 239);
const MUTED_TEXT: egui::Color32 = egui::Color32::from_rgb(150, 158, 169);
const ACCENT: egui::Color32 = egui::Color32::from_rgb(110, 205, 174);
const ADJUST_ACCENT: egui::Color32 = egui::Color32::from_rgb(232, 174, 76);

/// Production eframe/egui adapter for Crest Synth's passive Phase One shell.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EframeGraphicalWindow {
    title: String,
}

impl EframeGraphicalWindow {
    /// Creates a window with the supplied native title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
        }
    }

    /// Returns the native window title.
    pub fn title(&self) -> &str {
        &self.title
    }
}

impl Default for EframeGraphicalWindow {
    fn default() -> Self {
        Self::new("crest-synth")
    }
}

impl AppWindow for EframeGraphicalWindow {
    fn run(
        &self,
        on_input: AppInputCallback,
        projection: ProjectionCallback,
        audio_observation: AudioObservationCallback,
        on_tick: TickCallback,
        on_frame: FrameObservationCallback,
    ) -> Result<(), WindowError> {
        let title = self.title.clone();
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([AUTHORED_WIDTH, AUTHORED_HEIGHT])
                .with_min_inner_size([MINIMUM_WIDTH, MINIMUM_HEIGHT]),
            ..Default::default()
        };
        eframe::run_native(
            &title,
            options,
            Box::new(move |creation_context| {
                egui_extras::install_image_loaders(&creation_context.egui_ctx);
                Ok(Box::new(
                    EframeGraphicalApplication::new_with_audio_observation(
                        on_input,
                        projection,
                        audio_observation,
                        on_tick,
                        on_frame,
                    ),
                ))
            }),
        )
        .map_err(|error| WindowError::new(format!("eframe window failed: {error}")))
    }
}

/// Production eframe application used by the native window and real-context tests.
pub struct EframeGraphicalApplication {
    on_input: AppInputCallback,
    projection: ProjectionCallback,
    audio_observation: AudioObservationCallback,
    on_tick: TickCallback,
    on_frame: FrameObservationCallback,
    translator: KeyboardInputTranslator,
    previous_tick: Instant,
    frame_observation_error: Option<ShellFrameObservationError>,
    close_requested: bool,
}

impl EframeGraphicalApplication {
    /// Creates the passive GUI adapter around application-owned callbacks.
    pub fn new(
        on_input: AppInputCallback,
        projection: ProjectionCallback,
        on_tick: TickCallback,
        on_frame: FrameObservationCallback,
    ) -> Self {
        Self::new_with_audio_observation(
            on_input,
            projection,
            Box::new(AudioObservationSnapshot::default),
            on_tick,
            on_frame,
        )
    }

    /// Creates the GUI adapter with one immutable latest-audio observation source.
    pub fn new_with_audio_observation(
        on_input: AppInputCallback,
        projection: ProjectionCallback,
        audio_observation: AudioObservationCallback,
        on_tick: TickCallback,
        on_frame: FrameObservationCallback,
    ) -> Self {
        Self {
            on_input,
            projection,
            audio_observation,
            on_tick,
            on_frame,
            translator: KeyboardInputTranslator::new(),
            previous_tick: Instant::now(),
            frame_observation_error: None,
            close_requested: false,
        }
    }

    /// Returns the first invalid production-frame observation, if one occurred.
    pub const fn frame_observation_error(&self) -> Option<ShellFrameObservationError> {
        self.frame_observation_error
    }

    fn handle_input(&mut self, context: &egui::Context) {
        context.input(|input| {
            for event in &input.events {
                if let Some(event) = translate_egui_event(&mut self.translator, event) {
                    (self.on_input)(event);
                }
            }
        });
    }

    fn tick(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.previous_tick);
        self.previous_tick = now;
        (self.on_tick)(elapsed)
    }

    fn request_close_once(&mut self, context: &egui::Context) {
        if !self.close_requested {
            self.close_requested = true;
            context.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn paint_shell(
        &mut self,
        context: &egui::Context,
        projection: &GraphicalShellProjection,
        audio_observation: AudioObservationSnapshot,
    ) -> Result<(ShellFrameObservation, Option<SemanticAction>), ShellFrameObservationError> {
        let context_panel = egui::TopBottomPanel::top("crest-context-line")
            .exact_height(CONTEXT_LINE_HEIGHT)
            .frame(shell_frame(ELEVATED))
            .show(context, |ui| paint_context_line(ui, projection));

        let identity_panel = egui::TopBottomPanel::top("crest-identity-header")
            .exact_height(IDENTITY_HEADER_HEIGHT)
            .frame(shell_frame(PANEL))
            .show(context, |ui| paint_identity_header(ui, projection));

        let mut passive_action = None;
        let footer_panel = egui::TopBottomPanel::bottom("crest-footer")
            .exact_height(FOOTER_HEIGHT)
            .frame(shell_frame(ELEVATED))
            .show(context, |ui| passive_action = paint_footer(ui, projection));

        let viewport = context.input(egui::InputState::screen_rect);
        let side_width = desired_side_width(viewport.width());
        let side_panel = egui::SidePanel::right("crest-persistent-side-region")
            .exact_width(side_width)
            .frame(shell_frame(PANEL))
            .show(context, |ui| paint_side_region(ui, projection));

        let main_panel = egui::CentralPanel::default()
            .frame(shell_frame(BACKGROUND))
            .show(context, |ui| {
                paint_main_workspace(ui, projection, audio_observation)
            });

        let relative = |rect: egui::Rect| relative_rect(rect.intersect(viewport), viewport.min);
        let observation = ShellFrameObservation::try_new_semantic(
            viewport.width(),
            viewport.height(),
            projection.semantic_model(),
            [
                ShellRegionObservation::new(
                    ShellRegionId::ContextLine,
                    relative(context_panel.response.rect),
                    projection.context_line().context_label(),
                ),
                ShellRegionObservation::new(
                    ShellRegionId::IdentityHeader,
                    relative(identity_panel.response.rect),
                    projection.identity_header().primary_label(),
                ),
                ShellRegionObservation::new(
                    ShellRegionId::MainWorkspace,
                    relative(main_panel.response.rect),
                    projection.workspace().main_label(),
                ),
                ShellRegionObservation::new(
                    ShellRegionId::PersistentSideRegion,
                    relative(side_panel.response.rect),
                    projection.workspace().side_label(),
                ),
                ShellRegionObservation::new(
                    ShellRegionId::Footer,
                    relative(footer_panel.response.rect),
                    projection.footer().path_label(),
                ),
            ],
        )?;
        Ok((observation, passive_action))
    }
}

impl eframe::App for EframeGraphicalApplication {
    fn update(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        if self.close_requested {
            return;
        }
        self.handle_input(context);
        if !self.tick() {
            self.request_close_once(context);
            return;
        }

        let projection = (self.projection)();
        let audio_observation = (self.audio_observation)();
        match self.paint_shell(context, &projection, audio_observation) {
            Ok((observation, passive_action)) => {
                (self.on_frame)(observation);
                if let Some(action) = passive_action {
                    (self.on_input)(action);
                }
            }
            Err(error) => {
                self.frame_observation_error.get_or_insert(error);
                self.request_close_once(context);
                return;
            }
        }
        context.request_repaint_after(FRAME_INTERVAL);
    }
}

fn shell_frame(fill: egui::Color32) -> egui::Frame {
    egui::Frame::new()
        .fill(fill)
        .inner_margin(egui::Margin::ZERO)
        .outer_margin(egui::Margin::ZERO)
}

fn paint_context_line(ui: &mut egui::Ui, projection: &GraphicalShellProjection) {
    let mode_color = match projection.semantic_model().interaction_mode() {
        crate::control::InteractionMode::Navigate => ACCENT,
        crate::control::InteractionMode::Adjust => ADJUST_ACCENT,
        crate::control::InteractionMode::Modal | crate::control::InteractionMode::MultiSelect => {
            MUTED_TEXT
        }
    };
    StripBuilder::new(ui)
        .size(Size::relative(0.34))
        .size(Size::relative(0.32))
        .size(Size::remainder())
        .horizontal(|mut strip| {
            strip.cell(|ui| {
                padded_label(ui, projection.context_line().product_label(), TEXT, true);
            });
            strip.cell(|ui| {
                ui.centered_and_justified(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(projection.context_line().context_label())
                                .color(mode_color)
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "· {}",
                                projection.semantic_model().interaction_mode().label()
                            ))
                            .color(mode_color)
                            .strong(),
                        );
                    });
                });
            });
            strip.cell(|ui| {
                trailing_label(
                    ui,
                    projection.context_line().status_label(),
                    MUTED_TEXT,
                    false,
                );
            });
        });
}

fn paint_identity_header(ui: &mut egui::Ui, projection: &GraphicalShellProjection) {
    StripBuilder::new(ui)
        .size(Size::exact(39.0))
        .size(Size::remainder())
        .vertical(|mut strip| {
            strip.cell(|ui| {
                padded_label(ui, projection.identity_header().primary_label(), TEXT, true);
            });
            strip.cell(|ui| {
                padded_label(
                    ui,
                    projection.identity_header().secondary_label(),
                    MUTED_TEXT,
                    false,
                );
            });
        });
}

fn paint_main_workspace(
    ui: &mut egui::Ui,
    projection: &GraphicalShellProjection,
    audio_observation: AudioObservationSnapshot,
) {
    StripBuilder::new(ui)
        .size(Size::exact(42.0))
        .size(Size::remainder())
        .vertical(|mut strip| {
            strip.cell(|ui| {
                padded_label(ui, projection.workspace().main_label(), ACCENT, true);
            });
            strip.cell(|ui| {
                let semantic = projection.semantic_model();
                if semantic.context() == crate::control::TopLevelContext::Mixer {
                    paint_mixer_workspace(ui, projection, audio_observation);
                } else {
                    paint_patch_workspace(ui, projection);
                }
            });
        });
}

fn paint_patch_workspace(ui: &mut egui::Ui, projection: &GraphicalShellProjection) {
    egui::ScrollArea::vertical()
        .animated(false)
        .id_salt("crest-synth-patch-parameters")
        .show(ui, |ui| {
            ui.add_space(12.0);
            let semantic = projection.semantic_model();
            if let Some(surface) = semantic.surface(crate::control::SurfaceId::PatchMain) {
                for control in surface
                    .controls()
                    .iter()
                    .filter(|control| control.visible())
                {
                    paint_semantic_control(ui, control, semantic.interaction_mode());
                }
            }
            paint_diagnostic(ui, projection);
        });
}

fn paint_mixer_workspace(
    ui: &mut egui::Ui,
    projection: &GraphicalShellProjection,
    audio_observation: AudioObservationSnapshot,
) {
    let semantic = projection.semantic_model();
    let observation_matches = audio_observation.parameter_generation() == semantic.generation()
        && audio_observation.active_graph_revision() == semantic.status().graph_revision();
    if let Some(surface) = semantic.surface(crate::control::SurfaceId::MixerMain) {
        egui::ScrollArea::vertical()
            .animated(false)
            .id_salt("crest-synth-mixer-workspace")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for track_id in crate::mixer::mixer_track_id::MixerTrackId::ALL {
                        ui.label(
                            egui::RichText::new(track_id.to_string())
                                .monospace()
                                .small()
                                .color(ACCENT),
                        );
                    }
                });
                ui.separator();
                egui::ScrollArea::horizontal()
                    .animated(false)
                    .id_salt("crest-synth-mixer-tracks")
                    .show(ui, |ui| {
                        ui.horizontal_top(|ui| {
                            for track_id in crate::mixer::mixer_track_id::MixerTrackId::ALL {
                                ui.push_id(track_id.index(), |ui| {
                                    egui::Frame::new()
                                        .fill(PANEL)
                                        .stroke(egui::Stroke::new(1.0_f32, ELEVATED))
                                        .inner_margin(egui::Margin::same(8))
                                        .show(ui, |ui| {
                                            ui.set_min_width(176.0);
                                            ui.label(
                                                egui::RichText::new(track_id.to_string())
                                                    .color(ACCENT)
                                                    .strong(),
                                            );
                                            let meter = if observation_matches {
                                                audio_observation.track(track_id)
                                            } else {
                                                crate::mixer::track_meter::TrackMeter::ZERO
                                            };
                                            ui.add(
                                                egui::ProgressBar::new(meter.rms().clamp(0.0, 1.0))
                                                    .text(format!("METER {:.3}", meter.rms()))
                                                    .animate(false),
                                            );
                                            for control in
                                                surface.controls().iter().filter(|control| {
                                                    control.visible()
                                                        && control
                                                            .path()
                                                            .control_id()
                                                            .as_mixer_track_id()
                                                            == Some(track_id)
                                                })
                                            {
                                                paint_semantic_control(
                                                    ui,
                                                    control,
                                                    semantic.interaction_mode(),
                                                );
                                            }
                                        });
                                });
                            }
                        });
                    });
                paint_diagnostic(ui, projection);
            });
    } else {
        paint_diagnostic(ui, projection);
    }
}

fn paint_diagnostic(ui: &mut egui::Ui, projection: &GraphicalShellProjection) {
    ui.add_space(12.0);
    ui.separator();
    egui::CollapsingHeader::new("DIAGNOSTIC")
        .default_open(true)
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(projection.workspace().diagnostic().body())
                    .monospace()
                    .color(MUTED_TEXT),
            );
        });
}

fn paint_side_region(ui: &mut egui::Ui, projection: &GraphicalShellProjection) {
    StripBuilder::new(ui)
        .size(Size::exact(42.0))
        .size(Size::remainder())
        .vertical(|mut strip| {
            strip.cell(|ui| {
                padded_label(ui, projection.workspace().side_label(), TEXT, true);
            });
            strip.cell(|ui| {
                egui::ScrollArea::vertical()
                    .animated(false)
                    .id_salt("crest-synth-side-controls")
                    .show(ui, |ui| {
                        ui.add_space(12.0);
                        ui.horizontal(|ui| {
                            ui.add_space(16.0);
                            ui.vertical(|ui| {
                                let semantic = projection.semantic_model();
                                let side_id =
                                    crate::control::SurfaceId::side_for(semantic.context());
                                if let Some(surface) = semantic.surface(side_id) {
                                    paint_surface_summary(ui, surface.summary());
                                    for control in surface.controls() {
                                        paint_semantic_control(
                                            ui,
                                            control,
                                            semantic.interaction_mode(),
                                        );
                                    }
                                }
                                if semantic.errors().is_empty() {
                                    ui.label(egui::RichText::new("NO ERRORS").color(ACCENT));
                                } else {
                                    for error in semantic.errors() {
                                        ui.label(
                                            egui::RichText::new(error.label()).color(ADJUST_ACCENT),
                                        );
                                    }
                                }
                            });
                        });
                    });
            });
        });
}

fn paint_surface_summary(ui: &mut egui::Ui, summary: &crate::control::SemanticSurfaceSummary) {
    match summary {
        crate::control::SemanticSurfaceSummary::MixerInspector {
            focused_track,
            routed_patches,
            ..
        } => {
            ui.label(
                egui::RichText::new(format!("SELECTED {focused_track}"))
                    .monospace()
                    .color(ACCENT),
            );
            if routed_patches.is_empty() {
                ui.label(egui::RichText::new("ROUTED PATCHES · EMPTY").color(MUTED_TEXT));
            } else {
                for patch in routed_patches {
                    ui.label(
                        egui::RichText::new(format!(
                            "PATCH {:02} · {}",
                            patch.patch_id().value(),
                            patch.patch_name()
                        ))
                        .color(MUTED_TEXT),
                    );
                }
            }
        }
        _ => {
            ui.label(
                egui::RichText::new(format!("{summary:?}"))
                    .monospace()
                    .color(MUTED_TEXT),
            );
        }
    }
}

fn paint_footer(
    ui: &mut egui::Ui,
    projection: &GraphicalShellProjection,
) -> Option<SemanticAction> {
    let mut emitted = None;
    StripBuilder::new(ui)
        .size(Size::relative(0.38))
        .size(Size::remainder())
        .horizontal(|mut strip| {
            strip.cell(|ui| {
                padded_label(ui, projection.footer().path_label(), TEXT, true);
            });
            strip.cell(|ui| {
                egui::ScrollArea::horizontal()
                    .id_salt("crest-valid-actions")
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for valid in projection.semantic_model().valid_actions() {
                                let label = valid.hint().map_or_else(
                                    || valid.label().to_owned(),
                                    |hint| format!("{hint} · {}", valid.label()),
                                );
                                if ui.small_button(label).clicked() && emitted.is_none() {
                                    emitted = Some(valid.action().clone());
                                }
                            }
                        });
                    });
            });
        });
    emitted
}

fn paint_semantic_control(
    ui: &mut egui::Ui,
    control: &crate::control::SemanticControlViewModel,
    mode: crate::control::InteractionMode,
) {
    let focus_color = if mode == crate::control::InteractionMode::Adjust {
        ADJUST_ACCENT
    } else {
        ACCENT
    };
    let fill = if control.focused() {
        focus_color.gamma_multiply(0.16)
    } else {
        egui::Color32::TRANSPARENT
    };
    let stroke = if control.focused() {
        egui::Stroke::new(1.5_f32, focus_color)
    } else {
        egui::Stroke::new(1.0_f32, PANEL)
    };
    let response = egui::Frame::new()
        .fill(fill)
        .stroke(stroke)
        .inner_margin(egui::Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let label_color = if control.enabled() { TEXT } else { MUTED_TEXT };
                ui.label(egui::RichText::new(control.label()).color(label_color));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(semantic_value_label(control.value()))
                            .monospace()
                            .color(if control.focused() {
                                focus_color
                            } else {
                                MUTED_TEXT
                            }),
                    );
                    // Structural rows carry their reducer-owned lifecycle and
                    // any typed refusal as text, never colour alone; the
                    // adapter renders the projection without recomputing it.
                    if let Some(error) = control.error() {
                        ui.label(
                            egui::RichText::new(error.label())
                                .monospace()
                                .small()
                                .color(ADJUST_ACCENT),
                        );
                    }
                    if let Some(status) = control.status().filter(|status| {
                        status.kind() != crate::control::EngineSelectionStatusKind::Ready
                    }) {
                        ui.label(
                            egui::RichText::new(status.label())
                                .monospace()
                                .small()
                                .color(ADJUST_ACCENT),
                        );
                    }
                });
            });
        });
    if control.focused() && !ui.clip_rect().contains(response.response.rect.center()) {
        response.response.scroll_to_me(Some(egui::Align::Center));
    }
    ui.add_space(4.0);
}

fn semantic_value_label(value: &crate::control::SemanticControlValue) -> String {
    match value {
        crate::control::SemanticControlValue::Scalar(value) => format!("{value:.3}"),
        crate::control::SemanticControlValue::Parameter(value) => match value {
            crate::synth::ParameterValue::Continuous(value) => format!("{value:.3}"),
            crate::synth::ParameterValue::Stepped(value) => value.to_string(),
            crate::synth::ParameterValue::Choice(value) => value.clone(),
            crate::synth::ParameterValue::Toggle(value) => {
                if *value { "ON" } else { "OFF" }.to_owned()
            }
        },
        crate::control::SemanticControlValue::Asset(reference) => reference.locator().to_owned(),
        crate::control::SemanticControlValue::Identity(value)
        | crate::control::SemanticControlValue::Summary(value) => value.clone(),
    }
}

fn padded_label(ui: &mut egui::Ui, label: &str, color: egui::Color32, strong: bool) {
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        let text = egui::RichText::new(label).color(color);
        ui.label(if strong { text.strong() } else { text });
    });
}

fn trailing_label(ui: &mut egui::Ui, label: &str, color: egui::Color32, strong: bool) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.add_space(16.0);
        let text = egui::RichText::new(label).color(color);
        ui.label(if strong { text.strong() } else { text });
    });
}

fn desired_side_width(viewport_width: f32) -> f32 {
    (viewport_width * AUTHORED_SIDE_WIDTH / AUTHORED_WIDTH).max(MINIMUM_SIDE_WIDTH)
}

fn relative_rect(rect: egui::Rect, origin: egui::Pos2) -> ShellRegionRect {
    ShellRegionRect::new(
        rect.min.x - origin.x,
        rect.min.y - origin.y,
        rect.max.x - origin.x,
        rect.max.y - origin.y,
    )
}

fn translate_egui_event(
    translator: &mut KeyboardInputTranslator,
    event: &egui::Event,
) -> Option<SemanticAction> {
    normalize_egui_event(event).and_then(|input| translator.translate(input))
}

fn normalize_egui_event(event: &egui::Event) -> Option<WindowInput> {
    match event {
        egui::Event::Key { key, pressed, .. } => {
            let key = normalize_key(*key);
            Some(if *pressed {
                WindowInput::key_down(key)
            } else {
                WindowInput::key_up(key)
            })
        }
        egui::Event::WindowFocused(false) => Some(WindowInput::focus_lost()),
        _ => None,
    }
}

fn normalize_key(key: egui::Key) -> WindowKey {
    match key {
        egui::Key::Num1 => WindowKey::Digit1,
        egui::Key::Num2 => WindowKey::Digit2,
        egui::Key::W => WindowKey::W,
        egui::Key::S => WindowKey::S,
        egui::Key::A => WindowKey::A,
        egui::Key::D => WindowKey::D,
        egui::Key::K => WindowKey::K,
        _ => WindowKey::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        desired_side_width, normalize_egui_event, translate_egui_event, EframeGraphicalWindow,
    };
    use crate::control::app_event::Direction;
    use crate::control::{InteractionMode, SemanticAction};
    use crate::shell::keyboard_input_translator::KeyboardInputTranslator;
    use crate::shell::window_input::{WindowInput, WindowKey};
    use eframe::egui::{self, Key, Modifiers};

    const DIRECTION_CASES: [(Key, WindowKey, Direction); 4] = [
        (Key::W, WindowKey::W, Direction::Up),
        (Key::S, WindowKey::S, Direction::Down),
        (Key::A, WindowKey::A, Direction::Left),
        (Key::D, WindowKey::D, Direction::Right),
    ];

    fn key_event(key: Key, pressed: bool) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: Modifiers::default(),
        }
    }

    #[test]
    fn graphical_window_default_uses_the_product_name() {
        assert_eq!(EframeGraphicalWindow::default().title(), "crest-synth");
    }

    #[test]
    fn graphical_window_uses_the_two_reference_side_widths() {
        assert_eq!(desired_side_width(1_920.0), 420.0);
        assert_eq!(desired_side_width(1_280.0), 320.0);
    }

    #[test]
    fn graphical_window_normalizes_the_complete_key_vocabulary() {
        let cases = [
            (Key::Num1, WindowKey::Digit1),
            (Key::Num2, WindowKey::Digit2),
            (Key::W, WindowKey::W),
            (Key::S, WindowKey::S),
            (Key::A, WindowKey::A),
            (Key::D, WindowKey::D),
            (Key::K, WindowKey::K),
            (Key::Enter, WindowKey::Other),
        ];

        for (egui_key, window_key) in cases {
            assert_eq!(
                normalize_egui_event(&key_event(egui_key, true)),
                Some(WindowInput::key_down(window_key))
            );
            assert_eq!(
                normalize_egui_event(&key_event(egui_key, false)),
                Some(WindowInput::key_up(window_key))
            );
        }
    }

    #[test]
    fn graphical_window_delegates_direct_context_keys() {
        let mut translator = KeyboardInputTranslator::new();
        assert_eq!(
            translate_egui_event(&mut translator, &key_event(Key::Num1, true)),
            Some(SemanticAction::SelectContext(
                crate::control::TopLevelContext::Mixer
            ))
        );
        assert_eq!(
            translate_egui_event(&mut translator, &key_event(Key::Num2, true)),
            Some(SemanticAction::SelectContext(
                crate::control::TopLevelContext::Patch
            ))
        );
        assert_eq!(
            translate_egui_event(&mut translator, &key_event(Key::Num1, false)),
            None
        );
        assert_eq!(
            translate_egui_event(&mut translator, &key_event(Key::Num2, false)),
            None
        );
    }

    #[test]
    fn graphical_window_normalizes_focus_loss_and_ignores_other_events() {
        assert_eq!(
            normalize_egui_event(&egui::Event::WindowFocused(false)),
            Some(WindowInput::focus_lost())
        );
        assert_eq!(
            normalize_egui_event(&egui::Event::WindowFocused(true)),
            None
        );
        assert_eq!(normalize_egui_event(&egui::Event::Copy), None);
    }

    #[test]
    fn graphical_window_delegates_every_direction_to_the_shared_translator() {
        let mut translator = KeyboardInputTranslator::new();

        for (egui_key, _, direction) in DIRECTION_CASES {
            assert_eq!(
                translate_egui_event(&mut translator, &key_event(egui_key, true)),
                Some(SemanticAction::Navigate(direction))
            );
            assert_eq!(
                translate_egui_event(&mut translator, &key_event(egui_key, false)),
                None
            );
        }

        assert_eq!(
            translate_egui_event(&mut translator, &key_event(Key::K, true)),
            Some(SemanticAction::SetInteractionMode(InteractionMode::Adjust))
        );
        for (egui_key, _, direction) in DIRECTION_CASES {
            assert_eq!(
                translate_egui_event(&mut translator, &key_event(egui_key, true)),
                Some(SemanticAction::Adjust(direction))
            );
        }
        assert_eq!(
            translate_egui_event(&mut translator, &key_event(Key::Enter, true)),
            None
        );
    }

    #[test]
    fn graphical_window_delegates_release_and_focus_reset_without_private_state() {
        let mut translator = KeyboardInputTranslator::new();

        assert_eq!(
            translate_egui_event(&mut translator, &key_event(Key::K, true)),
            Some(SemanticAction::SetInteractionMode(InteractionMode::Adjust))
        );
        assert_eq!(
            translate_egui_event(&mut translator, &egui::Event::WindowFocused(false)),
            Some(SemanticAction::SetInteractionMode(
                InteractionMode::Navigate
            ))
        );
        assert_eq!(
            translate_egui_event(&mut translator, &key_event(Key::S, true)),
            Some(SemanticAction::Navigate(Direction::Down))
        );

        assert_eq!(
            translate_egui_event(&mut translator, &key_event(Key::K, true)),
            Some(SemanticAction::SetInteractionMode(InteractionMode::Adjust))
        );
        assert_eq!(
            translate_egui_event(&mut translator, &key_event(Key::K, false)),
            Some(SemanticAction::SetInteractionMode(
                InteractionMode::Navigate
            ))
        );
        assert_eq!(
            translate_egui_event(&mut translator, &key_event(Key::A, true)),
            Some(SemanticAction::Navigate(Direction::Left))
        );
    }
}
