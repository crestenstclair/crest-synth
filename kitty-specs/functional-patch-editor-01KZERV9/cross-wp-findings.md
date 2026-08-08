# Cross-WP findings — functional-patch-editor-01KZERV9

Findings raised during implementation that outlive the work package that raised
them. Recorded here so none is silently inherited.

## F-01 — FR-009 must not be credited to WP01 alone

**Raised by**: WP01's review
**Owner**: WP02 (H1), then WP05 and the mission review

WP01 built `VoiceLimit`, enforced it in the callback, and proved the enforcement
falsifiable. It did **not** make the shipped product correct: `Patch::new` seeds
`VoicePolicy::EngineManaged` = 64 unconditionally, so in the running app every
Patch carries 64 — including the Braids Patches whose engine declares 16.

The crest-spec's `valueObject.Synth.VoiceLimit` invariant says installation seeds
from the active engine's ceiling "so no Patch starts with a limit its engine
could not honour". That is **unsatisfied** until WP02's H1 lands, because
`install_patches` is the only site with registry access and it belongs to WP02.

WP01's boundary was honest — the reviewer verified independently that neither
alternative was available inside WP01's ownership map. But FR-009 is not complete
on WP01, and neither WP05 nor the mission review should credit it there.

## F-02 — Six public APIs with no production caller

**Raised by**: WP01's review
**Owner**: WP02, WP03

`Patch::installed`, `seed_voice_limit`, `replace_instrument_config`,
`VoiceLimitCarryOver`, `with_voice_limit`, and `VoiceLimitDescriptor` /
`surface_descriptor` currently have zero production callers — only tests. They
exist for WP02 and WP03 to consume. If those packages do not consume them, they
are dead code at mission review, and the honest remedy then is deletion, not a
retroactive justification.

## F-03 — The leaf-descriptor enumeration gap was mine, and it is closed

**Raised by**: WP01's review
**Owner**: closed at crest-spec authoring; WP02 implements

The crest-spec widened `RtPatchParameters` to carry `VoiceLimit` without amending
the leaf-descriptor invariant's enumeration. WP01's prompt then demanded
enumeration the declaration did not require. The reviewer caught the disagreement
and ruled correctly that the declaration wins over a prompt.

Resolved deliberately rather than by letting the code decide: the enumeration in
`.kittify/crest-spec/contexts/realtime.yaml` now names `voiceLimit`, because a
canonical value that crosses the real-time boundary and changes what is audible
must be visible in the trace or no measured proof can correlate it. WP06's live
scene has to correlate exactly that.

## F-04 — One overclaiming test in `parameter_snapshot.rs`

**Raised by**: WP01's review
**Owner**: WP02 (it opens that file for H3)

`the_voice_limit_widens_the_entry_by_one_bounded_integer` asserts two tautologies
while its docstring claims the assertion is exact. The bullets it claims are
covered by two stronger pre-existing tests, so nothing is uncovered — but a false
rigor claim in a docstring is worse than no test. WP02 makes it exact or deletes
it.

## F-05 — The mission baseline's "1 pre-existing test failure" is malformed

**Raised by**: WP01, confirmed by its review

The baseline capture records one pre-existing failure. It is not a test: the
entry is `<declared-command>` with "For more information, try '--help'" — a CLI
usage error, the same capture defect a previous mission recorded as F-12. The
lane measured 734 passed / 0 failed at base. No WP should carry blame for it and
no WP should chase it.

## F-06 — Re-trigger at the limit is refused

**Raised by**: WP01's review
**Owner**: unowned; a deliberate ruling if re-trigger behaviour ever matters

The active-note bitset is per (patch, note), so re-triggering an already-sounding
note while at the limit is refused even though it would add no voice. This is
consistent with the declaration as written. Recorded rather than fixed, because
changing it is a product decision about what "sounding its limit" means.
