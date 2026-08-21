#!/usr/bin/env python3
"""Convert the evaluated crest-synth CUE `project` JSON into the proposed
flat, ID-referenced, multi-file YAML architecture-spec format.

Also rebuilds the original nested JSON from the YAML output and deep-compares
to prove the conversion is lossless.
"""
import json
import sys
from pathlib import Path

import yaml

SCRATCH = Path(__file__).parent
SRC = SCRATCH / "crest-synth-project.json"
OUT = Path.home() / "workspace" / "crest-synth" / "spec-yaml"

KIND_SINGULAR = {
    "valueObjects": "valueObject",
    "aggregates": "aggregate",
    "ports": "port",
    "domainServices": "domainService",
    "applicationServices": "applicationService",
}
CONTEXT_RESOURCE_KINDS = list(KIND_SINGULAR)


# --- YAML style: readable multi-line prose, stable key order -----------------
class SpecDumper(yaml.SafeDumper):
    pass


def _str_representer(dumper, value):
    if "\n" in value:
        return dumper.represent_scalar("tag:yaml.org,2002:str", value, style="|")
    return dumper.represent_scalar("tag:yaml.org,2002:str", value)


SpecDumper.add_representer(str, _str_representer)


def dump(path: Path, doc: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    text = yaml.dump(
        doc, Dumper=SpecDumper, sort_keys=False, allow_unicode=True, width=100
    )
    path.write_text(text, encoding="utf-8")


def with_id(id_value: str, body: dict, extra_first: dict | None = None) -> dict:
    out = {"id": id_value}
    if extra_first:
        out.update(extra_first)
    for k, v in body.items():
        if k != "id":
            out[k] = v
    return out


# --- forward conversion ------------------------------------------------------
def convert(project: dict) -> dict[str, dict]:
    """Return {relative-file-path: yaml-document}."""
    files: dict[str, dict] = {}
    attached: list[dict] = []

    def hoist(owner_id: str, resource: dict) -> dict:
        rest = {k: v for k, v in resource.items() if k != "validations"}
        for v in resource.get("validations", []):
            attached.append(with_id(v.get("id"), v, {"attachedTo": owner_id}))
        return rest

    files["project.yaml"] = {
        "name": project["name"],
        "mission": project["mission"],
        "nonGoals": [
            {"id": f"nonGoal.{name}", "statement": text}
            for name, text in project["nonGoals"].items()
        ],
        "meta": project["meta"],
        "layers": project["layers"],
        "layerRules": project["layerRules"],
        "contextMap": project["contextMap"],
        "completion": project["completion"],
    }

    files["actors.yaml"] = {
        "actors": [with_id(f"actor.{n}", a) for n, a in project["actors"].items()]
    }
    files["goals.yaml"] = {
        "goals": [with_id(f"goal.{n}", g) for n, g in project["goals"].items()]
    }
    files["requirements.yaml"] = {
        "requirements": [
            with_id(f"requirement.{n}", r) for n, r in project["requirements"].items()
        ]
    }

    caps = []
    for name, cap in project["capabilities"].items():
        entry = with_id(f"capability.{name}", {k: v for k, v in cap.items() if k != "acceptance"})
        if "acceptance" in cap:
            entry["acceptance"] = [
                with_id(f"acceptance.{name}.{scenario}", body)
                for scenario, body in cap["acceptance"].items()
            ]
        caps.append(entry)
    files["capabilities.yaml"] = {"capabilities": caps}

    for ctx_name, ctx in project["contexts"].items():
        doc: dict = {
            "context": with_id(
                f"context.{ctx_name}",
                {k: v for k, v in ctx.items() if k not in CONTEXT_RESOURCE_KINDS},
            )
        }
        for kind in CONTEXT_RESOURCE_KINDS:
            if kind not in ctx:
                continue
            singular = KIND_SINGULAR[kind]
            doc[kind] = [
                with_id(f"{singular}.{ctx_name}.{rn}", hoist(f"{singular}.{ctx_name}.{rn}", r))
                for rn, r in ctx[kind].items()
            ]
        files[f"contexts/{ctx_name.lower()}.yaml"] = doc

    files["adapters.yaml"] = {
        "adapters": [
            with_id(f"adapter.{n}", hoist(f"adapter.{n}", a))
            for n, a in project["adapters"].items()
        ]
    }
    files["assets.yaml"] = {
        "assetKinds": [
            with_id(f"assetKind.{n}", k) for n, k in project["assetKinds"].items()
        ],
        "assets": [
            with_id(f"asset.{n}", hoist(f"asset.{n}", a))
            for n, a in project["assets"].items()
        ],
    }

    project_validations = []
    for name, v in project["validations"].items():
        declared = v.get("id")
        expected = f"validation.{name}"
        if declared != expected:
            raise SystemExit(f"validation key/id mismatch: {name!r} vs {declared!r}")
        project_validations.append(with_id(declared, v))
    files["proof/validations.yaml"] = {
        "projectValidations": project_validations,
        "attachedValidations": attached,
    }
    files["proof/witnesses.yaml"] = {
        "witnesses": [with_id(f"witness.{n}", w) for n, w in project["witnesses"].items()]
    }
    files["proof/evidence.yaml"] = {
        "evidence": [with_id(f"evidence.{n}", e) for n, e in project["evidence"].items()]
    }
    files["proof/invariants.yaml"] = {
        "invariantGroups": [
            {"group": name, "invariants": items}
            for name, items in project["invariants"].items()
        ]
    }
    return files


# --- reverse: rebuild nested JSON from the YAML tree -------------------------
def strip_id(entry: dict) -> dict:
    return {k: v for k, v in entry.items() if k != "id"}


def local_name(id_value: str) -> str:
    return id_value.split(".", 1)[1]


def rebuild(files: dict[str, dict]) -> dict:
    proj_file = files["project.yaml"]
    validations_file = files["proof/validations.yaml"]

    attached_by_owner: dict[str, list[dict]] = {}
    for v in validations_file["attachedValidations"]:
        owner = v["attachedTo"]
        body = {k: val for k, val in v.items() if k != "attachedTo"}
        if body.get("id") is None:
            body.pop("id", None)
        attached_by_owner.setdefault(owner, []).append(body)

    def restore(owner_id: str, body: dict) -> dict:
        out = dict(body)
        if owner_id in attached_by_owner:
            out["validations"] = attached_by_owner[owner_id]
        return out

    project: dict = {
        "mission": proj_file["mission"],
        "actors": {
            local_name(a["id"]): strip_id(a) for a in files["actors.yaml"]["actors"]
        },
        "goals": {local_name(g["id"]): strip_id(g) for g in files["goals.yaml"]["goals"]},
        "capabilities": {},
        "name": proj_file["name"],
        "layers": proj_file["layers"],
        "layerRules": proj_file["layerRules"],
        "meta": proj_file["meta"],
        "requirements": {
            local_name(r["id"]): strip_id(r)
            for r in files["requirements.yaml"]["requirements"]
        },
        "adapters": {
            local_name(a["id"]): restore(a["id"], strip_id(a))
            for a in files["adapters.yaml"]["adapters"]
        },
        "assets": {
            local_name(a["id"]): restore(a["id"], strip_id(a))
            for a in files["assets.yaml"]["assets"]
        },
        "contexts": {},
        "evidence": {
            local_name(e["id"]): strip_id(e)
            for e in files["proof/evidence.yaml"]["evidence"]
        },
        "validations": {
            local_name(v["id"]): v
            for v in files["proof/validations.yaml"]["projectValidations"]
        },
        "witnesses": {
            local_name(w["id"]): strip_id(w)
            for w in files["proof/witnesses.yaml"]["witnesses"]
        },
        "invariants": {
            g["group"]: g["invariants"]
            for g in files["proof/invariants.yaml"]["invariantGroups"]
        },
        "contextMap": proj_file["contextMap"],
        "assetKinds": {
            local_name(k["id"]): strip_id(k) for k in files["assets.yaml"]["assetKinds"]
        },
        "nonGoals": {
            local_name(n["id"]): n["statement"] for n in proj_file["nonGoals"]
        },
        "completion": proj_file["completion"],
    }

    for cap in files["capabilities.yaml"]["capabilities"]:
        body = {k: v for k, v in cap.items() if k not in ("id", "acceptance")}
        if "acceptance" in cap:
            body["acceptance"] = {
                s["id"].rsplit(".", 1)[1]: strip_id(s) for s in cap["acceptance"]
            }
        project["capabilities"][local_name(cap["id"])] = body

    for path, doc in files.items():
        if not path.startswith("contexts/"):
            continue
        ctx_entry = doc["context"]
        ctx_name = local_name(ctx_entry["id"])
        ctx: dict = strip_id(ctx_entry)
        for kind in CONTEXT_RESOURCE_KINDS:
            if kind not in doc:
                continue
            ctx[kind] = {
                r["id"].rsplit(".", 1)[1]: restore(r["id"], strip_id(r))
                for r in doc[kind]
            }
        project["contexts"][ctx_name] = ctx
    return project


# --- deep compare ------------------------------------------------------------
def diff(a, b, path="$"):
    problems = []
    if isinstance(a, dict) and isinstance(b, dict):
        for k in a.keys() | b.keys():
            if k not in a:
                problems.append(f"{path}.{k}: missing in original")
            elif k not in b:
                problems.append(f"{path}.{k}: missing in rebuilt")
            else:
                problems.extend(diff(a[k], b[k], f"{path}.{k}"))
    elif isinstance(a, list) and isinstance(b, list):
        if len(a) != len(b):
            problems.append(f"{path}: list length {len(a)} vs {len(b)}")
        else:
            for i, (x, y) in enumerate(zip(a, b)):
                problems.extend(diff(x, y, f"{path}[{i}]"))
    elif a != b:
        problems.append(f"{path}: {a!r} != {b!r}")
    return problems


def main() -> None:
    project = json.loads(SRC.read_text())
    files = convert(project)

    for rel, doc in files.items():
        dump(OUT / rel, doc)

    reloaded = {
        rel: yaml.safe_load((OUT / rel).read_text(encoding="utf-8")) for rel in files
    }
    rebuilt = rebuild(reloaded)
    problems = diff(project, rebuilt)
    if problems:
        print(f"ROUND-TRIP FAILED ({len(problems)} differences):")
        for p in problems[:40]:
            print(" ", p)
        sys.exit(1)

    total_attached = len(files["proof/validations.yaml"]["attachedValidations"])
    print(f"wrote {len(files)} files to {OUT}")
    print(f"project validations: {len(files['proof/validations.yaml']['projectValidations'])}")
    print(f"attached validations hoisted: {total_attached}")
    print("round-trip: LOSSLESS (deep-equal to evaluated CUE output)")


if __name__ == "__main__":
    main()
