#!/usr/bin/env python3
"""Generate SDK client stubs from protocol.json."""
import json
import os
import re
from pathlib import Path
from jinja2 import Environment, FileSystemLoader

HERE = Path(__file__).parent
TEMPLATES = HERE / "templates"
OUTPUTS = {
    "types.ts.j2":   HERE.parent / "sdk-node/src/types.ts",
    "client.ts.j2":  HERE.parent / "sdk-node/src/_client.ts",
    "types.py.j2":   HERE.parent / "sdk-python/runlens_sdk/_types.py",
    "client.py.j2":  HERE.parent / "sdk-python/runlens_sdk/_client.py",
    "types.go.j2":   HERE.parent / "sdk-go/types.go",
    "client.go.j2":  HERE.parent / "sdk-go/client.go",
}


def snake_to_camel(s: str) -> str:
    parts = s.split("_")
    return parts[0] + "".join(p.capitalize() for p in parts[1:])


def snake_case(s: str) -> str:
    return s


def ts_type(t: str) -> str:
    mapping = {"string": "string", "integer": "number", "object": "Record<string, unknown>", "array": "unknown[]"}
    return mapping.get(t, "unknown")


def py_type(t: str) -> str:
    mapping = {"string": "str", "integer": "int", "object": "dict", "array": "list"}
    return mapping.get(t, "Any")


def go_type(t: str) -> str:
    mapping = {"string": "string", "integer": "uint64", "object": "map[string]any", "array": "[]any"}
    return mapping.get(t, "any")


def go_name(s: str) -> str:
    parts = s.split("_")
    return "".join(p.capitalize() for p in parts)


def load_protocol():
    with open(HERE / "protocol.json") as f:
        return json.load(f)


def main():
    proto = load_protocol()
    env = Environment(loader=FileSystemLoader(str(TEMPLATES)))
    env.filters["snake_to_camel"] = snake_to_camel
    env.filters["snake_case"] = snake_case
    env.filters["ts_type"] = ts_type
    env.filters["py_type"] = py_type
    env.filters["go_type"] = go_type
    env.filters["go_name"] = go_name
    fields = proto["event"]["fields"]
    required = [k for k, v in fields.items() if v.get("required")]
    optional = [k for k, v in fields.items() if not v.get("required")]
    enums = {k: v["enum"] for k, v in fields.items() if "enum" in v}
    methods = proto["methods"]

    for tmpl_name, out_path in OUTPUTS.items():
        tmpl = env.get_template(tmpl_name)
        os.makedirs(out_path.parent, exist_ok=True)
        with open(out_path, "w") as f:
            f.write(tmpl.render(
                fields=fields,
                required=required,
                optional=optional,
                enums=enums,
                methods=methods,
                daemon=proto["daemon"],
            ))
        print(f"  OK {out_path.relative_to(HERE.parent.parent)}")

    print("Done -- all stubs generated")


if __name__ == "__main__":
    main()
