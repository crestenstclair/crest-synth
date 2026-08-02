use crate::control::{GraphicalShellProjection, SemanticAction};
use crate::real_time::AudioObservationSnapshot;
use crate::shell::app_window::{
    AppInputCallback, AppWindow, AudioObservationCallback, FrameObservationCallback,
    ProjectionCallback, TickCallback, WindowError,
};
use crate::shell::keyboard_input_translator::KeyboardInputTranslator;
use crate::shell::visual::primitives::{focus, rules, text, value};
use crate::shell::visual::{
    AuthoredTypeface, ComponentState, SemanticColor, SpacingStep, TypeStyle, TypefaceError,
    ViewportDensityPolicy, KEYLINE_RESTING_PX, MIN_INTERACTIVE_TARGET_PX,
};
use crate::shell::window_input::{WindowInput, WindowKey};
use crate::shell::{
    ShellFrameObservation, ShellFrameObservationError, ShellRegionId, ShellRegionObservation,
    ShellRegionRect,
};
use eframe::egui;
use egui_extras::{Size, StripBuilder};
use std::time::{Duration, Instant};

/// The idle repaint cadence. A separate declared decision from the visual
/// vocabulary and not resolved through the density policy.
const FRAME_INTERVAL: Duration = Duration::from_millis(16);

/// The height of a workspace region's title row.
///
/// This and the two extents below are sub-band splits the authored vocabulary
/// does not declare. They are named rather than resolved because there is
/// nothing yet to resolve them from; the gap is recorded for the follow-on
/// mission rather than papered over with an invented policy accessor. This one
/// sits inside the workspace, which the policy sizes as the remainder, so it
/// is not squeezed by the Steam Deck bands.
const WORKSPACE_TITLE_ROW_PX: f32 = 42.0;

/// The minimum width of one mixer track column.
const MIXER_TRACK_MIN_WIDTH_PX: f32 = 176.0;

/// How the context line divides horizontally: product, context, status.
const CONTEXT_PRODUCT_FRACTION: f32 = 0.34;
const CONTEXT_MODE_FRACTION: f32 = 0.32;

