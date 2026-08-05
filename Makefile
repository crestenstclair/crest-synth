.DEFAULT_GOAL := help

.PHONY: help build check test lint fmt fmt-check run play ui smoke observe demo demo-live demo-live-effects-and-buses demo-live-sixteen-track-mixer-routing demo-live-semantic-view-model demo-live-graphical-shell demo-live-component-library semantic-graphical-view-model-acceptance webview-tokens clean

help: ## Show the available project commands
	@grep -E '^[a-zA-Z_-]+:.*##' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*##"}; {printf "  %-12s %s\n", $$1, $$2}'

build: ## Build the library and crest-synth binary
	cargo build

check: ## Type-check all targets
	cargo check --all-targets

test: ## Run all tests
	cargo test --all-targets

lint: ## Run Clippy with warnings denied
	cargo clippy --all-targets -- -D warnings

fmt: ## Format all Rust sources
	cargo fmt --all

fmt-check: ## Verify Rust formatting without changing files
	cargo fmt --all -- --check

run: ## Launch crest-synth with its fixed SoundFont and MIDI fixture
	cargo run --bin crest-synth

play: ## Launch the automatically playing synth
	cargo run --bin crest-synth

ui: ## Launch the synth graphical window
	cargo run --bin crest-synth

smoke: ## Run the complete headless synth path
	cargo run --bin crest-synth -- --smoke

observe: ## Print the structured headless behavioral observation
	cargo run --bin crest-synth -- --smoke --observe

demo: ## Run the exhaustive GUI demo and structured trace
	cargo run --bin crest-synth -- --smoke --observe --demo-scene

demo-live: demo-live-effects-and-buses ## Run the newest optimized graphical live demo

demo-live-effects-and-buses: ## Run the cumulative effects-and-buses demo with a real window and physical audio
	cargo run --release --bin crest-synth -- --demo-live-effects-and-buses

demo-live-sixteen-track-mixer-routing: ## Run the cumulative sixteen-track mixer-routing demo with a real window and physical audio
	cargo run --release --bin crest-synth -- --demo-live-sixteen-track-mixer-routing

demo-live-semantic-view-model: ## Run the Phase Two semantic view model with a real window and physical audio
	cargo run --release --bin crest-synth -- --demo-live-semantic-view-model

demo-live-graphical-shell: ## Run the Phase One shell with a real window and physical audio
	cargo run --release --bin crest-synth -- --demo-live-graphical-shell

# Browsable by hand, not autonomous: it waits for the operator and is
# deliberately not part of the demo-live alias group above.
demo-live-component-library: ## Browse the component gallery by hand — digits 1-9 and 0 select the first ten pages, [ and ] step through all fifteen, closing the window finishes
	cargo run --release --bin crest-synth -- --demo-live-component-library

semantic-graphical-view-model-acceptance: ## Prove the deterministic Phase Two semantic view model
	cargo test --test semantic_graphical_view_model -- --nocapture

webview-tokens: ## Regenerate webview-page/tokens.css from the authored Rust vocabulary
	cargo test --lib token_export::tests::write_tokens_css -- --ignored

clean: ## Remove Cargo build output
	cargo clean
