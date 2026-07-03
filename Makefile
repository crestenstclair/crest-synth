# path: Makefile

.PHONY: help build test lint fmt tone smoke play ui

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
	cargo run --bin synth_ui -- --smoke --play $(DEFAULT_MIDI)

play: ## Play a MIDI file (FILE=path/to.mid), defaults to midi/Megalovania.mid
	cargo run --bin synth_ui -- --play $(if $(FILE),$(FILE),$(DEFAULT_MIDI))

ui: ## Launch the synth_ui app windowed (set FILE=path/to.mid to also play it)
ifdef FILE
	cargo run --bin synth_ui -- --play $(FILE)
else
	cargo run --bin synth_ui
endif
