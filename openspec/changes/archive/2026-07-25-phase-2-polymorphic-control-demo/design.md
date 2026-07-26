## Context

The production fixture is the first mixed-capability composition edge, while keyboard and autonomous actions must still become semantic events accepted by `AppState::apply`. The live proof must consume those same projections and rendered observations.

## Decisions

### Provider selection stays at the fixture edge

Standalone composition retains the ordered providers used to build its immutable registry. `AutomaticMidiTest` requires an exact ordered provider/registry match, selects by discovery position, asks that provider to create the config, validates the result, and installs all Patches in one reducer event. It never constructs a descriptor config directly or substitutes another provider.

### Editing stays descriptor-derived

Patch mixer values, common ADSR, and descriptor-classified Scalars share one canonical editable-target resolver. `Adjust` remains the semantic mutation, and fixed real-time scalar storage is descriptor ordered and capacity checked.

### The live proof observes the production path

The live scene derives exact coverage from installed state, dispatches through the real window callback and reducer, compares state/text/scalar generations, observes bounded rendered audio, reports alternating engine identities, and cleans every active note.

## Verification

Typed unit tests cover missing providers, registry mismatch, provider failure, and atomic installation. Capability-schema, schema-surface, GUI-context, live-scene, smoke, and deterministic acceptance targets cover the complete projection and physical-path contract.
