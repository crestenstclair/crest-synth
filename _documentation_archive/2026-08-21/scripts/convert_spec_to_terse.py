#!/usr/bin/env python3
"""Convert the canonical flat spec-yaml tree into the terse authoring format:

- single-level maps (name -> body) instead of `- id:` lists; max depth 2
- suffix references: shortest dot-suffix that resolves uniquely, with
  field-typed scoping (a `capabilities:` list resolves against capability IDs)
- contributesTo as a map {capability-ref: contribution}
- defaults block for repeated validation workingDirectory/limits

Expands back to canonical and deep-compares to prove losslessness.
"""
import sys
from pathlib import Path

import yaml

CANON = Path.home() / "workspace" / "crest-synth" / "spec-yaml"
OUT = Path.home() / "workspace" / "crest-synth" / "spec-terse"

KIND_SINGULAR = {
    "valueObjects": "valueObject",
    "aggregates": "aggregate",
    "ports": "port",
    "domainServices": "domainService",
    "applicationServices": "applicationService",
}

# fields whose values are references of a known kind
TYPED_FIELDS = {
    "goals": "goal",
    "requiredGoals": "goal",
    "goal": "goal",
    "capabilities": "capability",
    "capability": "capability",
    "requirements": "requirement",
    "actors": "actor",
    "actor": "actor",
    "projectChecks": "validation",
}
# reference fields resolved against ALL declared ids
# (`implements` is polymorphic here: adapters implement ports, test assets implement validations)
GENERIC_FIELDS = {"implements", "resources", "repairResources", "targets", "uses", "consumes", "publishes", "attachedTo"}


class Dumper(yaml.SafeDumper):
    pass


Dumper.add_representer(
    str,
    lambda d, v: d.represent_scalar(
        "tag:yaml.org,2002:str", v, style="|" if "\n" in v else None
    ),
)


def dump(path, doc):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        yaml.dump(doc, Dumper=Dumper, sort_keys=False, allow_unicode=True, width=100),
        encoding="utf-8",
    )


def load_canonical():
    return {
        str(p.relative_to(CANON)): yaml.safe_load(p.read_text(encoding="utf-8"))
        for p in sorted(CANON.rglob("*.yaml"))
    }


def build_index(files):
    """canonical id -> kind, plus per-kind id sets."""
    ids: set[str] = set()
    for rel, doc in files.items():
        for section, items in doc.items():
            if isinstance(items, list):
                for it in items:
                    if isinstance(it, dict) and "id" in it:
                        ids.add(it["id"])
                        for sc in it.get("acceptance", []) if section == "capabilities" else []:
                            ids.add(sc["id"])
    by_kind: dict[str, set[str]] = {}
    for i in ids:
        by_kind.setdefault(i.split(".", 1)[0], set()).add(i)
    return ids, by_kind


STATS = {"shortened": 0, "kept_full": 0, "nonref": 0, "unions": 0}


def shorten(canonical, scope):
    if canonical not in scope:
        # not a declared ID (type expression / prose) — leave verbatim
        STATS["nonref"] += 1
        return canonical
    parts = canonical.split(".")
    for k in range(1, len(parts) + 1):
        suffix = ".".join(parts[-k:])
        matches = [i for i in scope if i == suffix or i.endswith("." + suffix)]
        if matches == [canonical]:
            if k < len(parts):
                STATS["shortened"] += 1
            else:
                STATS["kept_full"] += 1
            return suffix
    raise SystemExit(f"cannot shorten {canonical}")


def expand(ref, scope, where):
    matches = [i for i in scope if i == ref or i.endswith("." + ref)]
    if len(matches) > 1:
        raise SystemExit(f"ambiguous ref {ref!r} at {where}: {matches}")
    return matches[0] if matches else ref


def split_union(expr):
    """Split a type expression at top-level ' | ' only; None if not a clean union."""
    parts, depth, cur = [], 0, ""
    i = 0
    while i < len(expr):
        ch = expr[i]
        if ch in "({[<":
            depth += 1
        elif ch in ")}]>":
            depth -= 1
        if depth == 0 and expr.startswith(" | ", i):
            parts.append(cur)
            cur = ""
            i += 3
            continue
        cur += ch
        i += 1
    parts.append(cur)
    if len(parts) < 2 or any(not p or p != p.strip() for p in parts):
        return None
    if " | ".join(parts) != expr:
        return None
    return parts


def maybe_union(val, forward):
    if forward and isinstance(val, str):
        parts = split_union(val)
        if parts:
            STATS["unions"] += 1
            return parts
        return val
    if not forward and isinstance(val, list) and all(isinstance(p, str) for p in val):
        return " | ".join(val)
    return val


