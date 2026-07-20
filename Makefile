.DEFAULT_GOAL := help

.PHONY: help build check test lint fmt fmt-check run play ui smoke observe demo demo-live clean

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

ui: ## Launch the synth text window
	cargo run --bin crest-synth

smoke: ## Run the complete headless synth path
	cargo run --bin crest-synth -- --smoke

observe: ## Print the structured headless behavioral observation
	cargo run --bin crest-synth -- --smoke --observe

demo: ## Run the exhaustive GUI demo and structured trace
	cargo run --bin crest-synth -- --smoke --observe --demo-scene

demo-live: ## Run the paced demo in the real window with physical audio
	cargo run --bin crest-synth -- --demo-live

clean: ## Remove Cargo build output
	cargo clean
