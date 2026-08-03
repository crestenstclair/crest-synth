# Cross-WP findings — Phase 4b

Findings raised during implementation and review that **no single work package can close**, recorded here so they reach the work package that owns them rather than being rediscovered.

Each entry names its owner. An entry with no owner is for mission review.

---

## F-01 — Three controls have no authored Figma specimen

**Raised by**: WP02 (choice-row adjacency, toggle), WP03 (meter)
**Confirmed by**: WP02's review, via an independent node-type census of the entire Screens page (`6:3`)
**Owner**: design authority — not a work package

The design file defines exactly five component sets: Context Switch (`30:14`), CLI Hint (`30:31`), CLI Browser Line (`32:44`), Compact Parameter Slider (`33:69`), Compact Mixer Fader (`34:45`).

The census found 202 frames, 188 text nodes, 104 rounded rectangles, 39 instances, 6 vectors — **zero ellipses, zero polygons, zero boolean shapes**, and the six vectors are the ADSR curve. Every binary in the file is a text run.

What is therefore absent:

| Missing specimen | What was shipped instead | Flagged in source |
|---|---|---|
| Toggle (no toggle, switch, checkbox, or stepper set exists) | Authored `ON`/`OFF` words per `DESIGN.md:468`, plus one filled/hollow shape channel | Yes |
| Choice-row adjacency affordance (the Compact Parameter Slider's twelve variants add no directional mark in any state; the only two directional glyphs in the file are a `→` row-type marker in the browser Meta column and arrowheads on an interaction-map diagram) | Row rendered without direction marks | Yes |
| Meter (the Mixer screen holds sixteen Fader instances and no level readout, segment ladder, or peak mark) | Read-only twin of the mixer column — same geometry, no cap, since the cap is the grab affordance | Yes |

None of the three was approximated silently. **The decision this leaves open is whether Phase 4 ships them as flagged minimums or the design file is extended first.** That is a product call, not an implementation one.

Also confirmed by the census: **the file contains no Steam Deck or compact-viewport screens at all.** This independently validates every control asking `ViewportDensityPolicy` for compact geometry rather than reading it off a single-viewport frame — there is nothing to read.

---

## F-02 — The mixer mute/solo readout is claimed by two work packages

**Raised by**: WP02's review
**Owner**: WP05, before the mixer strip composes

`control_for` routes `(Toggle, VerticalStrip)` to WP02's `Toggle`. But the authored `State` run that renders mute and solo in the mixer column is a child of the **Compact Mixer Fader** component (`34:45`) — WP03's specimen, not WP02's.

The consequence is a spelling divergence already in the tree:

- WP02's toggle paints `ON`/`OFF` there.
- The design file authors `M -- · S --` at rest, `M ON · S --` when muted, `M -- · S ON` when soloed.

WP02 is not at fault — its own specimen genuinely does not exist (F-01) and it said so. But **the seam needs one owner before the mixer strip composes**, or the mixer column will read differently from the design file.

Related, and settled: `UNAVAILABLE_MARK` (`--`) is **authored**, not merely permitted. The Compact Mixer Fader's `State` run uses `--` in exactly the "this fact is not present" role, which is what C-003's mark-unavailable rule asks for.

---

## F-03 — Two Phase 4a primitives diverge from the design file

**Raised by**: WP02's review
**Owner**: mission review — outside every Phase 4b work package's owned files

The Compact Parameter Slider's variants render:

- **Focused** as `>` plus a cyan underline, and **Editing** as `*`. The shipped `focus::cursor` primitive paints `>` for both Focused and Adjusting.
- **Disabled** as dimmed with no word. WP01's state vocabulary gives Disabled `NonColorSignal::Word("Locked")`.

These live in `src/shell/visual/primitives/focus.rs` and `src/shell/visual/state.rs` — Phase 4a files that no Phase 4b work package owns. WP02 composed the primitives rather than redrawing them, which is what it was instructed to do. **Do not send these back to a control work package.**

---

## F-04 — The fader specimen paints mute and solo simultaneously

**Raised by**: WP03
**Owner**: a later mission — C-002 forbids closing it here

The Compact Mixer Fader specimen shows `M ON · S --`: mute and solo are independently valued. Single-valued `ComponentState` cannot express two simultaneous states. WP03 raised it rather than adding a state, which C-002 forbids.

---

## F-05 — `SemanticControlViewModel` cannot support T009's boundary criterion

**Raised by**: WP02
**Owner**: a later mission — C-002 forbids extending the projection here

T009 asks that at the first or last choice, the adjacency affordance shows unavailable. The view model carries **no choice set, no neighbour, and no boundary flag**, so the criterion is unimplementable from current view data regardless of visual treatment.

This is the mission's own declared edge case working as intended: *"A control needs a value the view model does not carry. The control declares the gap; the view model is not extended in this mission, and no control invents state to fill it."*

---

## F-06 — Compact-viewport label overflow

**Raised by**: WP03
**Owner**: WP06, and **asserted by WP08 T044**

The projected label `"T00 Level"` overflows a compact-viewport mixer column. It is currently clipped at the column edge rather than bleeding into its neighbour, because everything paints into a painter clipped to the column. Figma's column shows only `T00`.

WP03 assigns the fix to the composition layer. **WP08's T044 asserts no clipped or overlapping text at either viewport**, so this must actually be resolved by then — a note is not enough.

---

## F-07 — Value formatting is duplicated until the adapter is reduced

**Raised by**: WP02
**Owner**: WP06

WP02 reproduced the adapter's private `semantic_value_label` exactly (3 dp, `ON`/`OFF`, locator) rather than inventing a second format. The duplicate retires when `src/adapter/eframe_graphical_window.rs:816` is converted during the adapter reduction.

---

## F-08 — `NON_TRACK_STATES` is hand-written where its sibling derives

**Raised by**: WP01's review
**Owner**: whoever next adds a `ComponentState`

`controls/mod.rs:139` `MIXER_STRIP_STATES` derives from `ALL_COMPONENT_STATES`. `:153` `NON_TRACK_STATES` is a hand-written `[ComponentState; 7]`.

A tenth state would grow the mixer set automatically but silently omit itself from the non-track set, and no current test catches it because `Fader` and `Meter` accept everything. Out of scope for Phase 4b; worth deriving when a state is next added.

---

## Settled — do not relitigate

- **Role shape ranges are not binding.** `research.md` R-01's "Shapes it can select" column is a characteristic-shape summary, not a constraint; the crest-spec states the selection invariant (`shell.yaml:195-198`) with no per-role shape range. WP01's T004 table is authoritative: `Asset→BrowserRow` and `Identity`/`Surface→ParameterRow` are askable in `PanelEntry`. Confirmed by WP01's review against three independent lines of evidence.
- **Three pairs are genuinely un-askable and pinned as data**: `Choice`, `Asset`, and `Surface` in `VerticalStrip`.
- **`(Toggle, ModalEntry)` resolves to `ModalOption`**, so the toggle is reachable in three roles, not four. The WP prompt text saying otherwise is wrong; the pinned table is right.
- **The baseline's "1 pre-existing test failure" is not a Rust test.** It is a malformed CLI invocation in the capture harness (`"<declared-command>"` / `"For more information, try '--help'."`). The Rust suite was green before Phase 4b and every failure since is owned by the work package that caused it.

---

## W-01 — Waiver: `--skip-review-artifact-check` used to approve WP03 cycle 2

**Date**: 2026-08-03
**Gate waived**: the review-artifact governance check on `move-task --to approved`
**Waived by**: the WP03 cycle-2 reviewer, recorded in its approval note
**Class**: process/tooling gate — the charter permits self-service waiver here provided it is committed in-repo with rationale and flagged in the next human-visible report. It is **not** a product or proof gate (acceptance validation, live-demo gate, real-time contract proof, or `crest-spec doctor`), none of which may be waived autonomously.

### What happened

`spec-kitty` refused the approval with `WP03 has a rejected review artifact (review-cycle-2.md)`.

### Why the refusal was wrong

`review-cycle-2.md` is not a second rejection. It is the **implementer's acknowledgement copy of the cycle-1 rejection**, written when it set `review_status: "acknowledged"`. Its YAML frontmatter carries `cycle_number: 2` and a stale `verdict: rejected` inherited from the file it copied.

Verified independently by the orchestrator: stripping the frontmatter, the cycle-2 body is **byte-identical to `review-cycle-1.md`** (14,093 bytes each). Both bodies open `# WP03 review — cycle 1`. No cycle-2 rejection was ever issued — cycle 2 was reviewed and approved.

### The underlying tooling defect

**Review-artifact numbering runs one ahead of the review cycle in this mission.** The acknowledgement of cycle-N feedback is written as `review-cycle-(N+1).md` and inherits `verdict: rejected`, so the next approval attempt is blocked by the WP's own acknowledged feedback. The review prompt compounds it by directing a genuine new rejection to `review-cycle-3.md`.

Expect this to recur on every work package that goes through a rejection cycle. WP04 is in cycle 2 now and will hit it.

### Consequence to watch

The board still reports `WP03 ⚠ review artifact: verdict=rejected`. If the merge gate reads the same signal it will refuse — and at that point the override would be applied to a **merge** gate, a different and more serious class than the per-WP approval gate. Resolve the artifact frontmatter before merge rather than waiving again there.
