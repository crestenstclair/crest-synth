//! Spike harness: dump the production MIXER semantic view model as JSON.
//!
//! Part of the webview UI-stack spike (Op 01KZ9CN8E0YEMQEZQG8Q7VEESM).
//! Run explicitly with:
//!
//! ```sh
//! cargo test --test spike_webview_view_model_dump -- --ignored
//! ```
//!
//! Writes `spike/webview-mixer/view-model.json`, the exact serialized
//! [`SemanticGraphicalViewModel`] the production projector emits for the
//! production fixture with the MIXER context selected. The webview spike
//! renders from this file; nothing here invents a second schema.

use crest_synth::adapter::braids_capability::BraidsCapability;
use crest_synth::adapter::production_effects::{
    production_chorus_config, production_effect_registry,
};
use crest_synth::adapter::production_instruments::{
    production_capability_registry, production_soundfont_capability,
};
use crest_synth::control::{AppEvent, AppState, StateProjector, TopLevelContext};
use crest_synth::kernel::{MidiChannel, PatchId};
use crest_synth::mixer::global_parameters::GlobalParameters;
use crest_synth::mixer::mixer_track_id::MixerTrackId;
use crest_synth::mixer::patch_output::PatchOutput;
use crest_synth::synth::effect_slot_id::EffectSlotIndex;
use crest_synth::synth::sound_font_instrument::SoundFontInstrument;
use crest_synth::synth::{EffectSlotId, InstrumentConfig, Patch};
use crest_synth::testing::automatic_midi_test::create_soundfont_config;
use std::fs;
use std::path::Path;

fn soundfont_config() -> InstrumentConfig {
    create_soundfont_config(
        &production_soundfont_capability().unwrap(),
        SoundFontInstrument::new(0, 40, false).unwrap(),
    )
    .unwrap()
}

fn production_fixture_state() -> AppState {
    let soundfont = soundfont_config();
    let braids = BraidsCapability::new().unwrap().default_config().unwrap();
    let patches = [soundfont, braids]
        .into_iter()
        .enumerate()
        .map(|(index, config)| {
            let patch = Patch::new(
                PatchId::new(index as u32 + 1).unwrap(),
                format!("Semantic {}", index + 1),
                config,
                MidiChannel::new(index as u8).unwrap(),
                PatchOutput::new(MixerTrackId::new(index as u8).unwrap(), -5.0 - index as f32)
                    .unwrap(),
            );
            if index == 0 {
                patch.with_effect_slot(
                    EffectSlotIndex::ALL[0],
                    production_chorus_config(EffectSlotId::new(1).unwrap()).unwrap(),
                )
            } else {
                patch
            }
        })
        .collect();
    let mut state = AppState::new_with_effects(
        production_capability_registry().unwrap(),
        production_effect_registry().unwrap(),
        GlobalParameters::new(-3.0).unwrap(),
    );
    state.apply(AppEvent::InstallPatches(patches)).unwrap();
    state
}

#[test]
#[ignore = "spike harness: writes spike/webview-mixer/view-model.json on demand"]
fn dump_production_mixer_view_model() {
    let mut state = production_fixture_state();
    state
        .apply(AppEvent::SelectContext(TopLevelContext::Mixer))
        .unwrap();
    let semantic = StateProjector::new()
        .project_with_shell(&state)
        .unwrap()
        .3
        .semantic_model()
        .clone();
    let json = serde_json::to_string_pretty(&semantic).unwrap();
    let out_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("spike/webview-mixer");
    fs::create_dir_all(&out_dir).unwrap();
    fs::write(out_dir.join("view-model.json"), &json).unwrap();
    println!("wrote {} bytes", json.len());
}
