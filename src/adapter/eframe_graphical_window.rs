use crate::control::{GraphicalShellProjection, SemanticAction};
use crate::mixer::track_meter::TrackMeter;
use crate::real_time::AudioObservationSnapshot;
use crate::shell::app_window::{
    AppInputCallback, AppWindow, AudioObservationCallback, FrameObservationCallback,
    ProjectionCallback, TickCallback, WindowError,
};
use crate::shell::keyboard_input_translator::KeyboardInputTranslator;
use crate::shell::visual::compositions::application_shell::{
    self, BandPlacement, FrameBand, ShellFrameIntent, FRAME_BAND_COUNT,
};
use crate::shell::visual::primitives::text;
use crate::shell::visual::{
    AuthoredTypeface, ComponentState, SemanticColor, TypeStyle, TypefaceError,
    ViewportDensityPolicy,
};
use crate::shell::window_input::{WindowInput, WindowKey};
use crate::shell::{
    ShellFrameObservation, ShellFrameObservationError, ShellRegionId, ShellRegionObservation,
    ShellRegionRect,
};
use eframe::egui;
use std::time::{Duration, Instant};

/// The idle repaint cadence. A separate declared decision from the visual
/// vocabulary and not resolved through the density policy.
const FRAME_INTERVAL: Duration = Duration::from_millis(16);

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
        match paint_shell(context, &projection, audio_observation) {
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

/// Builds the frame's panels, arranges each band's composition inside the panel
/// it belongs to, and reports what was painted.
///
/// Nothing here is a visual decision. `frame_plan` supplies the bands in the
/// order they claim space, each already carrying its placement, extent, surface,
/// panel id, and composition; this only turns a placement into the panel that
/// realizes it. The observation is built afterwards from the rectangles those
/// panels actually produced, in the order `observed_bands` supplies, which is
/// the order the observation demands rather than the order the panels claimed.
fn paint_shell(
    context: &egui::Context,
    projection: &GraphicalShellProjection,
    audio_observation: AudioObservationSnapshot,
) -> Result<(ShellFrameObservation, Option<SemanticAction>), ShellFrameObservationError> {
    application_shell::install_authored_chrome(context);
    // The one place this adapter reads the raw viewport, and it reads it to
    // resolve the policy rather than to choose anything itself.
    let viewport = context.input(egui::InputState::screen_rect);
    let policy = ViewportDensityPolicy::resolve(viewport.width());

    let mut frame = ShellFrameIntent::none();
    let mut claimed: Vec<(ShellRegionId, egui::Rect)> = Vec::with_capacity(FRAME_BAND_COUNT);
    for band in application_shell::frame_plan_for(projection, &policy) {
        let painted = show_band(context, band, |ui| {
            // A band is the extent the plan declared. The panel reports the
            // rectangle its *content* filled, so a composition that lays out
            // more than fits would report a side region running through the
            // footer, and the observation would describe a frame nobody
            // painted. Arranging into a detached child claims the band and
            // leaves the panel's own extent alone, which is what keeps the
            // reported rectangle equal to the band the policy declared.
            //
            // The panel already clips to itself, so overflow is cut rather than
            // reported. Nothing here decides an extent: `bounds` is whatever
            // the plan's placement gave the panel.
            let bounds = ui.max_rect();
            ui.expand_to_include_rect(bounds);
            let mut band_ui = ui.new_child(egui::UiBuilder::new().max_rect(bounds));
            frame.absorb(application_shell::arrange_band(
                &mut band_ui,
                band,
                projection,
                &policy,
            ));
            // After the composition, and through a painter rather than the
            // layout, so the residue cannot move or shorten what the band
            // contains. See `paint_focused_track_meter`.
            if band.observed_region_id() == ShellRegionId::MainWorkspace {
                paint_focused_track_meter(&band_ui, bounds, projection, &audio_observation);
            }
        });
        claimed.push((band.observed_region_id(), painted));
    }

    let observation = ShellFrameObservation::try_new_semantic(
        viewport.width(),
        viewport.height(),
        projection.semantic_model(),
        application_shell::observed_bands(&policy).map(|band| {
            // A band the loop above did not paint has no rectangle, and
            // `Rect::NOTHING` fails the observation's own bounds check rather
            // than reporting a plausible zero for a region nothing produced.
            let painted = claimed
                .iter()
                .find(|(id, _)| *id == band.observed_region_id())
                .map_or(egui::Rect::NOTHING, |(_, rect)| *rect);
            ShellRegionObservation::new(
                band.observed_region_id(),
                relative_rect(painted.intersect(viewport), viewport.min),
                band.observed_label(projection),
            )
        }),
    )?;

    // Which valid action an addressed footer hint names is a lookup with no
    // choice in it, and it happens here so the reducer's vocabulary stays out
    // of the composition that painted the target.
    let passive_action = frame
        .activated_hint()
        .and_then(|hint| hint.resolve(projection.semantic_model().valid_actions()))
        .map(|valid| valid.action().clone());
    Ok((observation, passive_action))
}

/// Paints the focused track's meter reading — the one paint left in this file.
///
/// It is here because the level has a source, a declared painter, and no route
/// between them. `AudioObservationSnapshot` is delivered to the *window*, and
/// `adapters.yaml` assigns the track-to-observation pairing to this adapter; but
/// `ShellComposition` declares that "a composition owns no ... audio state" and
/// `CompositionRenderFn` has no argument one could arrive through, while the
/// declared home for a level is view data — `Meter` "presents whatever level the
/// view data reports" — which `SemanticControlViewModel` does not carry (F-10,
/// item 9). Closing that needs a projection change C-002 forbids here.
///
/// The shipped adapter drove sixteen readings, one per column. This paints one,
/// for the track the operator is on; the other fifteen are a recorded
/// regression. Aligning sixteen would mean restating `MixerStripBank`'s own
/// placement rule — inset, pitch, origin — in this file, which is both the
/// layout the `AppWindow` port forbids the window to decide and a second copy
/// that drifts the moment `ViewportDensityPolicy::mixer_column` changes.
///
/// The compatibility check is the shipped one, unrelaxed: a reading is shown
/// only when the observation's generation and graph revision both match the
/// projection being painted, so a stale observation reads zero rather than
/// reporting a level belonging to a graph the operator is no longer looking at.
///
/// # It takes no space, and that is the point
///
/// A residue that *displaced* the composition would be worse than the one it
/// replaced. An earlier form of this ran before `arrange_band` through
/// `ui.label`, which advances the layout cursor — so `MixerStripBank`, which
/// anchors on `available_rect_before_wrap()`, was handed a band one line shorter
/// than the policy declared and divided sixteen columns into a rect the *window*
/// had shrunk. Measured through the production paint path, that cost the bank
/// **23 px at both authored viewports**: the first track header sat at y=190
/// rather than y=167 at 1920×1080, and at y=158 rather than y=135 at 1280×800.
/// Placement and extent are exactly what `shell.yaml:377` and FR-006 deny the
/// window, and no composition-level proof can see it, because those call the
/// composition directly with a full band.
///
/// So the reading is painted after the composition, through a painter over the
/// band rectangle rather than into the band's layout. A painter allocates
/// nothing, so there is no cursor to advance and no extent to take: the bank
/// sees the whole band whether this paints or not.
///
/// The position is derived from the band and from the run's own authored line
/// height — never from the bank's inset, pitch, or origin. Horizontally centred
/// on the first line, which is the one region of the legend row neither the
/// projected title (left) nor the focus annotation (right) occupies at either
/// authored viewport.
fn paint_focused_track_meter(
    ui: &egui::Ui,
    band: egui::Rect,
    projection: &GraphicalShellProjection,
    audio_observation: &AudioObservationSnapshot,
) {
    let semantic = projection.semantic_model();
    let Some(track) = semantic.focus_path().control_id().as_mixer_track_id() else {
        return;
    };
    let compatible = audio_observation.parameter_generation() == semantic.generation()
        && audio_observation.active_graph_revision() == semantic.status().graph_revision();
    let meter = if compatible {
        audio_observation.track(track)
    } else {
        TrackMeter::ZERO
    };
    let state = ComponentState::Resting;
    let color = SemanticColor::TextSecondary;
    let painter = ui.painter_at(band);
    let run = text::layout(
        &painter,
        format!("METER {:.3}", meter.rms()),
        TypeStyle::CodeValue,
        color,
        state,
    );
    let size = run.size();
    let line = TypeStyle::CodeValue.metrics().line_height_px;
    painter.galley(
        egui::pos2(
            band.center().x - size.x / 2.0,
            band.min.y + (line - size.y) / 2.0,
        ),
        run,
        text::resolved_color(color, state).resolve(),
    );
}

/// Builds the panel one band's placement names and arranges the band inside it.
fn show_band(
    context: &egui::Context,
    band: FrameBand,
    arrange: impl FnOnce(&mut egui::Ui),
) -> egui::Rect {
    let surface = egui::Frame::new()
        .fill(band.surface().resolve())
        .inner_margin(egui::Margin::ZERO)
        .outer_margin(egui::Margin::ZERO);
    match band.placement() {
        BandPlacement::TopEdge { height_px } => {
            egui::TopBottomPanel::top(band.panel_id())
                .exact_height(height_px)
                .frame(surface)
                .show(context, arrange)
                .response
                .rect
        }
        BandPlacement::BottomEdge { height_px } => {
            egui::TopBottomPanel::bottom(band.panel_id())
                .exact_height(height_px)
                .frame(surface)
                .show(context, arrange)
                .response
                .rect
        }
        BandPlacement::TrailingEdge { width_px } => {
            egui::SidePanel::right(band.panel_id())
                .exact_width(width_px)
                .frame(surface)
                .show(context, arrange)
                .response
                .rect
        }
        BandPlacement::Remainder => {
            egui::CentralPanel::default()
                .frame(surface)
                .show(context, arrange)
                .response
                .rect
        }
    }
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
