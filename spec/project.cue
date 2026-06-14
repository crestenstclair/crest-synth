package crestsynth

// ── Project configuration ──────────────────────────────
// Layers, dependency rules, language/style meta, asset kinds, and the
// whole-crate validations run at wave verification.

project: name: "crest-synth"
project: layers: ["domain", "application", "infrastructure"]
project: layerRules: domain:         {dependsOn: []}
project: layerRules: application:    {dependsOn: ["domain"]}
project: layerRules: infrastructure: {dependsOn: ["domain", "application"]}

project: meta: {
	language: "rust"
	style:    "idiomatic Rust; lock-free audio thread; gamepad-driven UI"
	avoid: ["heap allocation on audio thread", "mutex locks on audio thread", "blocking I/O on audio thread"]
}

// ── Default whole-crate validations (run at wave verification) ──
project: validations: [
	{kind: "compiles", command: ["cargo", "fmt", "--", "--check"], description: "rustfmt clean"},
	{kind: "compiles", command: ["cargo", "clippy", "--all-targets", "--", "-D", "warnings"], description: "clippy clean (incl. tests/bins)"},
	{kind: "compiles", command: ["cargo", "build"], description: "crate builds"},
	{kind: "test", command: ["cargo", "test"], description: "tests pass"},
]

// ── Asset kinds ────────────────────────────────────────

project: assetKinds: "cargo-manifest": {
	description: "Rust Cargo.toml project manifest"
	filePattern: "Cargo.toml"
	prompts: ["Use edition 2021", "Only include dependencies actually needed by the generated code", #"Include [lib] section with path = "src/lib.rs""#]
}
project: assetKinds: makefile:                  {description: "GNU Makefile for build automation", filePattern: "Makefile", prompts: ["Include targets: build, test, clean, check, run", "Use cargo for all Rust operations"]}
project: assetKinds: "rust-binary":             {description: "Rust main.rs binary entry point", filePattern: "src/main.rs", prompts: ["Must compile and execute with `cargo run`", "Use only types from the crate's own lib"]}
project: assetKinds: "rust-module-declaration": {description: "Rust mod.rs or lib.rs module declaration file", prompts: ["Only output module declarations (pub mod) and re-exports", "Do not add any implementation code"]}
project: assetKinds: "rust-adapter":            {description: "Rust infrastructure adapter implementing a port trait", prompts: ["Implement the port trait using the specified crate", "Include proper error handling and resource cleanup"]}
project: assetKinds: "rust-bin-target": {
	description: "Rust binary in src/bin/ — a [[bin]] target runnable with `cargo run --bin <name>`"
	prompts: ["Place the file under src/bin/", "Must compile and run with `cargo run --bin <name>`", "Use only types from the crate's own lib plus declared dependencies"]
}

// ── Context map ────────────────────────────────────────
// Relationships between bounded contexts.

project: contextMap: shellToSynth:        {from: "Shell", to: "Synth", kind: "customer-supplier", direction: "downstream"}
project: contextMap: shellToPatch:        {from: "Shell", to: "Patch", kind: "customer-supplier", direction: "downstream"}
project: contextMap: patchToSynth:        {from: "Patch", to: "Synth", kind: "customer-supplier", direction: "downstream"}
project: contextMap: patchToEffects:      {from: "Patch", to: "Effects", kind: "customer-supplier", direction: "downstream"}
project: contextMap: modToSynth:          {from: "Modulation", to: "Synth", kind: "customer-supplier", direction: "downstream"}
project: contextMap: modToPatch:          {from: "Modulation", to: "Patch", kind: "customer-supplier", direction: "downstream"}
project: contextMap: sampleLibToSynth:    {from: "SampleLibrary", to: "Synth", kind: "customer-supplier", direction: "downstream"}
project: contextMap: sampleLibToRealTime: {from: "SampleLibrary", to: "RealTime", kind: "customer-supplier", direction: "downstream"}
project: contextMap: presetsToPatch:      {from: "Presets", to: "Patch", kind: "customer-supplier", direction: "downstream"}
project: contextMap: presetsToMod:        {from: "Presets", to: "Modulation", kind: "customer-supplier", direction: "downstream"}
project: contextMap: presetsToEffects:    {from: "Presets", to: "Effects", kind: "customer-supplier", direction: "downstream"}
project: contextMap: kernelToSynth:       {from: "Kernel", to: "Synth", kind: "shared-kernel"}
project: contextMap: kernelToPatch:       {from: "Kernel", to: "Patch", kind: "shared-kernel"}
project: contextMap: kernelToMod:         {from: "Kernel", to: "Modulation", kind: "shared-kernel"}
project: contextMap: realTimeToSynth:     {from: "RealTime", to: "Synth", kind: "anti-corruption", direction: "upstream"}
project: contextMap: realTimeToPatch:     {from: "RealTime", to: "Patch", kind: "anti-corruption", direction: "upstream"}
project: contextMap: pluginToSynth:       {from: "Plugin", to: "Synth", kind: "customer-supplier", direction: "downstream"}
project: contextMap: pluginToPatch:       {from: "Plugin", to: "Patch", kind: "customer-supplier", direction: "downstream"}
project: contextMap: pluginToPresets:     {from: "Plugin", to: "Presets", kind: "customer-supplier", direction: "downstream"}