def walk(node, all_ids, by_kind, forward=True):
    """Recursively rewrite reference fields (shorten or expand)."""
    if not isinstance(node, dict):
        return node
    out = {}
    for key, val in node.items():
        if key == "contributesTo" and forward and isinstance(val, list):
            out[key] = {
                shorten(e["capability"], by_kind["capability"]): e["contribution"]
                for e in val
            }
            if len(out[key]) != len(val):
                raise SystemExit("contributesTo key collision")
        elif key == "contributesTo" and not forward and isinstance(val, dict):
            out[key] = [
                {"capability": expand(r, by_kind["capability"], key), "contribution": c}
                for r, c in val.items()
            ]
        elif key in TYPED_FIELDS or key in GENERIC_FIELDS:
            scope = by_kind.get(TYPED_FIELDS.get(key), all_ids) if key in TYPED_FIELDS else all_ids
            fn = (lambda r: shorten(r, scope)) if forward else (lambda r: expand(r, scope, key))
            out[key] = fn(val) if isinstance(val, str) else [fn(r) for r in val]
        elif key == "validations" and isinstance(val, list) and all(isinstance(v, str) for v in val):
            scope = by_kind["validation"]
            fn = (lambda r: shorten(r, scope)) if forward else (lambda r: expand(r, scope, key))
            out[key] = [fn(r) for r in val]
        elif key == "state" and isinstance(val, dict):
            out[key] = {sk: maybe_union(sv, forward) for sk, sv in val.items()}
        elif key == "from":
            out[key] = maybe_union(val, forward)
        elif isinstance(val, dict):
            out[key] = walk(val, all_ids, by_kind, forward)
        elif isinstance(val, list):
            out[key] = [walk(v, all_ids, by_kind, forward) for v in val]
        else:
            out[key] = val
    return out


def listify(section_list, prefix, all_ids, by_kind, forward=True):
    """list-with-id  <->  single-level map with short names."""
    if forward:
        out = {}
        for item in section_list:
            name = item["id"][len(prefix):]
            out[name] = walk({k: v for k, v in item.items() if k != "id"}, all_ids, by_kind)
        if len(out) != len(section_list):
            raise SystemExit(f"duplicate keys under {prefix}")
        return out
    return [
        {"id": prefix + name, **walk(body, all_ids, by_kind, forward=False)}
        for name, body in section_list.items()
    ]


COMMON_DEFAULTS = {"workingDirectory": ".", "limits": {"timeoutMs": 300000, "stdoutBytes": 8388608, "stderrBytes": 8388608}}


def to_terse(files, all_ids, by_kind):
    terse = {}
    for rel, doc in files.items():
        if rel.startswith("contexts/"):
            ctx = doc["context"]
            ctx_name = ctx["id"].split(".", 1)[1]
            new = {"context": ctx_name}
            new.update({k: v for k, v in ctx.items() if k != "id"})
            for kind, singular in KIND_SINGULAR.items():
                if kind in doc:
                    new[kind] = listify(doc[kind], f"{singular}.{ctx_name}.", all_ids, by_kind)
            terse[rel] = new
        elif rel == "project.yaml":
            new = dict(doc)
            new["nonGoals"] = {n["id"].split(".", 1)[1]: n["statement"] for n in doc["nonGoals"]}
            new["completion"] = walk(doc["completion"], all_ids, by_kind)
            terse[rel] = new
        elif rel == "capabilities.yaml":
            caps = {}
            for cap in doc["capabilities"]:
                name = cap["id"].split(".", 1)[1]
                body = walk({k: v for k, v in cap.items() if k not in ("id", "acceptance")}, all_ids, by_kind)
                if "acceptance" in cap:
                    body["acceptance"] = listify(cap["acceptance"], f"acceptance.{name}.", all_ids, by_kind)
                caps[name] = body
            terse[rel] = {"capabilities": caps}
        elif rel == "proof/validations.yaml":
            proj = {}
            for v in doc["projectValidations"]:
                body = walk({k: x for k, x in v.items() if k != "id"}, all_ids, by_kind)
                for dk, dv in COMMON_DEFAULTS.items():
                    if body.get(dk) == dv:
                        del body[dk]
                proj[v["id"].split(".", 1)[1]] = body
            att = {}
            for v in doc["attachedValidations"]:
                att[v["id"].split(".", 1)[1]] = walk({k: x for k, x in v.items() if k != "id"}, all_ids, by_kind)
            terse[rel] = {"defaults": COMMON_DEFAULTS, "projectValidations": proj, "attachedValidations": att}
        elif rel == "assets.yaml":
            terse[rel] = {
                "assetKinds": listify(doc["assetKinds"], "assetKind.", all_ids, by_kind),
                "assets": listify(doc["assets"], "asset.", all_ids, by_kind),
            }
        elif rel == "proof/invariants.yaml":
            terse[rel] = doc
        else:
            (section,) = doc.keys()
            prefix = {"actors": "actor.", "goals": "goal.", "requirements": "requirement.",
                      "adapters": "adapter.", "witnesses": "witness.", "evidence": "evidence."}[section]
            terse[rel] = {section: listify(doc[section], prefix, all_ids, by_kind)}
    return terse


