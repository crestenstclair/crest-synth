//! No-name-enumeration guard — the open-closed property as a build gate.
//!
//! Invariant (`.kittify/crest-spec/proof/invariants.yaml`, core group;
//! executable check `validation.no_name_enumerated_identity`):
//! no type in Synth, Mixer, RealTime, or Control may enumerate a variant,
//! field, or descriptor entry named after a specific effect or bus. Effects,
//! slots, sends, and returns are addressed by index into descriptor-driven
//! arrays, so adding a registry entry must require no change to any of these
//! types. The single declared exception is `MasterGainDb`: master gain is
//! genuinely global — a property of the master stage, not of any effect.
//!
//! Why this is a mechanical gate rather than prose: the closed enumerated
//! design (`MixerTrackParameter::ReverbSend`, `GlobalEffectsProcessor`,
//! `GlobalParameter::ReverbRoomSize`, ...) shipped even though `DESIGN.md`
//! declared the three-slot / eight-return expansion in advance. A design
//! document is a demonstrably insufficient control here, so a failed check —
//! not a reviewer's attention — is what stops the next closed shortcut.
//!
//! The scan itself lives in `scripts/check_no_name_enumerated_identity.sh`
//! (the command declared by the crest-spec validation). It matches the
//! retired identifiers exactly, in identifier position only, over the four
//! context trees. Deliberately excluded, with reasons:
//! - `src/adapter/*`: an adapter implementing reverb is supposed to say
//!   "reverb"; the invariant binds the four domain contexts, not the
//!   adapters that register concrete entries.
//! - comments and string literals: doc comments narrate the retired design
//!   as history; registry capability ids (`"effect.reverb"`) are the open
//!   registry working as designed; retained observation/telemetry labels are
//!   `logs_telemetry: do_not_change` in `occurrence_map.yaml`.
//! - composite identifiers (`reverb_input_rms`, the transitional telemetry
//!   accessors over generic `bus_input_rms` arrays;
//!   `InvalidMaxDelayMilliseconds`): word-boundary matching keeps them out.
//!
//! If this test fails on your change, do not weaken the script or this test:
//! address the effect or bus by index/registry entry instead, or — if you
//! believe the identifier is genuinely legitimate — amend the invariant in
//! the crest-spec first.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const PASS_MARKER: &str = "CREST_STATIC_VALIDATION no_name_enumerated_identity passed";
const SELF_TEST_MARKER: &str =
    "CREST_STATIC_VALIDATION no_name_enumerated_identity self-test passed";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn run_guard(args: &[&str]) -> Output {
    Command::new("bash")
        .arg(repo_root().join("scripts/check_no_name_enumerated_identity.sh"))
        .args(args)
        .current_dir(repo_root())
        .output()
        .expect("the declared guard script must be runnable")
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn fixture_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("stale fixture directory must be removable");
    }
    fs::create_dir_all(&dir).expect("fixture directory must be creatable");
    dir
}

fn write_fixture(dir: &Path, file: &str, content: &str) {
    fs::write(dir.join(file), content).expect("fixture file must be writable");
}

/// The delivered tree satisfies the invariant: the declared validation
/// command passes and emits the marker `spec-kitty accept` asserts on.
#[test]
fn guard_passes_on_the_delivered_tree() {
    let output = run_guard(&[]);
    let stdout = stdout_of(&output);
    assert!(
        output.status.success(),
        "guard failed on the delivered tree — an earlier change reintroduced \
         a name-enumerated identity:\n{stdout}"
    );
    assert!(
        stdout.contains(PASS_MARKER),
        "guard must emit the declared pass marker, got:\n{stdout}"
    );
}

