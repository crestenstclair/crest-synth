.DEFAULT_GOAL := help

.PHONY: help cache-guard cache-status cache-prune-preview cache-prune build check test test-webview-detail-native lint fmt fmt-check run play ui smoke observe demo demo-live demo-live-detail-and-assets demo-live-mixer demo-live-effects-and-buses demo-live-patch-editor demo-live-sixteen-track-mixer-routing demo-live-semantic-view-model demo-live-graphical-shell demo-live-component-library semantic-graphical-view-model-acceptance webview-tokens clean

help: ## Show the available project commands
	@grep -E '^[a-zA-Z_-]+:.*##' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*##"}; {printf "  %-12s %s\n", $$1, $$2}'

cache-guard:
	@scripts/check_build_cache_size.sh --guard

cache-status: ## Report target/ usage and its configured limits
	@scripts/check_build_cache_size.sh --status

cache-prune-preview: ## Preview removal of debug and test build artifacts
	cargo clean --dry-run --profile dev

cache-prune: ## Remove debug and test artifacts while retaining release output
	cargo clean --profile dev

build: cache-guard ## Build the library and crest-synth binary
	cargo build

check: cache-guard ## Type-check all targets
	cargo check --all-targets

test: cache-guard ## Run all tests
	cargo test --all-targets

test-webview-detail-native: cache-guard ## Run the real-window Detail/Mixer witness without unrelated soak/fault scenes
	CREST_WEBVIEW_TESTS=1 CREST_WEBVIEW_DETAIL_WITNESS=1 cargo test --test webview_projection_shell -- --nocapture

lint: cache-guard ## Run Clippy with warnings denied
	cargo clippy --all-targets -- -D warnings

fmt: ## Format all Rust sources
	cargo fmt --all

fmt-check: ## Verify Rust formatting without changing files
	cargo fmt --all -- --check

run: cache-guard ## Launch crest-synth with its fixed SoundFont and MIDI fixture
	cargo run --bin crest-synth

play: cache-guard ## Launch the automatically playing synth
	cargo run --bin crest-synth

ui: cache-guard ## Launch the synth graphical window
	cargo run --bin crest-synth

smoke: cache-guard ## Run the complete headless synth path
	cargo run --bin crest-synth -- --smoke

observe: cache-guard ## Print the structured headless behavioral observation
	cargo run --bin crest-synth -- --smoke --observe

demo: cache-guard ## Run the exhaustive GUI demo and structured trace
	cargo run --bin crest-synth -- --smoke --observe --demo-scene

# The five retained live targets below (and the demo-live alias) run on the
# webview shell: every --demo-live-* mode composes its own TauriWebviewWindow
# inside StandaloneApplication::run_live_demo_scene. The webview shell is the product's
# only shell — the interactive and headless targets compose the same
# TauriWebviewWindow directly and no renderer flag exists. Target names are
# retained as stable operator-facing compatibility targets.
demo-live: demo-live-detail-and-assets ## Run the newest optimized graphical live demo

demo-live-detail-and-assets: cache-guard ## Run the cumulative Phase 7 detail, choice, browser, preview, and Sample assignment demo
	cargo run --release --bin crest-synth -- --demo-live-detail-and-assets

# Controlled negative (must exit non-zero on preview reach/audio predicates):
#   cargo run --release --bin crest-synth -- --demo-live-detail-and-assets --defeat-detail-and-assets-preview

demo-live-mixer: cache-guard ## Run the dedicated sixteen-track Mixer demo with a real window, MIDI probes, and physical audio
	cargo run --release --bin crest-synth -- --demo-live-mixer

demo-live-effects-and-buses: cache-guard ## Run the cumulative effects-and-buses demo with a real window and physical audio
	cargo run --release --bin crest-synth -- --demo-live-effects-and-buses

# Additive: the demo-live alias above keeps pointing at the cumulative
# detail-and-assets scene, and this scene does not subsume it. Its subject is
# the SECOND installed Patch, reached through the on-screen SelectPatch
# gesture. Its declared controlled negative removes that gesture:
#   cargo run --release --bin crest-synth -- --demo-live-patch-editor --defeat-patch-selection
# which must exit 1 on the reach predicates.
demo-live-patch-editor: cache-guard ## Run the functional Patch editor demo on the second Patch with a real window and physical audio
	cargo run --release --bin crest-synth -- --demo-live-patch-editor

demo-live-sixteen-track-mixer-routing: cache-guard ## Run the cumulative sixteen-track mixer-routing demo with a real window and physical audio
	cargo run --release --bin crest-synth -- --demo-live-sixteen-track-mixer-routing

demo-live-semantic-view-model: cache-guard ## Run the Phase Two semantic view model with a real window and physical audio
	cargo run --release --bin crest-synth -- --demo-live-semantic-view-model

demo-live-graphical-shell: cache-guard ## Run the Phase One shell with a real window and physical audio
	cargo run --release --bin crest-synth -- --demo-live-graphical-shell

# Browsable by hand, not autonomous: it waits for the operator and is
# deliberately not part of the demo-live alias group above.
demo-live-component-library: cache-guard ## Browse the component gallery by hand — digits 1-9 and 0 select the first ten pages, [ and ] step through all fifteen, closing the window finishes
	cargo run --release --bin crest-synth -- --demo-live-component-library

semantic-graphical-view-model-acceptance: cache-guard ## Prove the deterministic Phase Two semantic view model
	cargo test --test semantic_graphical_view_model -- --nocapture

webview-tokens: cache-guard ## Regenerate webview-page/tokens.css from the authored Rust vocabulary
	cargo test --lib token_export::tests::write_tokens_css -- --ignored

clean: ## Remove Cargo build output
	cargo clean