/// How the footer divides horizontally: current path, then valid actions.
const FOOTER_PATH_FRACTION: f32 = 0.38;

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

        // The authored faces are read before the window exists, so an
        // unavailable face is a typed startup failure carrying the file at
        // fault rather than a window that opens and paints in a substituted
        // face. A substituted face looks plausible while being wrong, which is
        // the one failure this path must not have.
        let typeface = AuthoredTypeface::load()
            .map_err(|error| WindowError::new(format!("authored typeface unavailable: {error}")))?;

        let authored = ViewportDensityPolicy::Desktop.authored_viewport();
        let smallest = ViewportDensityPolicy::SteamDeck.authored_viewport();
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([authored.width_px, authored.height_px])
                .with_min_inner_size([smallest.width_px, smallest.height_px]),
            ..Default::default()
        };
        eframe::run_native(
            &title,
            options,
            Box::new(move |creation_context| {
                egui_extras::install_image_loaders(&creation_context.egui_ctx);
                // Once, during eframe setup, before the first painted frame.
                install_typeface(&creation_context.egui_ctx, &typeface);
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
        install_authored_chrome(context);
        // The one place this adapter reads the raw viewport to choose a
        // layout. Every band and split below comes from the resolved policy,
        // so no surface branches on the size itself.
        let viewport = context.input(egui::InputState::screen_rect);
        let policy = ViewportDensityPolicy::resolve(viewport.width());
        let bands = policy.bands();

        let context_panel = egui::TopBottomPanel::top("crest-context-line")
            .exact_height(bands.context_line_px)
            .frame(shell_frame(SemanticColor::BgElevated))
            .show(context, |ui| paint_context_line(ui, projection));

        let identity_panel = egui::TopBottomPanel::top("crest-identity-header")
            .exact_height(bands.identity_header_px)
            .frame(shell_frame(SemanticColor::BgPanel))
            .show(context, |ui| paint_identity_header(ui, projection));

        let mut passive_action = None;
        let footer_panel = egui::TopBottomPanel::bottom("crest-footer")
            .exact_height(bands.footer_px)
            .frame(shell_frame(SemanticColor::BgElevated))
            .show(context, |ui| passive_action = paint_footer(ui, projection));

        let side_panel = egui::SidePanel::right("crest-persistent-side-region")
            .exact_width(policy.split().side_px)
            .frame(shell_frame(SemanticColor::BgPanel))
            .show(context, |ui| paint_side_region(ui, projection));

        let main_panel = egui::CentralPanel::default()
            .frame(shell_frame(SemanticColor::BgCanvas))
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

/// Resolves the chrome the rendering stack paints for itself to authored roles.
///
/// Most of the shell names its own colors at the call site. A handful are
/// drawn by the stack rather than by this adapter — the rule between two
/// panels, the indent rule beside an expanded body, a disclosure triangle, the
/// recessed track behind a meter — and those come from the stack's own default
/// visuals, which are grays the vocabulary does not declare. Naming them once
/// here is what makes "every color the shell paints resolves through the
/// vocabulary" true of the whole frame rather than of the part this file
/// happens to paint by hand.
///
/// Idempotent, and cheap on every frame but the first: the style is only
/// rewritten when it does not already carry the authored rule.
fn install_authored_chrome(context: &egui::Context) {
    let rule = SemanticColor::BorderDefault.resolve();
    if context
        .style()
        .visuals
        .widgets
        .noninteractive
        .bg_stroke
        .color
        == rule
    {
        return;
    }
    context.style_mut(|style| {
        let visuals = &mut style.visuals;
        // The rule between two panels, and beside an indented body.
        visuals.widgets.noninteractive.bg_stroke.color = rule;
        // Glyph chrome the stack draws itself, at rest and under interaction.
        visuals.widgets.noninteractive.fg_stroke.color = SemanticColor::TextSecondary.resolve();
        visuals.widgets.inactive.fg_stroke.color = SemanticColor::TextSecondary.resolve();
        visuals.widgets.open.fg_stroke.color = SemanticColor::TextSecondary.resolve();
        visuals.widgets.hovered.fg_stroke.color = SemanticColor::TextPrimary.resolve();
        visuals.widgets.active.fg_stroke.color = SemanticColor::AccentFocus.resolve();
        // The recessed background behind a meter's filled portion.
        visuals.extreme_bg_color = SemanticColor::BgCanvas.resolve();
    });
}

fn shell_frame(fill: SemanticColor) -> egui::Frame {
    egui::Frame::new()
        .fill(fill.resolve())
        .inner_margin(egui::Margin::ZERO)
        .outer_margin(egui::Margin::ZERO)
}

fn paint_context_line(ui: &mut egui::Ui, projection: &GraphicalShellProjection) {
    let mode_color = match projection.semantic_model().interaction_mode() {
        crate::control::InteractionMode::Navigate => SemanticColor::TextPrimary,
        crate::control::InteractionMode::Adjust => SemanticColor::AccentAdjust,
        crate::control::InteractionMode::Modal | crate::control::InteractionMode::MultiSelect => {
            SemanticColor::TextMuted
        }
    };
    StripBuilder::new(ui)
        .size(Size::relative(CONTEXT_PRODUCT_FRACTION))
        .size(Size::relative(CONTEXT_MODE_FRACTION))
        .size(Size::remainder())
        .horizontal(|mut strip| {
            strip.cell(|ui| {
                padded_text(
                    ui,
                    projection.context_line().product_label(),
                    TypeStyle::LabelControl,
                    SemanticColor::TextPrimary,
                );
            });
            strip.cell(|ui| {
                ui.centered_and_justified(|ui| {
                    ui.horizontal(|ui| {
                        chrome_text(
                            ui,
                            projection.context_line().context_label(),
                            TypeStyle::LabelControl,
                            mode_color,
                        );
                        chrome_text(
                            ui,
                            format!(
                                "· {}",
                                projection.semantic_model().interaction_mode().label()
                            ),
                            TypeStyle::LabelControl,
                            mode_color,
                        );
                    });
                });
            });
            strip.cell(|ui| {
                trailing_text(
                    ui,
                    projection.context_line().status_label(),
                    TypeStyle::InstructionHint,
                    SemanticColor::TextMuted,
                );
            });
        });
}

/// Paints the identity band's two labels.
///
/// The two rows are sized by the authored type they carry rather than by a
/// fixed split: the identity band is 72 px on the desktop and 60 px on the
/// Steam Deck, and a split tuned to the larger band overflows the smaller one.
/// `Heading/Section` over `Body/Compact`, inset once at the top and separated
/// by one authored step, fits inside the smaller band with the larger one to
/// spare.
fn paint_identity_header(ui: &mut egui::Ui, projection: &GraphicalShellProjection) {
    ui.add_space(SpacingStep::S12.resolve());
    ui.horizontal(|ui| {
        ui.add_space(SpacingStep::S16.resolve());
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = SpacingStep::S4.resolve();
            chrome_text(
                ui,
                projection.identity_header().primary_label(),
                TypeStyle::HeadingSection,
                SemanticColor::TextPrimary,
            );
            chrome_text(
                ui,
                projection.identity_header().secondary_label(),
                TypeStyle::BodyCompact,
                SemanticColor::TextSecondary,
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
        .size(Size::exact(WORKSPACE_TITLE_ROW_PX))
        .size(Size::remainder())
        .vertical(|mut strip| {
            strip.cell(|ui| {
                padded_text(
                    ui,
                    projection.workspace().main_label(),
                    TypeStyle::HeadingPanel,
                    SemanticColor::TextPrimary,
                );
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
            ui.add_space(SpacingStep::S12.resolve());
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
                        chrome_text(
                            ui,
                            track_id.to_string(),
                            TypeStyle::InstructionHint,
                            SemanticColor::TextSecondary,
                        );
                    }
                });
                hairline_separator(ui);
                egui::ScrollArea::horizontal()
                    .animated(false)
                    .id_salt("crest-synth-mixer-tracks")
                    .show(ui, |ui| {
                        ui.horizontal_top(|ui| {
                            for track_id in crate::mixer::mixer_track_id::MixerTrackId::ALL {
                                ui.push_id(track_id.index(), |ui| {
                                    egui::Frame::new()
                                        .fill(SemanticColor::BgPanel.resolve())
                                        .stroke(egui::Stroke::new(
                                            KEYLINE_RESTING_PX,
                                            SemanticColor::BorderDefault.resolve(),
                                        ))
                                        .inner_margin(egui::Margin::same(margin(SpacingStep::S8)))
                                        .show(ui, |ui| {
                                            // A track is a column: its name,
                                            // its meter, and its four controls
                                            // stack. The frame inherits the
                                            // strip's horizontal layout, so
                                            // without this the six of them run
                                            // left to right inside the column
                                            // and each row's right-aligned
                                            // value lands on top of its label.
                                            ui.vertical(|ui| {
                                                ui.set_width(MIXER_TRACK_MIN_WIDTH_PX);
                                                chrome_text(
                                                    ui,
                                                    track_id.to_string(),
                                                    TypeStyle::HeadingPanel,
                                                    SemanticColor::TextPrimary,
                                                );
                                                let meter = if observation_matches {
                                                    audio_observation.track(track_id)
                                                } else {
                                                    crate::mixer::track_meter::TrackMeter::ZERO
                                                };
                                                ui.add(
                                                    egui::ProgressBar::new(
                                                        meter.rms().clamp(0.0, 1.0),
                                                    )
                                                    .fill(SemanticColor::AccentPositive.resolve())
                                                    // The bar carries a
                                                    // `Code/Value` run, so it
                                                    // is as tall as that style
                                                    // sets. The stack's own
                                                    // default is shorter than
                                                    // the line it holds and
                                                    // clips the reading.
                                                    .desired_height(
                                                        TypeStyle::CodeValue
                                                            .metrics()
                                                            .line_height_px,
                                                    )
                                                    .text(text::text_run(
                                                        format!("METER {:.3}", meter.rms()),
                                                        TypeStyle::CodeValue,
                                                        SemanticColor::TextPrimary,
                                                        ComponentState::Resting,
                                                    ))
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
    ui.add_space(SpacingStep::S12.resolve());
    hairline_separator(ui);
    // The collapsing header is a click target whose height the rendering stack
    // sizes from its own default interact size. Naming the authored minimum
    // here is what holds it at or above the declared target height; its colors
    // come from `install_authored_chrome`.
    ui.spacing_mut().interact_size.y = MIN_INTERACTIVE_TARGET_PX;
    egui::CollapsingHeader::new(text::text_run(
        "DIAGNOSTIC",
        TypeStyle::HeadingPanel,
        SemanticColor::TextPrimary,
        ComponentState::Resting,
    ))
    .default_open(true)
    .show(ui, |ui| {
        chrome_text(
            ui,
            projection.workspace().diagnostic().body(),
            TypeStyle::BodyCompact,
            SemanticColor::TextMuted,
        );
    });
}

fn paint_side_region(ui: &mut egui::Ui, projection: &GraphicalShellProjection) {
    StripBuilder::new(ui)
        .size(Size::exact(WORKSPACE_TITLE_ROW_PX))
        .size(Size::remainder())
        .vertical(|mut strip| {
            strip.cell(|ui| {
                padded_text(
                    ui,
                    projection.workspace().side_label(),
                    TypeStyle::HeadingPanel,
                    SemanticColor::TextPrimary,
                );
            });
            strip.cell(|ui| {
                egui::ScrollArea::vertical()
                    .animated(false)
                    .id_salt("crest-synth-side-controls")
                    .show(ui, |ui| {
                        ui.add_space(SpacingStep::S12.resolve());
                        ui.horizontal(|ui| {
                            ui.add_space(SpacingStep::S16.resolve());
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
                                    chrome_text(
                                        ui,
                                        "NO ERRORS",
                                        TypeStyle::LabelControl,
                                        SemanticColor::AccentPositive,
                                    );
                                } else {
                                    for error in semantic.errors() {
                                        text::paint_text(
                                            ui,
                                            error.label(),
                                            TypeStyle::LabelControl,
                                            SemanticColor::AccentWarning,
                                            ComponentState::Error,
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
            chrome_text(
                ui,
                format!("SELECTED {focused_track}"),
                TypeStyle::CodeValue,
                SemanticColor::TextPrimary,
            );
            if routed_patches.is_empty() {
                chrome_text(
                    ui,
                    "ROUTED PATCHES · EMPTY",
                    TypeStyle::BodyCompact,
                    SemanticColor::TextMuted,
                );
            } else {
                for patch in routed_patches {
                    chrome_text(
                        ui,
                        format!(
                            "PATCH {:02} · {}",
                            patch.patch_id().value(),
                            patch.patch_name()
                        ),
                        TypeStyle::BodyCompact,
                        SemanticColor::TextMuted,
                    );
                }
            }
        }
        _ => {
            chrome_text(
                ui,
                format!("{summary:?}"),
                TypeStyle::BodyCompact,
                SemanticColor::TextMuted,
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
        .size(Size::relative(FOOTER_PATH_FRACTION))
        .size(Size::remainder())
        .horizontal(|mut strip| {
            strip.cell(|ui| {
                padded_text(
                    ui,
                    projection.footer().path_label(),
                    TypeStyle::InstructionHint,
                    SemanticColor::TextPrimary,
                );
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
                                let button = egui::Button::new(text::text_run(
                                    label,
                                    TypeStyle::InstructionHint,
                                    SemanticColor::TextSecondary,
                                    ComponentState::Resting,
                                ))
                                .fill(SemanticColor::BgSurface.resolve())
                                .stroke(egui::Stroke::new(
                                    KEYLINE_RESTING_PX,
                                    SemanticColor::BorderDefault.resolve(),
                                ))
                                // A valid-action button is pointer- and
                                // touch-addressable, so it carries the
                                // authored minimum target height. Width is
                                // left to the label: the minimum binds the
                                // dimension the text does not already set.
                                .min_size(egui::vec2(0.0, MIN_INTERACTIVE_TARGET_PX));
                                if ui.add(button).clicked() && emitted.is_none() {
                                    emitted = Some(valid.action().clone());
                                }
                            }
                        });
                    });
            });
        });
    emitted
}

/// Resolves the one state a control row is painted in.
///
/// Nothing is decided here. Every input is already in the projection; this
/// only names which declared appearance renders it, so the row's keyline,
/// halo, and text treatment all come from one place instead of three
/// independent branches.
///
/// Focus outranks a typed failure because focus must stay visible while an
/// error is being read; the failure text is painted alongside regardless.
fn control_state(
    control: &crate::control::SemanticControlViewModel,
    mode: crate::control::InteractionMode,
) -> ComponentState {
    if !control.enabled() {
        ComponentState::Disabled
    } else if control.focused() {
        if mode == crate::control::InteractionMode::Adjust {
            ComponentState::Adjusting
        } else {
            ComponentState::Focused
        }
    } else if control.error().is_some() {
        ComponentState::Error
    } else {
        ComponentState::Resting
    }
}

fn paint_semantic_control(
    ui: &mut egui::Ui,
    control: &crate::control::SemanticControlViewModel,
    mode: crate::control::InteractionMode,
) {
    let state = control_state(control, mode);
    let appearance = state.appearance();
    let mut frame = egui::Frame::new()
        .stroke(egui::Stroke::new(
            appearance.keyline_px,
            appearance.accent.resolve(),
        ))
        // The left inset is the reserved cursor column: the `>` is painted
        // into it below, and reserving it for every state is what keeps a
        // row's text still when it gains focus.
        .inner_margin(egui::Margin {
            left: focus::LABEL_START_X_PX as i8,
            right: margin(SpacingStep::S12),
            top: margin(SpacingStep::S8),
            bottom: margin(SpacingStep::S8),
        });
    if appearance.fills_row {
        frame = frame.fill(appearance.accent.resolve());
    }
    if appearance.draws_halo {
        frame = frame.shadow(focus::halo(appearance.accent));
    }
    let response = frame.show(ui, |ui| {
        // A control row is a focus and adjustment target, so its painted
        // height carries the authored minimum rather than whatever its
        // content and margins happen to compose to. The margins are the
        // frame's own, so the inner minimum is the target less both of them.
        ui.set_min_height(MIN_INTERACTIVE_TARGET_PX - 2.0 * SpacingStep::S8.resolve());
        ui.horizontal(|ui| {
            text::paint_text(
                ui,
                control.label(),
                TypeStyle::LabelControl,
                SemanticColor::TextPrimary,
                state,
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                text::paint_text(
                    ui,
                    semantic_value_label(control.value()),
                    TypeStyle::CodeValue,
                    value::value_color(state),
                    state,
                );
                // Structural rows carry their reducer-owned lifecycle and
                // any typed refusal as text, never colour alone; the
                // adapter renders the projection without recomputing it.
                if let Some(error) = control.error() {
                    text::paint_text(
                        ui,
                        error.label(),
                        TypeStyle::InstructionHint,
                        SemanticColor::AccentWarning,
                        ComponentState::Error,
                    );
                }
                if let Some(status) = control.status().filter(|status| {
                    status.kind() != crate::control::EngineSelectionStatusKind::Ready
                }) {
                    text::paint_text(
                        ui,
                        status.label(),
                        TypeStyle::InstructionHint,
                        SemanticColor::AccentAdjust,
                        ComponentState::Loading,
                    );
                }
            });
        });
    });
    focus::cursor(ui.painter(), response.response.rect, state);
    if control.focused() && !ui.clip_rect().contains(response.response.rect.center()) {
        response.response.scroll_to_me(Some(egui::Align::Center));
    }
    ui.add_space(SpacingStep::S4.resolve());
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

/// Paints one run of shell chrome.
///
/// Chrome is not a component: it carries no behavioral state, so it always
/// paints at rest. Routing it through the text primitive anyway is what keeps
/// a type style resolvable in exactly one place.
fn chrome_text(
    ui: &mut egui::Ui,
    content: impl Into<String>,
    style: TypeStyle,
    color: SemanticColor,
) {
    text::paint_text(ui, content, style, color, ComponentState::Resting);
}

/// Paints a run of chrome inset from the top and left of its cell.
fn padded_text(ui: &mut egui::Ui, label: &str, style: TypeStyle, color: SemanticColor) {
    ui.add_space(SpacingStep::S12.resolve());
    ui.horizontal(|ui| {
        ui.add_space(SpacingStep::S16.resolve());
        chrome_text(ui, label, style, color);
    });
}

/// Paints a run of chrome against the right edge of its cell.
fn trailing_text(ui: &mut egui::Ui, label: &str, style: TypeStyle, color: SemanticColor) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.add_space(SpacingStep::S16.resolve());
        chrome_text(ui, label, style, color);
    });
}

/// Separates two stacked regions with the authored resting hairline.
///
/// `egui::Ui::separator` would paint egui's own non-interactive stroke, which
/// is neither the authored width nor the authored color.
fn hairline_separator(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), SpacingStep::S8.resolve()),
        egui::Sense::hover(),
    );
    rules::hairline(
        ui.painter(),
        rules::RuleSpan::Horizontal {
            y_px: rect.center().y,
            from_x_px: rect.min.x,
            to_x_px: rect.max.x,
        },
    );
}

/// Resolves an authored spacing step to the integer margin `egui::Margin`
/// carries. Every authored step is a whole number of pixels, so nothing is
/// lost.
const fn margin(step: SpacingStep) -> i8 {
    step.resolve() as i8
}

/// Installs the authored typeface into `context`.
///
/// Every owner of an `egui::Context` this adapter paints into calls this once,
/// before the first painted frame. The production window does so during eframe
/// setup; a test or scene that builds its own context calls it directly. There
/// is no fallback path: an unregistered authored family is a loud failure
/// rather than a plausible-looking substitution.
pub fn install_authored_typeface(context: &egui::Context) -> Result<(), TypefaceError> {
    install_typeface(context, &AuthoredTypeface::load()?);
    Ok(())
}

fn install_typeface(context: &egui::Context, typeface: &AuthoredTypeface) {
    context.set_fonts(typeface.font_definitions());
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
        egui::Key::Num3 => WindowKey::Digit3,
        egui::Key::Num4 => WindowKey::Digit4,
        egui::Key::Num5 => WindowKey::Digit5,
        egui::Key::Num6 => WindowKey::Digit6,
        egui::Key::Num7 => WindowKey::Digit7,
        egui::Key::Num8 => WindowKey::Digit8,
        egui::Key::Q => WindowKey::Q,
        egui::Key::E => WindowKey::E,
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
    use super::{normalize_egui_event, translate_egui_event, EframeGraphicalWindow};
    use crate::control::app_event::Direction;
    use crate::control::{InteractionMode, SemanticAction};
    use crate::shell::keyboard_input_translator::KeyboardInputTranslator;
    use crate::shell::visual::ViewportDensityPolicy;
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

    /// The two reference side widths still resolve, now from the density
    /// policy rather than from a proportional rule the adapter carried itself.
    #[test]
    fn graphical_window_uses_the_two_reference_side_widths() {
        assert_eq!(
            ViewportDensityPolicy::resolve(1_920.0).split().side_px,
            420.0
        );
        assert_eq!(
            ViewportDensityPolicy::resolve(1_280.0).split().side_px,
            320.0
        );
    }

    /// Every band the shell paints comes from the resolved policy, and the
    /// desktop bands are the ones the adapter used to carry as constants.
    #[test]
    fn graphical_window_resolves_the_desktop_bands_it_used_to_carry() {
        let desktop = ViewportDensityPolicy::resolve(1_920.0);
        assert_eq!(desktop, ViewportDensityPolicy::Desktop);
        assert_eq!(desktop.bands().context_line_px, 48.0);
        assert_eq!(desktop.bands().identity_header_px, 72.0);
        assert_eq!(desktop.bands().footer_px, 64.0);
        assert_eq!(
            desktop.authored_viewport().width_px,
            1_920.0,
            "the window opens at the authored desktop size"
        );
        assert_eq!(desktop.authored_viewport().height_px, 1_080.0);
        assert_eq!(
            ViewportDensityPolicy::SteamDeck
                .authored_viewport()
                .width_px,
            1_280.0,
            "the minimum window size is the authored Steam Deck size"
        );
        assert_eq!(
            ViewportDensityPolicy::SteamDeck
                .authored_viewport()
                .height_px,
            800.0
        );
    }

    #[test]
    fn graphical_window_normalizes_the_complete_key_vocabulary() {
        let cases = [
            (Key::Num1, WindowKey::Digit1),
            (Key::Num2, WindowKey::Digit2),
            (Key::Num3, WindowKey::Digit3),
            (Key::Num4, WindowKey::Digit4),
            (Key::Num5, WindowKey::Digit5),
            (Key::Num6, WindowKey::Digit6),
            (Key::Num7, WindowKey::Digit7),
            (Key::Num8, WindowKey::Digit8),
            (Key::Q, WindowKey::Q),
            (Key::E, WindowKey::E),
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

    /// The digits the gallery pages with normalize at the boundary and stop
    /// there. A normalized key that produced an approximate action would be
    /// worse than one that produced none.
    #[test]
    fn graphical_window_normalizes_the_unbound_digits_without_binding_them() {
        let mut translator = KeyboardInputTranslator::new();
        for (egui_key, window_key) in [
            (Key::Num3, WindowKey::Digit3),
            (Key::Num4, WindowKey::Digit4),
            (Key::Num5, WindowKey::Digit5),
            (Key::Num6, WindowKey::Digit6),
            (Key::Num7, WindowKey::Digit7),
            (Key::Num8, WindowKey::Digit8),
        ] {
            assert_eq!(
                normalize_egui_event(&key_event(egui_key, true)),
                Some(WindowInput::key_down(window_key))
            );
            assert_eq!(
                translate_egui_event(&mut translator, &key_event(egui_key, true)),
                None,
                "{window_key:?} produced a semantic action"
            );
            assert_eq!(
                translate_egui_event(&mut translator, &key_event(egui_key, false)),
                None
            );
        }
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
