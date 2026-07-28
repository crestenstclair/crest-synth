## ADDED Requirements

### Requirement: PATCH schema projection, focus, and effect edits remain canonical and audio-structure neutral
The canonical PATCH resolver SHALL append each configured ordered effect's read-only identity and visible enabled descriptor parameters classified `ScalarEdit` after Engine, common ADSR, and instrument `StructuralChoice` controls. Effect control identity SHALL include stable `EffectSlotId` and `ParameterId`; the projector, reducer, basic window, and coverage resolver SHALL NOT branch on `effect.chorus`, Amount, or Depth. Patches with no effect config SHALL expose no effect row or placeholder.

#### Scenario: Configured first Patch is focused
- **WHEN** PATCH projects the first production fixture Patch
- **THEN** its exact nonwrapping focus order ends with read-only Chorus identity followed by Amount and Depth and the page/text selected marker agrees with reducer-owned focus

#### Scenario: Unconfigured Patch is projected
- **WHEN** a production fixture Patch with an empty post-effect list is focused
- **THEN** PATCH ends after its applicable instrument controls and contains no Chorus label, disabled slot, add button, selector, or fabricated effect value

#### Scenario: Effect descriptor changes in a conforming fixture
- **WHEN** a test descriptor reorders, relabels, hides, disables, adds, or removes a `ScalarEdit` parameter within fixed capacity
- **THEN** focus, page/text projection, schema coverage, and adjustment targeting follow that descriptor and a stale duplicated field list fails exact verification

**Contract facet — focus and edits remain audio-structure neutral.**
Moving PATCH focus onto or among effect rows SHALL change only reducer-owned focus, generation, and coherent logical projections. Adjusting a valid effect scalar SHALL change only the matching canonical assignment and fixed scalar projection. Neither operation SHALL enqueue MIDI, prepare/publish a graph, replace ownership, or alter graph revision.

#### Scenario: Focus moves through Chorus controls
- **WHEN** bare Down moves from the last instrument control through Chorus identity, Amount, and Depth
- **THEN** exactly one adjacent stable control is selected at each step while every Patch/config value, fixed scalar value, graph revision, queue, and rendered sample remains unchanged

#### Scenario: Effect scalar is edited while a structural request is pending
- **WHEN** Amount or Depth receives a valid edit during Preparing or Activating
- **THEN** PATCH preserves exact focus and structural status, commits and projects the scalar through the compatible snapshot path, and does not start or replace structural work
