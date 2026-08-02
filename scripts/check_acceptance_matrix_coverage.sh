#!/usr/bin/env bash
#
# Static check: a mission's acceptance record must grade every requirement kind
# its specification declares.
#
# Declared at .kittify/crest-spec/proof/validations.yaml as
# `acceptance_matrix_covers_all_requirement_kinds`. Two consecutive missions
# shipped with an acceptance matrix that graded functional requirements only
# while their specifications also declared non-functional requirements and
# constraints, so a whole class of requirements went silently ungraded. This
# check makes that condition loud and names the omitted kind.
#
# It fails loud in both directions: missing tooling exits non-zero naming the
# tool, so an absent interpreter can never read as a pass, and a mission whose
# record omits a declared kind is named individually rather than folded into a
# count.
#
# Scope: a mission is graded once it has actually run acceptance (it has a
# deterministic-acceptance.json) and has not yet been archived. That is the
# window where grading can still change an outcome, and it is what "acceptance
# record" in the declaration names — a mission that never reached the gate has a
# scaffolded placeholder matrix, not a record, and an archived mission's record
# is closed history the operator has already signed off. Grading either forever
# would make every future archive re-litigate missions that shipped months ago,
# which blocks work without protecting anything.
#
# A mission whose acceptance record is not present on this surface is reported
# as not-yet-graded and does not fail the check. Under coordination topology the
# matrix is a coordination-partition artifact that lives on the coordination
# worktree until the mission consolidates; treating its absence as an omission
# would fail every mission for being mid-flight rather than for ungraded work.

set -euo pipefail

VALIDATION_NAME="acceptance_matrix_covers_all_requirement_kinds"

# --- tool preflight -------------------------------------------------------
# Verified before anything is scanned, so a missing tool is a named non-zero
# exit rather than an empty scan that reads as a pass.
missing_tools=()
for tool in python3 grep; do
  command -v "$tool" >/dev/null 2>&1 || missing_tools+=("$tool")
done
if [ ${#missing_tools[@]} -gt 0 ]; then
  echo "CREST_STATIC_VALIDATION ${VALIDATION_NAME} failed" >&2
  for tool in "${missing_tools[@]}"; do
    echo "  required tool not found: ${tool}" >&2
  done
  exit 2
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
missions_dir="${repo_root}/kitty-specs"

if [ ! -d "${missions_dir}" ]; then
  echo "CREST_STATIC_VALIDATION ${VALIDATION_NAME} failed" >&2
  echo "  mission directory not found: ${missions_dir}" >&2
  exit 2
fi

VALIDATION_NAME="${VALIDATION_NAME}" MISSIONS_DIR="${missions_dir}" python3 <<'PY'
import json
import os
import re
import sys
from pathlib import Path

VALIDATION = os.environ["VALIDATION_NAME"]
MISSIONS = Path(os.environ["MISSIONS_DIR"])
ARCHIVE_LEDGER = MISSIONS.parent / ".kittify" / "archive" / "archived-missions.jsonl"


def archived_mission_ids() -> set[str]:
    """Mission ids the operator has formally archived (closed history)."""
    ids: set[str] = set()
    if not ARCHIVE_LEDGER.is_file():
        return ids
    for line in ARCHIVE_LEDGER.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            record = json.loads(line)
        except json.JSONDecodeError:
            continue
        mission_id = record.get("mission_id")
        if mission_id:
            ids.add(mission_id)
    return ids


def mission_id_of(mission_dir: Path) -> str | None:
    meta = mission_dir / "meta.json"
    if not meta.is_file():
        return None
    try:
        return json.loads(meta.read_text(encoding="utf-8")).get("mission_id")
    except (OSError, json.JSONDecodeError):
        return None

# The kinds a specification can declare that an acceptance record must grade.
# Success criteria (SC-) are graded by some missions and are accepted when
# present, but they are outcome statements rather than a requirement kind, so
# their absence is not an omission.
REQUIRED_KINDS = ("FR", "NFR", "C")
KIND_LABEL = {
    "FR": "functional requirements (FR-)",
    "NFR": "non-functional requirements (NFR-)",
    "C": "constraints (C-)",
}

# A requirements table row: leading pipe, then the identifier in its own cell.
SPEC_ROW = re.compile(r"^\|\s*(FR|NFR|C|SC)-\d+[A-Za-z0-9-]*\s*\|")
# A criterion id, tolerating the -AMEND suffix missions use for amendments.
CRITERION_ID = re.compile(r"^(FR|NFR|C|SC)-\d+")


def declared_kinds(spec_path: Path) -> set[str]:
    kinds = set()
    for line in spec_path.read_text(encoding="utf-8", errors="replace").splitlines():
        match = SPEC_ROW.match(line.strip())
        if match:
            kinds.add(match.group(1))
    return kinds


def graded_kinds(matrix_path: Path) -> tuple[set[str], str | None]:
    try:
        data = json.loads(matrix_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        return set(), f"acceptance record is unreadable: {exc}"
    criteria = data.get("criteria")
    if not isinstance(criteria, list):
        return set(), "acceptance record has no criteria list"
    kinds = set()
    for criterion in criteria:
        raw = (criterion or {}).get("criterion_id") or ""
        match = CRITERION_ID.match(raw.strip())
        if match:
            kinds.add(match.group(1))
    return kinds, None


failures: list[str] = []
ungraded: list[str] = []
out_of_scope: list[str] = []
checked = 0
archived = archived_mission_ids()

for mission_dir in sorted(p for p in MISSIONS.iterdir() if p.is_dir()):
    spec = mission_dir / "spec.md"
    if not spec.is_file():
        continue

    if mission_id_of(mission_dir) in archived:
        out_of_scope.append(f"{mission_dir.name} (archived)")
        continue
    if not (mission_dir / "deterministic-acceptance.json").is_file():
        out_of_scope.append(f"{mission_dir.name} (has not run acceptance)")
        continue

    matrix = mission_dir / "acceptance-matrix.json"
    if not matrix.is_file():
        # No acceptance record on this surface — nothing to grade yet.
        ungraded.append(mission_dir.name)
        continue

    declared = declared_kinds(spec)
    graded, error = graded_kinds(matrix)
    if error is not None:
        failures.append(f"{mission_dir.name}: {error}")
        continue

    checked += 1
    for kind in REQUIRED_KINDS:
        if kind in declared and kind not in graded:
            failures.append(
                f"{mission_dir.name}: specification declares {KIND_LABEL[kind]} "
                f"but the acceptance record grades no {kind}- row"
            )

if failures:
    print(f"CREST_STATIC_VALIDATION {VALIDATION} failed", file=sys.stderr)
    for failure in failures:
        print(f"  {failure}", file=sys.stderr)
    sys.exit(1)

if checked == 0:
    # An empty scan is not a pass. Say so rather than printing the marker.
    print(f"CREST_STATIC_VALIDATION {VALIDATION} failed", file=sys.stderr)
    print("  no mission carried both a specification and an acceptance record", file=sys.stderr)
    sys.exit(1)

for name in out_of_scope:
    print(f"out of scope: {name}")
for name in ungraded:
    print(f"not yet graded (no acceptance record on this surface): {name}")
print(f"graded {checked} mission acceptance record(s) against {', '.join(REQUIRED_KINDS)}")
print(f"CREST_STATIC_VALIDATION {VALIDATION} passed")
PY