def to_canonical(terse, all_ids, by_kind):
    canon = {}
    for rel, doc in terse.items():
        if rel.startswith("contexts/"):
            ctx_name = doc["context"]
            ctx = {"id": f"context.{ctx_name}"}
            ctx.update({k: v for k, v in doc.items() if k not in ("context", *KIND_SINGULAR)})
            new = {"context": ctx}
            for kind, singular in KIND_SINGULAR.items():
                if kind in doc:
                    new[kind] = listify(doc[kind], f"{singular}.{ctx_name}.", all_ids, by_kind, forward=False)
            canon[rel] = new
        elif rel == "project.yaml":
            new = dict(doc)
            new["nonGoals"] = [{"id": f"nonGoal.{n}", "statement": s} for n, s in doc["nonGoals"].items()]
            new["completion"] = walk(doc["completion"], all_ids, by_kind, forward=False)
            canon[rel] = new
        elif rel == "capabilities.yaml":
            caps = []
            for name, body in doc["capabilities"].items():
                entry = {"id": f"capability.{name}"}
                entry.update(walk({k: v for k, v in body.items() if k != "acceptance"}, all_ids, by_kind, forward=False))
                if "acceptance" in body:
                    entry["acceptance"] = listify(body["acceptance"], f"acceptance.{name}.", all_ids, by_kind, forward=False)
                caps.append(entry)
            canon[rel] = {"capabilities": caps}
        elif rel == "proof/validations.yaml":
            proj = []
            for name, body in doc["projectValidations"].items():
                entry = {"id": f"validation.{name}"}
                entry.update(walk(body, all_ids, by_kind, forward=False))
                for dk, dv in doc.get("defaults", {}).items():
                    entry.setdefault(dk, dv)
                proj.append(entry)
            att = [
                {"id": f"validation.{name}", **walk(body, all_ids, by_kind, forward=False)}
                for name, body in doc["attachedValidations"].items()
            ]
            canon[rel] = {"projectValidations": proj, "attachedValidations": att}
        elif rel == "assets.yaml":
            canon[rel] = {
                "assetKinds": listify(doc["assetKinds"], "assetKind.", all_ids, by_kind, forward=False),
                "assets": listify(doc["assets"], "asset.", all_ids, by_kind, forward=False),
            }
        elif rel == "proof/invariants.yaml":
            canon[rel] = doc
        else:
            (section,) = doc.keys()
            prefix = {"actors": "actor.", "goals": "goal.", "requirements": "requirement.",
                      "adapters": "adapter.", "witnesses": "witness.", "evidence": "evidence."}[section]
            canon[rel] = {section: listify(doc[section], prefix, all_ids, by_kind, forward=False)}
    return canon


def diff(a, b, path="$"):
    out = []
    if isinstance(a, dict) and isinstance(b, dict):
        for k in a.keys() | b.keys():
            if k not in a or k not in b:
                out.append(f"{path}.{k}: only in {'rebuilt' if k not in a else 'original'}")
            else:
                out.extend(diff(a[k], b[k], f"{path}.{k}"))
    elif isinstance(a, list) and isinstance(b, list):
        if len(a) != len(b):
            out.append(f"{path}: length {len(a)} vs {len(b)}")
        else:
            for i, (x, y) in enumerate(zip(a, b)):
                out.extend(diff(x, y, f"{path}[{i}]"))
    elif a != b:
        out.append(f"{path}: {a!r} != {b!r}")
    return out


def main():
    files = load_canonical()
    all_ids, by_kind = build_index(files)
    terse = to_terse(files, all_ids, by_kind)
    for rel, doc in terse.items():
        dump(OUT / rel, doc)

    reloaded = {rel: yaml.safe_load((OUT / rel).read_text(encoding="utf-8")) for rel in terse}
    rebuilt = to_canonical(reloaded, all_ids, by_kind)
    problems = []
    for rel in files:
        problems += diff(files[rel], rebuilt[rel], rel)
    if problems:
        print(f"ROUND-TRIP FAILED ({len(problems)}):")
        print("\n".join(problems[:30]))
        sys.exit(1)

    c_lines = sum(len((CANON / r).read_text().splitlines()) for r in files)
    t_lines = sum(len((OUT / r).read_text().splitlines()) for r in terse)
    c_bytes = sum(len((CANON / r).read_text()) for r in files)
    t_bytes = sum(len((OUT / r).read_text()) for r in terse)
    print(f"round-trip: LOSSLESS. refs shortened: {STATS['shortened']}, kept full: {STATS['kept_full']}, unions split: {STATS['unions']}")
    print(f"lines: {c_lines} -> {t_lines} ({100 * (c_lines - t_lines) // c_lines}% fewer)")
    print(f"bytes: {c_bytes} -> {t_bytes} ({100 * (c_bytes - t_bytes) // c_bytes}% fewer)")


if __name__ == "__main__":
    main()
