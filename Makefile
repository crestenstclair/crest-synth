# path: Makefile

.PHONY: help build test lint fmt tone smoke play ui demo-voices demo-samples demo-effects demo-mod demo-patches demo-presets demo-midi check-live ui-smoke autopilot demo-mixer check-gamepad

DEFAULT_MIDI := midi/Megalovania.mid

help: ## Show this help message
	@grep -E '^[a-zA-Z_-]+:.*##' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*##"}; {printf "  %-10s %s\n", $$1, $$2}'

build: ## Build the project (cargo build)
	cargo build

test: ## Run the test suite (cargo test)
	cargo test

lint: ## Run clippy with warnings denied
	cargo clippy --all-targets -- -D warnings

fmt: ## Format the code (cargo fmt)
	cargo fmt

tone: ## Run the tone_test proof
	cargo run --bin tone_test

smoke: ## Run the audible self-check against the default MIDI file
	cargo run --bin synth_ui -- --smoke --play "$(DEFAULT_MIDI)"

play: ## Play a MIDI file (FILE=path/to.mid), defaults to midi/Megalovania.mid
	cargo run --bin synth_ui -- --play "$(if $(FILE),$(FILE),$(DEFAULT_MIDI))"

ui: ## Launch the synth_ui app windowed (set FILE=path/to.mid to also play it)
ifdef FILE
	cargo run --bin synth_ui -- --play "$(FILE)"
else
	cargo run --bin synth_ui
endif

demo-voices: ## Run the voice allocation proof (voice_demo)
	cargo run --bin voice_demo

demo-samples: ## Run the sample playback proof (sample_demo)
	cargo run --bin sample_demo

demo-effects: ## Run the effects chain proof (effects_demo)
	cargo run --bin effects_demo

demo-mod: ## Run the modulation matrix proof (mod_play)
	cargo run --bin mod_play

demo-patches: ## Run the patch dispatch proof (patch_play)
	cargo run --bin patch_play

demo-presets: ## Run the preset save/load proof (preset_demo)
	cargo run --bin preset_demo

demo-midi: ## Render a MIDI file offline to WAV (midi_play)
	cargo run --bin midi_play

check-live: ## Verify the live MIDI player's real-time pipeline wiring, no audio device required (midi_play_live --no-device-dry-run)
	cargo run --bin midi_play_live -- --no-device-dry-run

ui-smoke: ## Run the enriched hermetic self-check covering the mixer view + design system (no window/device/MIDI)
	cargo run --bin synth_ui -- --smoke --play "$(DEFAULT_MIDI)"

autopilot: ## Real end-to-end window+audio run that self-drives a scripted session and self-terminates
	cargo run --bin synth_ui -- --autopilot --seconds 4

demo-mixer: ## Headless prover for MixerView + its 16 ChannelStrip channels (mixer_demo)
	cargo run --bin mixer_demo

check-gamepad: ## Headless prover for GamepadNavigator/GlyphResolver (gamepad_demo)
	cargo run --bin gamepad_demo
