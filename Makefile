.PHONY: build test check clean run lint fmt demo-midi demo-voices check-live check-gamepad ui-smoke ui play-midi play-voices play-tone play-midi-live demo-patches play-patches demo-mod play-mod demo-samples play-samples demo-effects play-effects demo-presets play-presets

.DEFAULT_GOAL := build

build:
	cargo build

test:
	cargo test

check:
	cargo check

clean:
	cargo clean

run:
	cargo run --bin crest-synth

lint:
	cargo clippy -- -D warnings

fmt:
	cargo fmt -- --check

demo-midi:
	cargo run --bin midi_play -- $(FILE)

demo-voices:
	cargo run --bin voice_demo

check-live:
	cargo run --bin midi_play_live -- --no-device-dry-run

check-gamepad:
	cargo run --bin gamepad_demo

play-midi: demo-midi
	afplay midi-play.wav

play-voices: demo-voices
	afplay voice-demo.wav

play-tone:
	cargo run --bin crest-synth
	afplay tone-test.wav

play-midi-live:
	cargo run --bin midi_play_live -- $(FILE)

demo-patches:
	cargo run --bin patch_play -- $(FILE)

play-patches: demo-patches
	afplay patch-play.wav

demo-mod:
	cargo run --bin mod_play -- $(FILE)

play-mod: demo-mod
	afplay mod-play.wav

demo-samples:
	cargo run --bin sample_demo

play-samples: demo-samples
	afplay sample-demo.wav

demo-effects:
	cargo run --bin effects_demo -- $(FILE)

play-effects: demo-effects
	afplay effects-demo.wav

demo-presets:
	cargo run --bin preset_demo

play-presets: demo-presets
	afplay preset-demo.wav

ui-smoke:
	cargo run --bin synth_ui -- --smoke

ui:
	cargo run --bin synth_ui -- --play "../../../midi/Corridors of Time - Chrono Trigger.mid"
