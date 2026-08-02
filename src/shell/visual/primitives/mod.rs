//! The reusable shell primitives.
//!
//! **Components paint. Views compose. The reducer decides.**
//!
//! A primitive here accepts immutable view data plus one explicit
//! [`ComponentState`](crate::shell::visual::state::ComponentState) and paints.
//! It does not decide. It never reads focus, looks up a Patch value, asks what
//! context is active, consults the reducer, or touches audio state — all of
//! that is passed in by the view that composes it.
//!
//! The boundary exists because the alternative erodes silently: one primitive
//! that reaches for focus "just this once" becomes the pattern, and the
//! one-way loop physical input → semantic action → `AppState::apply` →
//! projection stops being true of the render path. It is
//! `requirement.component_state_ownership_boundary`, and the tests at the
//! bottom of this file enforce it structurally rather than by convention.
//! What they check, exactly:
//!
//! - no `use` line in this module imports outside the visual vocabulary, egui,
//!   and std;
//! - no source in this module names a non-visual crate root anywhere in its
//!   text — not in a `use` line, not in an inline fully-qualified path, not in
//!   a turbofish or a type position;
//! - no source walks out of the vocabulary by a relative `super::` chain;
//! - no `match` in this module falls through a wildcard arm.
//!
//! The scans are textual, so what they prove is that these paths are not
//! *named* here. A primitive could still receive application state through an
//! argument whose type is re-exported from the vocabulary; closing that is
//! WP06's literal-absence proof, not this module's claim.
//!
//! The seven families:
//!
//! - [`text`] — the authored text roles every other primitive composes from
//! - [`rules`] — hairlines and structural keylines
//! - [`focus`] — the focus frame, the authored halo, and the cursor column
//! - [`value`] — the right-aligned value column
//! - [`status`] — the marks that make state legible without color
//! - [`hint`] — the footer's action hints