/// Negative proof: a reintroduced retired variant is detected, and the
/// report names file, line, and identifier so the failure is actionable.
#[test]
fn guard_fails_on_a_reintroduced_variant_with_file_line_and_identifier() {
    let dir = fixture_dir("name_enum_guard_reintroduced_variant");
    write_fixture(
        &dir,
        "mixer_track_parameters.rs",
        "pub enum MixerTrackParameter {\n    Level,\n    ReverbSend,\n}\n",
    );
    let output = run_guard(&["--scan-root", dir.to_str().unwrap()]);
    let stdout = stdout_of(&output);
    assert!(
        !output.status.success(),
        "guard accepted a reintroduced ReverbSend variant:\n{stdout}"
    );
    assert!(
        stdout.contains("mixer_track_parameters.rs:3: name-enumerated identifier ReverbSend"),
        "failure report must name file, line, and identifier, got:\n{stdout}"
    );
}

/// Negative proof across all four retired rename families from
/// `plan.md` § Bulk Edit Classification.
#[test]
fn guard_fails_on_every_retired_identifier_family() {
    let dir = fixture_dir("name_enum_guard_all_families");
    write_fixture(
        &dir,
        "reintroduced.rs",
        concat!(
            "pub struct GlobalEffectsProcessor;\n",
            "pub struct GlobalReverbDelay;\n",
            "pub enum GlobalParameter {\n",
            "    DelayFeedback,\n",
            "}\n",
            "pub fn process(reverb_input: &[f32], delay_send: f32) {}\n",
        ),
    );
    let output = run_guard(&["--scan-root", dir.to_str().unwrap()]);
    let stdout = stdout_of(&output);
    assert!(
        !output.status.success(),
        "guard accepted retired identifiers:\n{stdout}"
    );
    for expected in [
        "reintroduced.rs:1: name-enumerated identifier GlobalEffectsProcessor",
        "reintroduced.rs:2: name-enumerated identifier GlobalReverbDelay",
        "reintroduced.rs:4: name-enumerated identifier DelayFeedback",
        "reintroduced.rs:6: name-enumerated identifier delay_send",
        "reintroduced.rs:6: name-enumerated identifier reverb_input",
    ] {
        assert!(
            stdout.contains(expected),
            "expected report line '{expected}' missing from:\n{stdout}"
        );
    }
}

/// False-positive boundary: `MasterGainDb` (the single declared exception),
/// comment narration, string-literal registry ids and telemetry labels, and
/// composite identifiers all pass cleanly.
#[test]
fn guard_allows_master_gain_and_the_documented_exempt_surfaces() {
    let dir = fixture_dir("name_enum_guard_exempt_surfaces");
    write_fixture(
        &dir,
        "exceptions.rs",
        concat!(
            "/// The retired `ReverbSend` and `GlobalEffectsProcessor` are\n",
            "/// narrated here as history, which is not identity.\n",
            "pub enum GlobalParameter {\n",
            "    MasterGainDb,\n",
            "}\n",
            "pub struct GlobalParameters {\n",
            "    master_gain_db: f32,\n",
            "}\n",
            "pub enum EffectError {\n",
            "    InvalidMaxDelayMilliseconds,\n",
            "}\n",
            "pub const fn reverb_input_rms() -> f32 {\n",
            "    0.0\n",
            "}\n",
            "pub const CAPABILITY: &str = \"effect.reverb\";\n",
            "pub const LABEL: &str = \"PATCH Lead\\n> reverbSend=0.4\\nGLOBAL\";\n",
        ),
    );
    let output = run_guard(&["--scan-root", dir.to_str().unwrap()]);
    let stdout = stdout_of(&output);
    assert!(
        output.status.success(),
        "guard flagged a documented exception:\n{stdout}"
    );
    assert!(stdout.contains(PASS_MARKER));
}

/// The script's own `--self-test` (used at accept time without cargo)
/// observes the failure path and the exception path.
#[test]
fn guard_self_test_observes_its_own_failure_path() {
    let output = run_guard(&["--self-test"]);
    let stdout = stdout_of(&output);
    assert!(
        output.status.success(),
        "guard self-test failed:\n{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains(SELF_TEST_MARKER),
        "self-test must emit its marker, got:\n{stdout}"
    );
}