pub mod focus;
pub mod hint;
pub mod rules;
pub mod status;
pub mod text;
pub mod value;

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    /// The prefixes a primitive may import from: the visual vocabulary it
    /// paints through, the rendering stack it paints with, and the standard
    /// library. Anything else is application state reaching into a component.
    const ALLOWED_IMPORT_PREFIXES: [&str; 5] = [
        "use crate::shell::visual::",
        "use super::",
        "use eframe::",
        "use egui::",
        "use std::",
    ];

    /// The crate roots a primitive may never name, from `src/lib.rs`. `shell`
    /// is deliberately absent: the visual vocabulary lives under it, and the
    /// import check constrains which part of it a primitive may reach.
    const NON_VISUAL_CRATE_ROOTS: [&str; 7] = [
        "adapter",
        "control",
        "kernel",
        "mixer",
        "real_time",
        "synth",
        "testing",
    ];

    /// Every spelling of a path into a non-visual root, assembled at runtime so
    /// this file does not match itself: the `crate::` form a module inside the
    /// crate writes, and the `crest_synth::` form that names the crate through
    /// its own package.
    ///
    /// Only the first is reachable from here today — the package name does not
    /// resolve inside its own lib without an `extern crate self as` alias, so
    /// that spelling is carried against a future move of these sources rather
    /// than against a mutation that compiles now.
    fn forbidden_paths() -> Vec<String> {
        let roots = ["crate", "crest_synth"];
        roots
            .iter()
            .flat_map(|root| {
                NON_VISUAL_CRATE_ROOTS
                    .iter()
                    .map(move |leaf| format!("{root}{}{leaf}", "::"))
            })
            .collect()
    }

    /// The primitives module's own sources.
    fn primitive_sources() -> Vec<PathBuf> {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/shell/visual/primitives");
        let mut sources: Vec<PathBuf> = std::fs::read_dir(&dir)
            .expect("the primitives module directory")
            .map(|entry| entry.expect("directory entry").path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
            .collect();
        sources.sort();
        sources
    }

    /// A scan that reads nothing passes vacuously, so the file set is asserted
    /// before anything is asserted about its contents.
    #[test]
    fn the_scan_covers_every_primitive_source() {
        let names: Vec<String> = primitive_sources()
            .iter()
            .map(|path| {
                path.file_name()
                    .expect("file name")
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        assert_eq!(
            names,
            vec![
                "focus.rs",
                "hint.rs",
                "mod.rs",
                "rules.rs",
                "status.rs",
                "text.rs",
                "value.rs",
            ],
            "the primitives module gained or lost a source without updating this scan"
        );
    }

    /// The ownership boundary as it shows up in import statements.
    ///
    /// This check sees `use` lines and nothing else, which is why it does not
    /// stand alone: an inline fully-qualified path reaches application state
    /// without ever writing one.
    /// [`no_primitive_names_a_path_outside_the_visual_vocabulary`] closes that.
    /// The two are kept apart because they catch different things — this one
    /// rejects an import of *anything* unlisted, including a third-party crate
    /// the module has no business pulling in, which a fixed forbidden-path list
    /// would never name.
    #[test]
    fn no_primitive_imports_anything_outside_the_vocabulary_egui_and_std() {
        for path in primitive_sources() {
            let source = std::fs::read_to_string(&path).expect("primitive source");
            for line in source.lines() {
                let trimmed = line.trim();
                if !trimmed.starts_with("use ") {
                    continue;
                }
                assert!(
                    ALLOWED_IMPORT_PREFIXES
                        .iter()
                        .any(|prefix| trimmed.starts_with(prefix)),
                    "{} imports outside the boundary: {trimmed}",
                    path.display()
                );
            }
        }
    }

    /// Every import line is reachable by the check above. Without this, a
    /// refactor that reformatted imports onto continuation lines would quietly
    /// empty the scan.
    #[test]
    fn every_source_that_imports_at_all_is_seen_by_the_import_check() {
        let mut seen = 0_usize;
        for path in primitive_sources() {
            let source = std::fs::read_to_string(&path).expect("primitive source");
            seen += source
                .lines()
                .filter(|line| line.trim().starts_with("use "))
                .count();
        }
        assert!(
            seen >= 12,
            "the import scan found only {seen} import lines across the module"
        );
    }

    /// The ownership boundary, checked against the whole source text.
    ///
    /// A path does not need an import to be reached. A function body that says
    /// `size_of::<crate` + `::control::AppState>()` compiles, writes no `use`
    /// line, and has application state inside a primitive. (Spelled in two
    /// pieces here for the same reason the needles are assembled at runtime:
    /// this file is one of the sources the scan reads.)
    ///
    /// So the scan is the whole file, not its import lines: no primitive may
    /// name `adapter`, `control`, `kernel`, `mixer`, `real_time`, `synth`, or
    /// `testing` through either the `crate::` or the `crest_synth::` root,
    /// anywhere, in any position.
    #[test]
    fn no_primitive_names_a_path_outside_the_visual_vocabulary() {
        let forbidden = forbidden_paths();
        assert_eq!(
            forbidden.len(),
            NON_VISUAL_CRATE_ROOTS.len() * 2,
            "the forbidden-path list lost a spelling"
        );
        for path in primitive_sources() {
            let source = std::fs::read_to_string(&path).expect("primitive source");
            for needle in &forbidden {
                assert!(
                    !source.contains(needle.as_str()),
                    "{} names {needle}, which is application state reaching into a component",
                    path.display()
                );
            }
        }
    }

    /// The same boundary, closed against the relative spelling.
    ///
    /// Three or more `super::` steps walk out of `primitives` and up past
    /// `visual`, where the roots above are reachable without the word `crate`
    /// appearing anywhere. Two steps is all a primitive ever needs — out of a
    /// `#[cfg(test)]` module and into a sibling — and the vocabulary is named
    /// absolutely everywhere else, so anything deeper is an escape.
    #[test]
    fn no_primitive_walks_out_of_the_vocabulary_by_a_relative_path() {
        let escape = format!("{}{}", "super::super::super", "::");
        for path in primitive_sources() {
            let source = std::fs::read_to_string(&path).expect("primitive source");
            assert!(
                !source.contains(&escape),
                "{} climbs out of the visual vocabulary by a relative path",
                path.display()
            );
        }
    }

    /// No `match` in the module falls through a wildcard.
    ///
    /// The closed state vocabulary only closes anything if every rendering site
    /// is forced to name a new variant. One wildcard arm turns that compile
    /// error into a silent visual default.
    ///
    /// The needle is assembled at runtime so this test does not find itself.
    #[test]
    fn no_match_arm_falls_through_a_wildcard() {
        let wildcard_arm = format!("{}{}", "_ =", ">");
        let binding_arm = format!("{}{}", ".. =", ">");
        for path in primitive_sources() {
            let source = std::fs::read_to_string(&path).expect("primitive source");
            assert!(
                !source.contains(&wildcard_arm),
                "{} has a wildcard match arm",
                path.display()
            );
            assert!(
                !source.contains(&binding_arm),
                "{} has a catch-all match arm",
                path.display()
            );
        }
    }
}
